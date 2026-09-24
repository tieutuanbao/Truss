use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::application::PortError;
use crate::domain::{ContentHash, DomainError, RelativePath};

// Shared filesystem primitives for every `.truss-core/` writer.
//
// The core distribution state and the add-on record use one lock, one atomic
// write style, and one state root, so there is a single writer per repository
// rather than one style per distribution.

pub(crate) fn state_root(root: &Path) -> PathBuf {
    root.join(".truss-core")
}

/// Rules the core-owned shared `.truss-core/.gitignore` must carry.
///
/// Core install and core update are the only writers of this file. Add-on
/// operations validate the rules are present and refuse when one is missing;
/// they never patch the file.
pub(crate) const STATE_IGNORE_RULES: [&str; 6] = [
    "/lock",
    "/transaction.json",
    "/base.next-*",
    "/update/",
    "/update-candidate/",
    "/addon-update/",
];

pub(crate) fn ensure_state_ignore(state_root: &Path) -> Result<(), PortError> {
    let path = state_root.join(".gitignore");
    let rules = STATE_IGNORE_RULES;
    if path.exists() {
        reject_symlink(&path, ".truss-core/.gitignore")?;
        let metadata = fs::metadata(&path).map_err(io_error)?;
        if !metadata.is_file() {
            return Err(PortError::new(
                ".truss-core/.gitignore is not a regular file",
            ));
        }
        let mut content = fs::read_to_string(&path).map_err(io_error)?;
        let mut changed = false;
        for rule in rules {
            if !content.lines().any(|line| line.trim() == rule) {
                if !content.is_empty() && !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(rule);
                content.push('\n');
                changed = true;
            }
        }
        if changed {
            copy_bytes_atomic(content.as_bytes(), &path, "ignore")?;
        }
        return Ok(());
    }
    let mut content = String::new();
    for rule in STATE_IGNORE_RULES {
        content.push_str(rule);
        content.push('\n');
    }
    copy_bytes(content.as_bytes(), &path)
}

/// Validate the pre-existing core state that an add-on operation requires.
///
/// Core install and core update exclusively create and repair `.truss-core/`,
/// its `.gitignore`, and its `lock`. This check is read-only on purpose: a
/// missing, unsafe, or incomplete artifact is a refusal, and nothing here ever
/// creates or repairs one. It runs before any payload or workspace
/// observation.
pub(crate) fn validate_core_state(state_root: &Path) -> Result<(), PortError> {
    let metadata = fs::symlink_metadata(state_root).map_err(|error| {
        PortError::new(format!(
            "add-on state operations require an existing core state at .truss-core: {error}"
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(PortError::new(format!(
            "refusing symlink for managed path .truss-core: {}",
            state_root.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(PortError::new(
            "core state .truss-core is not a directory".to_owned(),
        ));
    }
    let ignore = state_root.join(".gitignore");
    require_regular_file(&ignore, ".truss-core/.gitignore")?;
    let content = fs::read_to_string(&ignore).map_err(io_error)?;
    for rule in STATE_IGNORE_RULES {
        if !content.lines().any(|line| line.trim() == rule) {
            return Err(PortError::new(format!(
                "core state .truss-core/.gitignore is missing the rule {rule}"
            )));
        }
    }
    let lock = state_root.join("lock");
    require_regular_file(&lock, ".truss-core/lock")?;
    // Decision 0003 clause 13: the core installation state that owns the
    // foreign paths set must be present. A state without the manifest is
    // invalid, so the foreign set can never be silently empty.
    let manifest = state_root.join("manifest.json");
    require_regular_file(&manifest, ".truss-core/manifest.json")?;
    Ok(())
}

fn require_regular_file(path: &Path, label: &str) -> Result<(), PortError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        PortError::new(format!(
            "{label} is required for add-on state operations: {error}"
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(PortError::new(format!(
            "refusing symlink for managed path {label}: {}",
            path.display()
        )));
    }
    if !metadata.is_file() {
        return Err(PortError::new(format!("{label} is not a regular file")));
    }
    Ok(())
}

pub(crate) fn validate_path(root: &Path, path: &RelativePath) -> Result<(), PortError> {
    let mut current = root.to_path_buf();
    for component in path.as_str().split('/') {
        current.push(component);
        if current.exists() {
            reject_symlink(&current, path.as_str())?;
        }
    }
    Ok(())
}

pub(crate) fn validate_state_path(state_root: &Path, target: &Path) -> Result<(), PortError> {
    let relative = target
        .strip_prefix(state_root)
        .map_err(|_| PortError::new("base path escaped state root"))?;
    let mut current = state_root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        if current.exists() {
            reject_symlink(&current, &target.display().to_string())?;
        }
    }
    Ok(())
}

pub(crate) fn copy_bytes_atomic(content: &[u8], target: &Path, id: &str) -> Result<(), PortError> {
    let parent = target
        .parent()
        .ok_or_else(|| PortError::new(format!("target has no parent: {}", target.display())))?;
    fs::create_dir_all(parent).map_err(io_error)?;
    let name = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| PortError::new(format!("invalid target filename: {}", target.display())))?;
    let temp = parent.join(format!(".{name}.truss-{id}.tmp"));
    copy_bytes(content, &temp)?;
    fs::rename(&temp, target).map_err(io_error)
}

pub(crate) fn copy_bytes(content: &[u8], target: &Path) -> Result<(), PortError> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let mut file = File::create(target).map_err(io_error)?;
    file.write_all(content).map_err(io_error)?;
    file.sync_all().map_err(io_error)
}

pub(crate) fn copy_file(source: &Path, target: &Path) -> Result<(), PortError> {
    let bytes = fs::read(source).map_err(io_error)?;
    copy_bytes(&bytes, target)
}

pub(crate) fn copy_file_atomic(source: &Path, target: &Path, id: &str) -> Result<(), PortError> {
    let bytes = fs::read(source).map_err(io_error)?;
    copy_bytes_atomic(&bytes, target, id)
}

pub(crate) fn copy_tree(source: &Path, target: &Path) -> Result<(), PortError> {
    fs::create_dir_all(target).map_err(io_error)?;
    for entry in fs::read_dir(source).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(PortError::new(format!(
                "refusing symlink in state tree: {}",
                source_path.display()
            )));
        }
        if metadata.is_dir() {
            copy_tree(&source_path, &target_path)?;
        } else if metadata.is_file() {
            copy_file(&source_path, &target_path)?;
        }
    }
    Ok(())
}

pub(crate) fn write_json_atomic<T: Serialize>(
    target: &Path,
    value: &T,
    id: &str,
) -> Result<(), PortError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| PortError::new(format!("could not encode JSON: {error}")))?;
    copy_bytes_atomic(&bytes, target, id)
}

pub(crate) fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, PortError> {
    let bytes = fs::read(path).map_err(io_error)?;
    serde_json::from_slice(&bytes)
        .map_err(|error| PortError::new(format!("could not parse {}: {error}", path.display())))
}

pub(crate) fn acquire_lock(state_root: &Path) -> Result<File, PortError> {
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(state_root.join("lock"))
        .map_err(io_error)?;
    FileExt::lock_exclusive(&lock).map_err(io_error)?;
    Ok(lock)
}

/// Open and exclusively lock the existing shared `.truss-core/lock`.
///
/// Unlike `acquire_lock`, this never creates the lock file: add-on state
/// operations require the core to have created it, and a missing lock is a
/// refusal rather than a reason to bootstrap core state.
pub(crate) fn acquire_existing_lock(state_root: &Path) -> Result<File, PortError> {
    let path = state_root.join("lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|error| {
            PortError::new(format!(
                ".truss-core/lock is required for add-on state operations: {error}"
            ))
        })?;
    FileExt::lock_exclusive(&lock).map_err(io_error)?;
    Ok(lock)
}

pub(crate) fn ensure_workspace_root(root: &Path) -> Result<(), PortError> {
    if !root.exists() {
        fs::create_dir_all(root).map_err(io_error)?;
    }
    validate_workspace_root(root)
}

pub(crate) fn validate_workspace_root(root: &Path) -> Result<(), PortError> {
    let metadata = fs::metadata(root).map_err(io_error)?;
    if !metadata.is_dir() {
        return Err(PortError::new(format!(
            "workspace root is not a directory: {}",
            root.display()
        )));
    }
    Ok(())
}

pub(crate) fn reject_symlink(path: &Path, label: &str) -> Result<(), PortError> {
    let metadata = fs::symlink_metadata(path).map_err(io_error)?;
    if metadata.file_type().is_symlink() {
        return Err(PortError::new(format!(
            "refusing symlink for managed path {label}: {}",
            path.display()
        )));
    }
    Ok(())
}

pub(crate) fn remove_if_exists(path: &Path) -> Result<(), PortError> {
    if fs::symlink_metadata(path).is_ok() {
        fs::remove_file(path).map_err(io_error)?;
    }
    Ok(())
}

pub(crate) fn remove_dir_if_exists(path: &Path) -> Result<(), PortError> {
    if fs::symlink_metadata(path).is_ok() {
        fs::remove_dir_all(path).map_err(io_error)?;
    }
    Ok(())
}

pub(crate) fn transaction_id() -> Result<String, PortError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| PortError::new(error.to_string()))?
        .as_nanos();
    Ok(format!("{nanos}-{}", std::process::id()))
}

pub(crate) fn hash_bytes(content: &[u8]) -> Result<ContentHash, PortError> {
    ContentHash::parse(format!("{:x}", Sha256::digest(content))).map_err(domain_error)
}

pub(crate) fn domain_error(error: DomainError) -> PortError {
    PortError::new(error.to_string())
}

pub(crate) fn io_error(error: std::io::Error) -> PortError {
    PortError::new(error.to_string())
}
