//! Persistence for the add-on conflict-session namespace.
//!
//! A staged add-on conflict session lives under
//! `.truss-core/addon-update/<name>/`, never `.truss-core/update/`, because an
//! older binary reads that path as a core session (decision
//! `.truss-core/docs/decisions/0003-add-on-state-ownership.md`, clause 7). The
//! in-memory shape is the shared [`UpdateResolutionSession`]; this module owns
//! only the per-add-on on-disk layout, so a session is add-on scoped and one
//! add-on can never see or clear another add-on's session.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::state_io::{
    copy_bytes, io_error, read_json, reject_symlink, remove_dir_if_exists, write_json_atomic,
};
use crate::application::PortError;
use crate::domain::{
    AddOnName, FrozenWorkspaceFile, RelativePath, ResolutionConflict, UpdateResolutionSession,
};

/// Directory under `.truss-core/` that holds every add-on conflict session.
pub(crate) const ADDON_UPDATE_DIR: &str = "addon-update";
const SESSION_FILE: &str = "session.json";
const SESSION_SCHEMA_VERSION: u32 = 1;
/// Per-session subdirectories, mirroring the core session layout.
const SESSION_DIRECTORIES: [&str; 5] = ["base", "local", "incoming", "resolved", "frozen"];

pub(crate) fn addon_update_root(state_root: &Path) -> PathBuf {
    state_root.join(ADDON_UPDATE_DIR)
}

/// The owned session directory for one add-on.
pub(crate) fn addon_session_root(state_root: &Path, name: &AddOnName) -> PathBuf {
    addon_update_root(state_root).join(name.as_str())
}

/// Persist one conflicted add-on plan for `name`.
///
/// Every conflict input (`base`, `local`, `incoming`) and its working
/// `resolved` copy, plus every frozen workspace observation, is written before
/// `session.json`, so a session is never visible while its inputs are partial.
/// The caller already holds the shared `.truss-core/lock` and has validated the
/// pre-existing core state; this function writes nothing outside the owned
/// add-on namespace.
pub(crate) fn stage_addon_session(
    state_root: &Path,
    name: &AddOnName,
    session: &UpdateResolutionSession,
) -> Result<(), PortError> {
    if session.conflicts.is_empty() {
        return Err(PortError::new(
            "cannot stage an empty add-on resolution session",
        ));
    }
    let parent = addon_update_root(state_root);
    if parent.exists() {
        reject_symlink(&parent, ".truss-core/addon-update")?;
    }
    let session_root = addon_session_root(state_root, name);
    if session_root.exists() {
        reject_symlink(&session_root, &format!(".truss-core/addon-update/{name}"))?;
    }
    remove_dir_if_exists(&session_root)?;
    fs::create_dir_all(&session_root).map_err(io_error)?;

    let mut conflicts = Vec::new();
    for conflict in &session.conflicts {
        validate_session_path(&session_root, &conflict.path)?;
        for (directory, content) in [
            ("base", &conflict.base),
            ("local", &conflict.local),
            ("incoming", &conflict.incoming),
            ("resolved", &conflict.resolved),
        ] {
            copy_bytes(
                content,
                &session_root.join(directory).join(conflict.path.as_str()),
            )?;
        }
        conflicts.push(ResolutionConflictDto {
            path: conflict.path.as_str().to_owned(),
        });
    }
    let mut frozen_files = Vec::new();
    for frozen in &session.frozen_files {
        validate_session_path(&session_root, &frozen.path)?;
        if let Some(content) = &frozen.content {
            copy_bytes(
                content,
                &session_root.join("frozen").join(frozen.path.as_str()),
            )?;
        }
        frozen_files.push(FrozenWorkspaceFileDto {
            path: frozen.path.as_str().to_owned(),
            present: frozen.content.is_some(),
        });
    }
    let dto = ResolutionSessionDto {
        schema_version: SESSION_SCHEMA_VERSION,
        from_version: session.from_version.clone(),
        to_version: session.to_version.clone(),
        conflicts,
        frozen_files,
    };
    write_json_atomic(&session_root.join(SESSION_FILE), &dto, "addon-resolution")
}

/// Read the staged add-on conflict session for `name`, or `None` when none is
/// pending. The caller already holds the shared lock.
pub(crate) fn load_addon_session(
    state_root: &Path,
    name: &AddOnName,
) -> Result<Option<UpdateResolutionSession>, PortError> {
    let session_root = addon_session_root(state_root, name);
    let session_path = session_root.join(SESSION_FILE);
    if !session_path.exists() {
        return Ok(None);
    }
    if session_root.exists() {
        reject_symlink(&session_root, &format!(".truss-core/addon-update/{name}"))?;
    }
    reject_symlink(
        &session_path,
        &format!(".truss-core/addon-update/{name}/session.json"),
    )?;
    let dto: ResolutionSessionDto = read_json(&session_path)?;
    if dto.schema_version != SESSION_SCHEMA_VERSION {
        return Err(PortError::new(format!(
            "unsupported add-on resolution schema: {}",
            dto.schema_version
        )));
    }
    let mut conflicts = Vec::new();
    for item in dto.conflicts {
        let path =
            RelativePath::parse(item.path).map_err(|error| PortError::new(error.to_string()))?;
        validate_session_path(&session_root, &path)?;
        conflicts.push(ResolutionConflict {
            base: read_session_file(&session_root, "base", &path)?,
            local: read_session_file(&session_root, "local", &path)?,
            incoming: read_session_file(&session_root, "incoming", &path)?,
            resolved: read_session_file(&session_root, "resolved", &path)?,
            path,
        });
    }
    let mut frozen_files = Vec::new();
    for item in dto.frozen_files {
        let path =
            RelativePath::parse(item.path).map_err(|error| PortError::new(error.to_string()))?;
        validate_session_path(&session_root, &path)?;
        let content = if item.present {
            Some(read_session_file(&session_root, "frozen", &path)?)
        } else {
            let target = session_root.join("frozen").join(path.as_str());
            if target.exists() {
                return Err(PortError::new(format!(
                    "absent frozen add-on path unexpectedly exists: {path}"
                )));
            }
            None
        };
        frozen_files.push(FrozenWorkspaceFile { path, content });
    }
    Ok(Some(UpdateResolutionSession {
        from_version: dto.from_version,
        to_version: dto.to_version,
        conflicts,
        frozen_files,
    }))
}

/// Remove only the owned add-on session directory.
///
/// Returns `true` when a session was present. It is idempotent: a second call
/// removes nothing and returns `false`. The `.truss-core/addon-update/`
/// container is removed only when this was its last session, so another
/// add-on's staged session is never touched, and `.truss-core/update/` is never
/// read, written, or cleared.
pub(crate) fn clear_addon_session(state_root: &Path, name: &AddOnName) -> Result<bool, PortError> {
    let parent = addon_update_root(state_root);
    if parent.exists() {
        reject_symlink(&parent, ".truss-core/addon-update")?;
    }
    let session_root = addon_session_root(state_root, name);
    let removed = session_root.exists();
    if removed {
        reject_symlink(&session_root, &format!(".truss-core/addon-update/{name}"))?;
        remove_dir_if_exists(&session_root)?;
    }
    if parent.exists() {
        // `remove_dir` fails while another add-on session remains; that failure
        // is the guard that keeps a sibling session intact.
        let _ = fs::remove_dir(&parent);
    }
    Ok(removed)
}

fn read_session_file(
    session_root: &Path,
    directory: &str,
    path: &RelativePath,
) -> Result<Vec<u8>, PortError> {
    let target = session_root.join(directory).join(path.as_str());
    let metadata = fs::symlink_metadata(&target).map_err(|error| {
        PortError::new(format!(
            "could not read {directory} resolution for {path}: {error}"
        ))
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(PortError::new(format!(
            "resolution input is not a regular file: {}",
            target.display()
        )));
    }
    fs::read(target).map_err(io_error)
}

fn validate_session_path(session_root: &Path, path: &RelativePath) -> Result<(), PortError> {
    for directory in SESSION_DIRECTORIES {
        let mut current = session_root.join(directory);
        if current.exists() {
            reject_symlink(&current, directory)?;
        }
        for component in path.as_str().split('/') {
            current.push(component);
            if current.exists() {
                reject_symlink(&current, path.as_str())?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
struct ResolutionSessionDto {
    schema_version: u32,
    from_version: String,
    to_version: String,
    conflicts: Vec<ResolutionConflictDto>,
    frozen_files: Vec<FrozenWorkspaceFileDto>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResolutionConflictDto {
    path: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct FrozenWorkspaceFileDto {
    path: String,
    present: bool,
}
