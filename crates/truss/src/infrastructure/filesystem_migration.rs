//! The filesystem adapter for `truss migrate` (D-06, D-08, D-09, D-12, D-13).
//!
//! This module owns every byte-level action of a migration: structural
//! inventory and classification, the stable lock, the retained verified
//! backup, the stage, per-path publication, the integration writes, legacy
//! retirement, the crash-monotonic journal, rollback, and recovery discovery.
//! It reuses the proven primitives in [`state_io`](super::state_io) —
//! `copy_bytes_atomic`, `hash_bytes`, `reject_symlink`, `io_error` — and does
//! not extend [`transaction`](super::transaction), whose journal and
//! `.truss-backup/` contract belong to core update and add-on operations.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

use super::state_io::{copy_bytes_atomic, hash_bytes, io_error, reject_symlink};
use crate::application::{
    CanonicalBlocks, MigrationExecution, MigrationInspection, MigrationPort, PortError,
};
use crate::domain::{
    authority_destination, compose_entrypoint, core_destination, delivery_destination,
    rewrite_document_tokens, ContentHash, IncompleteJournal, IntegrationKind, IntegrationOperation,
    InventoryEntry, InventoryKind, MigrationOperation, MigrationPlan, MigrationReason,
    MigrationState, OperationKind, BACKUP_LOCK, BACKUP_ROOT, ENTRYPOINTS, INTEGRATION_IGNORE_RULES,
    JOURNAL_FILE, LEGACY_CORE_ROOT, LEGACY_DELIVERY_ROOT, LEGACY_DISPATCH_ROOT, LEGACY_ROOTS,
    LOCAL_ONLY_IGNORE_RULES, NEW_CORE_ROOT,
};

/// The one adapter the composition root wires into `MigrationApplication`.
#[derive(Clone, Copy, Default)]
pub struct FileSystemMigration;

impl MigrationPort for FileSystemMigration {
    fn supported(&self) -> bool {
        cfg!(target_os = "linux")
    }

    fn inspect(
        &self,
        root: &Path,
        blocks: &CanonicalBlocks,
    ) -> Result<MigrationInspection, PortError> {
        inspect(root, blocks)
    }

    fn execute(&self, root: &Path, plan: &MigrationPlan) -> Result<MigrationExecution, PortError> {
        if !self.supported() {
            let mut execution = MigrationExecution::new(MigrationState::Blocked);
            execution.reason = Some(MigrationReason::UnsupportedApplyPlatform);
            return Ok(execution);
        }
        let backup_root = root.join(BACKUP_ROOT);
        fs::create_dir_all(&backup_root).map_err(io_error)?;
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(backup_root.join(BACKUP_LOCK))
            .map_err(io_error)?;
        FileExt::lock_exclusive(&lock).map_err(io_error)?;
        let result = execute_locked(root, &backup_root, plan);
        let _ = FileExt::unlock(&lock);
        result
    }

    fn recover(&self, root: &Path) -> Result<MigrationExecution, PortError> {
        recover(root)
    }
}

// ---------------------------------------------------------------------------
// Inspection: structural inventory and classification (D-02, D-10, D-11)
// ---------------------------------------------------------------------------

fn inspect(root: &Path, blocks: &CanonicalBlocks) -> Result<MigrationInspection, PortError> {
    if !root.is_dir() {
        return Err(PortError::new(format!(
            "migration directory is not a directory: {}",
            root.display()
        )));
    }
    let mut inspection = MigrationInspection::empty(repository_name(root)?);

    let new_root = root.join(NEW_CORE_ROOT);
    let new_installed = new_root.join("manifest.json").is_file() || new_root.join("base").is_dir();

    for legacy in LEGACY_ROOTS {
        if fs::symlink_metadata(root.join(legacy)).is_ok() {
            inspection.legacy_roots.push(legacy.to_owned());
        }
    }

    let scan = scan_journals(root)?;
    inspection.evidence.extend(scan.evidence.clone());
    inspection.existing_backup = scan
        .committed
        .last()
        .map(|journal| format!("{}/", journal.tx_dir.display()));
    if let Some(reason) = scan.blocked {
        inspection.blocked = Some(reason);
        inspection.conflicts.extend(scan.evidence);
        return Ok(inspection);
    }
    match scan.incomplete.len() {
        0 => {}
        1 => {
            let found = &scan.incomplete[0];
            inspection.recovery = Some(IncompleteJournal {
                transaction_id: found.journal.transaction_id.clone(),
                phase: found.journal.phase.as_str().to_owned(),
                path: found.tx_dir.join(JOURNAL_FILE).display().to_string(),
            });
            inspection.evidence.push(format!(
                "incomplete migration transaction {} at phase {}",
                found.journal.transaction_id,
                found.journal.phase.as_str()
            ));
            return Ok(inspection);
        }
        _ => {
            inspection.blocked = Some(MigrationReason::MultipleJournals);
            inspection.evidence.push(format!(
                "{} incomplete migration transactions found; exactly one is recoverable",
                scan.incomplete.len()
            ));
            return Ok(inspection);
        }
    }

    if inspection.legacy_roots.is_empty() {
        if new_installed {
            inspection.already_migrated = true;
            inspection
                .evidence
                .push("only the .truss/core installation is present".to_owned());
        } else {
            inspection.not_installed = true;
            inspection
                .evidence
                .push("no .truss installation is present".to_owned());
        }
        return Ok(inspection);
    }

    if new_installed {
        inspection.evidence.push(
            "both the legacy and the new installation roots are present; migrate reconciles them"
                .to_owned(),
        );
    }

    classify(root, &mut inspection, blocks)?;
    Ok(inspection)
}

fn classify(
    root: &Path,
    inspection: &mut MigrationInspection,
    blocks: &CanonicalBlocks,
) -> Result<(), PortError> {
    let legacy_core = root.join(LEGACY_CORE_ROOT);
    let mut members: BTreeMap<String, String> = BTreeMap::new();
    let mut pending = Vec::new();

    if fs::symlink_metadata(&legacy_core).is_ok() {
        reject_symlink(&legacy_core, LEGACY_CORE_ROOT)?;
        let manifest_path = legacy_core.join("manifest.json");
        if !manifest_path.is_file() {
            inspection.blocked = Some(MigrationReason::InvalidState);
            inspection.conflicts.push(format!(
                "{LEGACY_CORE_ROOT}/manifest.json is missing, so the legacy installation cannot be validated"
            ));
            return Ok(());
        }
        let bytes = fs::read(&manifest_path).map_err(io_error)?;
        let manifest: StateManifest = match serde_json::from_slice(&bytes) {
            Ok(manifest) => manifest,
            Err(error) => {
                inspection.blocked = Some(MigrationReason::InvalidState);
                inspection
                    .conflicts
                    .push(format!("{LEGACY_CORE_ROOT}/manifest.json: {error}"));
                return Ok(());
            }
        };
        if manifest.schema_version != 1 {
            inspection.blocked = Some(MigrationReason::UnknownSchema);
            inspection.conflicts.push(format!(
                "{LEGACY_CORE_ROOT}/manifest.json declares unsupported schema {}",
                manifest.schema_version
            ));
            return Ok(());
        }
        for file in &manifest.files {
            members.insert(file.path.clone(), file.upstream_sha256.clone());
        }
        for name in [
            "transaction.json",
            "update",
            "update-candidate",
            "addon-update",
        ] {
            if fs::symlink_metadata(legacy_core.join(name)).is_ok() {
                pending.push(format!("{LEGACY_CORE_ROOT}/{name}"));
            }
        }
    }

    if !pending.is_empty() {
        inspection.blocked = Some(MigrationReason::PendingSession);
        inspection.conflicts.extend(pending);
        return Ok(());
    }

    // Inventory every legacy tree, refusing symlinks and unknown entry kinds.
    let mut inventory = Vec::new();
    let mut symlinks = Vec::new();
    for legacy in inspection.legacy_roots.clone() {
        walk_legacy(&root.join(&legacy), &legacy, &mut inventory, &mut symlinks)?;
    }
    if !symlinks.is_empty() {
        inspection.blocked = Some(MigrationReason::UnsafeSymlink);
        inspection.conflicts.extend(symlinks);
        return Ok(());
    }
    inventory.sort_by(|left, right| left.path.cmp(&right.path));

    let run_key = recognize_run_key(root, inspection)?;
    if inspection.blocked.is_some() {
        return Ok(());
    }

    let mut conflicts = Vec::new();
    let operations = plan_operations(
        root,
        &inventory,
        &members,
        run_key.as_deref(),
        &mut conflicts,
        &mut symlinks,
        inspection,
    )?;
    if !symlinks.is_empty() {
        inspection.blocked = Some(MigrationReason::UnsafeSymlink);
        inspection.conflicts.extend(symlinks);
        return Ok(());
    }
    if !conflicts.is_empty() {
        inspection.blocked = Some(MigrationReason::Collision);
        inspection.conflicts.extend(conflicts);
        return Ok(());
    }
    if inspection.blocked.is_some() {
        return Ok(());
    }

    let mut operations = operations;
    operations.sort_by(|left, right| left.destination.cmp(&right.destination));

    let integration = plan_integration(root, blocks, inspection)?;
    if inspection.blocked.is_some() {
        return Ok(());
    }

    inspection.run_key = run_key;
    inspection.operations = operations;
    inspection.integration = integration;
    inspection.inventory = inventory;
    Ok(())
}

fn walk_legacy(
    directory: &Path,
    relative: &str,
    inventory: &mut Vec<InventoryEntry>,
    symlinks: &mut Vec<String>,
) -> Result<(), PortError> {
    let mut entries = fs::read_dir(directory)
        .map_err(io_error)?
        .map(|entry| entry.map_err(io_error))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{relative}/{name}");
        let absolute = entry.path();
        let metadata = fs::symlink_metadata(&absolute).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            symlinks.push(path);
        } else if metadata.is_dir() {
            inventory.push(InventoryEntry {
                path: path.clone(),
                kind: InventoryKind::Directory,
                size: 0,
                hash: None,
            });
            walk_legacy(&absolute, &path, inventory, symlinks)?;
        } else if metadata.is_file() {
            let bytes = fs::read(&absolute).map_err(io_error)?;
            inventory.push(InventoryEntry {
                path,
                kind: InventoryKind::File,
                size: bytes.len() as u64,
                hash: Some(hash_bytes(&bytes)?),
            });
        } else {
            symlinks.push(format!("{path} (unsupported filesystem entry)"));
        }
    }
    Ok(())
}

/// The deterministic run-key predicate (D-02, REQ-017–REQ-019).
fn recognize_run_key(
    root: &Path,
    inspection: &mut MigrationInspection,
) -> Result<Option<String>, PortError> {
    let mut proven: Option<(String, String)> = None;
    let mut candidates: Vec<String> = Vec::new();
    let mut malformed = false;
    for legacy in [LEGACY_DELIVERY_ROOT, LEGACY_DISPATCH_ROOT] {
        let directory = root.join(legacy);
        if fs::symlink_metadata(&directory).is_err() {
            continue;
        }
        reject_symlink(&directory, legacy)?;
        let mut entries = fs::read_dir(&directory)
            .map_err(io_error)?
            .map(|entry| entry.map_err(io_error))
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        if entries.is_empty() {
            continue;
        }
        for entry in &entries {
            candidates.push(format!("{legacy}/{}", entry.file_name().to_string_lossy()));
        }
        let only = &entries[0];
        let file_type = only.file_type().map_err(io_error)?;
        let shape_is_valid = entries.len() == 1
            && file_type.is_dir()
            && !file_type.is_symlink()
            && safe_component(&only.file_name().to_string_lossy());
        if !shape_is_valid {
            malformed = true;
            continue;
        }
        let name = only.file_name().to_string_lossy().into_owned();
        match &proven {
            None => proven = Some((legacy.to_owned(), name)),
            Some((_, existing)) if *existing == name => {}
            Some((origin, existing)) => {
                inspection.evidence.push(format!(
                    "{origin} proves run key {existing:?} but {legacy} proves {name:?}"
                ));
                malformed = true;
            }
        }
    }
    if malformed {
        inspection.blocked = Some(MigrationReason::AmbiguousRunKey);
        inspection.conflicts = candidates;
        return Ok(None);
    }
    Ok(proven.map(|(_, name)| name))
}

fn safe_component(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && !name.contains(':')
}

#[allow(clippy::too_many_arguments)]
fn plan_operations(
    root: &Path,
    inventory: &[InventoryEntry],
    members: &BTreeMap<String, String>,
    run_key: Option<&str>,
    conflicts: &mut Vec<String>,
    symlinks: &mut Vec<String>,
    inspection: &mut MigrationInspection,
) -> Result<Vec<MigrationOperation>, PortError> {
    let mut operations = Vec::new();
    for entry in inventory {
        if entry.kind != InventoryKind::File {
            continue;
        }
        let path = &entry.path;
        let is_member = members.contains_key(path.as_str());
        let authority = authority_destination(path);
        let destination = if path.starts_with(&format!("{LEGACY_CORE_ROOT}/")) {
            if is_member {
                core_destination(path)
            } else if let Some(authority) = authority.clone() {
                authority
            } else if is_core_state_path(path) {
                core_destination(path)
            } else {
                inspection.blocked = Some(MigrationReason::UnknownDocument);
                inspection.conflicts.push(format!(
                    "{path} is neither a manifest member nor a recognized unmanaged document"
                ));
                return Ok(operations);
            }
        } else if path.starts_with(&format!("{LEGACY_DELIVERY_ROOT}/"))
            || path.starts_with(&format!("{LEGACY_DISPATCH_ROOT}/"))
        {
            let legacy = if path.starts_with(&format!("{LEGACY_DELIVERY_ROOT}/")) {
                LEGACY_DELIVERY_ROOT
            } else {
                LEGACY_DISPATCH_ROOT
            };
            match run_key.and_then(|key| delivery_destination(legacy, key, path)) {
                Some(destination) => destination,
                None => {
                    inspection.blocked = Some(MigrationReason::AmbiguousRunKey);
                    inspection.conflicts.push(path.clone());
                    return Ok(operations);
                }
            }
        } else {
            inspection.blocked = Some(MigrationReason::UnknownLegacyEntry);
            inspection.conflicts.push(path.clone());
            return Ok(operations);
        };

        let content = if is_state_record(path) {
            let bytes = read_bytes(root, path)?;
            match rewrite_state_record(&bytes, path) {
                Ok(rewritten) => Some(rewritten),
                Err(reason) => {
                    inspection.blocked = Some(reason);
                    inspection.conflicts.push(path.clone());
                    return Ok(operations);
                }
            }
        } else if authority.is_some() && !is_member {
            let bytes = read_bytes(root, path)?;
            match String::from_utf8(bytes) {
                Ok(text) => Some(rewrite_document_tokens(&text, run_key).into_bytes()),
                Err(_) => {
                    inspection.blocked = Some(MigrationReason::NonUtf8Document);
                    inspection.conflicts.push(path.clone());
                    return Ok(operations);
                }
            }
        } else {
            None
        };

        let intended = match &content {
            Some(bytes) => bytes.clone(),
            None => read_bytes(root, path)?,
        };
        let after_hash = hash_bytes(&intended)?;
        let (kind, before_hash) = resolve_destination(root, &destination, &intended, conflicts)?;
        operations.push(MigrationOperation {
            source: Some(path.clone()),
            destination,
            kind,
            source_hash: entry.hash.clone(),
            before_hash,
            after_hash,
            content,
        });
    }
    let _ = symlinks;
    Ok(operations)
}

fn is_state_record(path: &str) -> bool {
    path == format!("{LEGACY_CORE_ROOT}/manifest.json")
        || path == format!("{LEGACY_CORE_ROOT}/addons.json")
}

fn is_core_state_path(path: &str) -> bool {
    const FILES: [&str; 5] = [
        ".truss-core/.gitignore",
        ".truss-core/lock",
        ".truss-core/manifest.json",
        ".truss-core/addons.json",
        ".truss-core/transaction.json",
    ];
    if FILES.contains(&path) {
        return true;
    }
    const PREFIXES: [&str; 6] = [
        ".truss-core/base/",
        ".truss-core/base-addons/",
        ".truss-core/bin/",
        ".truss-core/update/",
        ".truss-core/update-candidate/",
        ".truss-core/addon-update/",
    ];
    PREFIXES.iter().any(|prefix| path.starts_with(prefix))
}

fn resolve_destination(
    root: &Path,
    destination: &str,
    intended: &[u8],
    conflicts: &mut Vec<String>,
) -> Result<(OperationKind, Option<ContentHash>), PortError> {
    let target = root.join(destination);
    match fs::symlink_metadata(&target) {
        Err(_) => Ok((OperationKind::Create, None)),
        Ok(metadata) if metadata.file_type().is_symlink() => {
            conflicts.push(format!(
                "{destination} is a symlink, not a migratable destination"
            ));
            Ok((OperationKind::Create, None))
        }
        Ok(metadata) if metadata.is_dir() => {
            conflicts.push(format!("{destination} is a directory, not a file"));
            Ok((OperationKind::Create, None))
        }
        Ok(_) => {
            let existing = fs::read(&target).map_err(io_error)?;
            if existing == intended {
                Ok((OperationKind::Preserve, Some(hash_bytes(&existing)?)))
            } else {
                conflicts.push(format!(
                    "{destination} exists with different bytes; the migration never overwrites it"
                ));
                Ok((OperationKind::Create, None))
            }
        }
    }
}

/// Plan the entrypoint block replacements and the integration ignore repair.
///
/// A defect is recorded on the inspection as the stable `invalid_markers`
/// reason, so a corrupt entrypoint is a refusal the operator sees rather than
/// a transport error (D-03, REQ-022). A corrupt entrypoint is never repaired.
fn plan_integration(
    root: &Path,
    blocks: &CanonicalBlocks,
    inspection: &mut MigrationInspection,
) -> Result<Vec<IntegrationOperation>, PortError> {
    let mut integration = Vec::new();
    for name in ENTRYPOINTS {
        let path = root.join(name);
        let metadata = match fs::symlink_metadata(&path) {
            Err(_) => continue,
            Ok(metadata) => metadata,
        };
        if metadata.file_type().is_symlink() {
            inspection.conflicts.push(format!("{name} is a symlink"));
            inspection.blocked = Some(MigrationReason::UnsafeSymlink);
            return Ok(integration);
        }
        if !metadata.is_file() {
            inspection
                .conflicts
                .push(format!("{name} is not a regular file"));
            inspection.blocked = Some(MigrationReason::InvalidState);
            return Ok(integration);
        }
        let existing = fs::read(&path).map_err(io_error)?;
        let canonical = blocks
            .for_path(name)
            .ok_or_else(|| PortError::new(format!("no canonical block for {name}")))?;
        match compose_entrypoint(&existing, canonical) {
            Ok(composed) => {
                if composed != existing {
                    integration.push(IntegrationOperation {
                        path: name.to_owned(),
                        kind: IntegrationKind::Entrypoint,
                        before_hash: Some(hash_bytes(&existing)?),
                        after_hash: hash_bytes(&composed)?,
                        content: composed,
                    });
                }
            }
            Err(defect) => {
                inspection
                    .conflicts
                    .push(format!("{name}: {}", defect.as_str()));
                inspection.blocked = Some(MigrationReason::InvalidMarkers);
                return Ok(integration);
            }
        }
    }

    if let Some(ignore) = plan_ignore(root)? {
        if Some(&ignore.after_hash) != ignore.before_hash.as_ref() {
            integration.push(ignore);
        }
    }
    Ok(integration)
}

// ---------------------------------------------------------------------------
// State-record rewrites (D-10)
// ---------------------------------------------------------------------------

/// Rewrite one schema-1 state record's structural path fields.
///
/// Only path fields move: schema versions, versions, ordering, membership,
/// and every `upstream_sha256` are preserved, because destination naming is
/// not payload content. JSON formatting may change; backup equality applies to
/// the originals and the enumerated transformation applies to the rewrite.
fn rewrite_state_record(bytes: &[u8], path: &str) -> Result<Vec<u8>, MigrationReason> {
    if path.ends_with("manifest.json") {
        let mut manifest: StateManifest =
            serde_json::from_slice(bytes).map_err(|_| MigrationReason::InvalidState)?;
        if manifest.schema_version != 1 {
            return Err(MigrationReason::UnknownSchema);
        }
        let mut seen = std::collections::BTreeSet::new();
        for file in &mut manifest.files {
            if !is_hex_hash(&file.upstream_sha256) {
                return Err(MigrationReason::InvalidState);
            }
            file.path = core_destination(&file.path);
            if !seen.insert(file.path.clone()) {
                return Err(MigrationReason::InvalidState);
            }
        }
        serde_json::to_vec_pretty(&manifest).map_err(|_| MigrationReason::InvalidState)
    } else {
        let mut value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| MigrationReason::InvalidState)?;
        rewrite_json_paths(&mut value);
        serde_json::to_vec_pretty(&value).map_err(|_| MigrationReason::InvalidState)
    }
}

fn is_hex_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn rewrite_json_paths(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => *text = core_destination(text),
        serde_json::Value::Array(items) => items.iter_mut().for_each(rewrite_json_paths),
        serde_json::Value::Object(map) => {
            for item in map.values_mut() {
                rewrite_json_paths(item);
            }
        }
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {}
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct StateManifest {
    schema_version: u32,
    #[serde(default)]
    core_version: String,
    #[serde(default)]
    files: Vec<StateManifestFile>,
}

#[derive(Debug, Deserialize, Serialize)]
struct StateManifestFile {
    path: String,
    upstream_sha256: String,
}

// ---------------------------------------------------------------------------
// Integration ignore repair (BR-17, REQ-024, D-01)
// ---------------------------------------------------------------------------

const GITIGNORE: &str = ".gitignore";

/// The Truss rules the integration rewrite owns or withdraws. Any other rule
/// that merely mentions `.truss` (for example `.truss/core/bin/truss-v0-migrate`
/// in this repository) is unrelated and stays byte-identical.
const WITHDRAWN_RULES: [&str; 20] = [
    ".truss/",
    "/.truss/",
    ".truss/authority/",
    "/.truss/authority/",
    ".truss/delivery/",
    "/.truss/delivery/",
    ".truss/core/",
    "/.truss/core/",
    ".truss-core/",
    "/.truss-core/",
    ".truss/core/bin/truss",
    "/.truss/core/bin/truss",
    ".truss/core/bin/truss.exe",
    "/.truss/core/bin/truss.exe",
    ".truss-core/bin/truss",
    "/.truss-core/bin/truss",
    ".truss-core/bin/truss.exe",
    "/.truss-core/bin/truss.exe",
    ".truss-migration-backup/",
    "/.truss-migration-backup/",
];

fn is_truss_ignore_rule(line: &str) -> bool {
    !line.is_empty()
        && !line.starts_with('#')
        && (line.starts_with(".truss") || line.starts_with("/.truss"))
}

fn plan_ignore(root: &Path) -> Result<Option<IntegrationOperation>, PortError> {
    let (target, existing, local_only) = resolve_ignore(root)?;
    let repaired = repair_ignore(existing.as_deref(), local_only);
    let before_hash = match &existing {
        Some(text) => Some(hash_bytes(text.as_bytes())?),
        None => None,
    };
    let after_hash = hash_bytes(repaired.as_bytes())?;
    if before_hash.as_ref() == Some(&after_hash) {
        return Ok(None);
    }
    Ok(Some(IntegrationOperation {
        path: target,
        kind: IntegrationKind::Ignore,
        before_hash,
        after_hash,
        content: repaired.into_bytes(),
    }))
}

/// Choose the integration file the repository already uses for Truss rules,
/// and detect local-only intent. A repository with no Truss rules receives
/// them where the installer would put them: the repository-root `.gitignore`.
fn resolve_ignore(root: &Path) -> Result<(String, Option<String>, bool), PortError> {
    let gitignore = read_optional_text(&root.join(GITIGNORE))?;
    let exclude_relative = info_exclude_relative(root);
    let exclude = match &exclude_relative {
        Some(relative) => read_optional_text(&root.join(relative))?,
        None => None,
    };
    let local_only = [&gitignore, &exclude].iter().any(|text| {
        text.as_ref()
            .map(|text| text.lines().any(|line| is_local_only_rule(line.trim())))
            .unwrap_or(false)
    });
    let gitignore_has = gitignore
        .as_ref()
        .map(|text| text.lines().any(|line| is_truss_ignore_rule(line.trim())))
        .unwrap_or(false);
    let exclude_has = exclude
        .as_ref()
        .map(|text| text.lines().any(|line| is_truss_ignore_rule(line.trim())))
        .unwrap_or(false);
    if gitignore_has {
        Ok((GITIGNORE.to_owned(), gitignore, local_only))
    } else if exclude_has {
        Ok((
            exclude_relative.unwrap_or_else(|| GITIGNORE.to_owned()),
            exclude,
            local_only,
        ))
    } else {
        Ok((GITIGNORE.to_owned(), gitignore, local_only))
    }
}

fn is_local_only_rule(line: &str) -> bool {
    line == ".truss/"
}

/// Repair the selected integration file: withdraw the owned legacy spellings,
/// preserve every unrelated byte, and add the relative canonical rules.
fn repair_ignore(existing: Option<&str>, local_only: bool) -> String {
    let rules: &[&str] = if local_only {
        &LOCAL_ONLY_IGNORE_RULES
    } else {
        &INTEGRATION_IGNORE_RULES
    };
    let mut output = String::new();
    if let Some(text) = existing {
        for line in text.split_inclusive('\n') {
            let trimmed = line.trim_end_matches(['\n', '\r']).trim();
            if WITHDRAWN_RULES.contains(&trimmed) {
                continue;
            }
            output.push_str(line);
        }
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    let present: Vec<String> = output.lines().map(|line| line.trim().to_owned()).collect();
    for rule in rules {
        if !present.iter().any(|line| line == rule) {
            output.push_str(rule);
            output.push('\n');
        }
    }
    output
}

fn read_optional_text(path: &Path) -> Result<Option<String>, PortError> {
    match fs::symlink_metadata(path) {
        Err(_) => Ok(None),
        Ok(metadata) if metadata.file_type().is_symlink() => Err(PortError::new(format!(
            "refusing symlink: {}",
            path.display()
        ))),
        Ok(metadata) if !metadata.is_file() => Err(PortError::new(format!(
            "{} is not a regular file",
            path.display()
        ))),
        Ok(_) => fs::read_to_string(path).map(Some).map_err(io_error),
    }
}

/// The resolved `info/exclude` path relative to the repository root, when the
/// git directory is inside the repository (ADR 0005 item 2).
fn info_exclude_relative(root: &Path) -> Option<String> {
    let git_dir = resolve_git_dir(root)?;
    let exclude = normalize(&git_dir.join("info").join("exclude"));
    let relative = exclude.strip_prefix(normalize(root)).ok()?;
    Some(posix(relative))
}

fn resolve_git_dir(root: &Path) -> Option<PathBuf> {
    let dot_git = root.join(".git");
    let metadata = fs::symlink_metadata(&dot_git).ok()?;
    if metadata.is_dir() {
        return Some(dot_git);
    }
    if !metadata.is_file() {
        return None;
    }
    let text = fs::read_to_string(&dot_git).ok()?;
    let value = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:"))?
        .trim();
    let path = PathBuf::from(value);
    Some(if path.is_absolute() {
        path
    } else {
        root.join(path)
    })
}

fn normalize(path: &Path) -> PathBuf {
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                output.pop();
            }
            other => output.push(other.as_os_str()),
        }
    }
    output
}

fn posix(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

// ---------------------------------------------------------------------------
// Journal schema and recovery discovery (D-08, D-13, REQ-031, REQ-032)
// ---------------------------------------------------------------------------

const JOURNAL_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MigrationJournal {
    schema_version: u32,
    transaction_id: String,
    repository: String,
    repository_identity: String,
    created_at: String,
    state: JournalState,
    phase: JournalPhase,
    run_key: Option<String>,
    legacy_roots: Vec<String>,
    inventory: Vec<JournalInventory>,
    integration: Vec<JournalIntegration>,
    operations: Vec<JournalOperation>,
    created_paths: Vec<String>,
    backup_relative: String,
    stage_relative: String,
    verification: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum JournalState {
    InProgress,
    Committed,
    RolledBack,
    RollbackFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum JournalPhase {
    Inventoried,
    BackupVerified,
    StageVerified,
    NamespacesPublished,
    IntegrationPublished,
    LegacyRetired,
    FinalVerified,
    Committed,
}

impl JournalPhase {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Inventoried => "inventoried",
            Self::BackupVerified => "backup_verified",
            Self::StageVerified => "stage_verified",
            Self::NamespacesPublished => "namespaces_published",
            Self::IntegrationPublished => "integration_published",
            Self::LegacyRetired => "legacy_retired",
            Self::FinalVerified => "final_verified",
            Self::Committed => "committed",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JournalInventory {
    path: String,
    kind: String,
    size: u64,
    hash: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JournalIntegration {
    path: String,
    kind: String,
    before_hash: Option<String>,
    after_hash: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JournalOperation {
    destination: String,
    source: Option<String>,
    kind: String,
    before_hash: Option<String>,
    after_hash: String,
    created: bool,
}

struct DiscoveredJournal {
    tx_dir: PathBuf,
    journal: MigrationJournal,
}

#[derive(Default)]
struct JournalScan {
    incomplete: Vec<DiscoveredJournal>,
    committed: Vec<DiscoveredJournal>,
    blocked: Option<MigrationReason>,
    evidence: Vec<String>,
}

fn scan_journals(root: &Path) -> Result<JournalScan, PortError> {
    let mut scan = JournalScan::default();
    let backup_root = root.join(BACKUP_ROOT);
    if fs::symlink_metadata(&backup_root).is_err() {
        return Ok(scan);
    }
    reject_symlink(&backup_root, BACKUP_ROOT)?;
    if !backup_root.is_dir() {
        return Ok(scan);
    }
    let identity = repository_identity(root)?;
    let mut directories = fs::read_dir(&backup_root)
        .map_err(io_error)?
        .map(|entry| entry.map_err(io_error))
        .collect::<Result<Vec<_>, _>>()?;
    directories.sort_by_key(|entry| entry.file_name());
    for entry in directories {
        let file_type = entry.file_type().map_err(io_error)?;
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let journal_path = entry.path().join(JOURNAL_FILE);
        if !journal_path.is_file() {
            continue;
        }
        let bytes = fs::read(&journal_path).map_err(io_error)?;
        let journal: MigrationJournal = match serde_json::from_slice(&bytes) {
            Ok(journal) => journal,
            Err(error) => {
                scan.blocked = Some(MigrationReason::MalformedJournal);
                scan.evidence
                    .push(format!("{}: {error}", journal_path.display()));
                return Ok(scan);
            }
        };
        if journal.schema_version != JOURNAL_SCHEMA_VERSION {
            scan.blocked = Some(MigrationReason::MalformedJournal);
            scan.evidence.push(format!(
                "{} declares unsupported schema {}",
                journal_path.display(),
                journal.schema_version
            ));
            return Ok(scan);
        }
        if journal.repository_identity != identity {
            scan.blocked = Some(MigrationReason::RepoMismatch);
            scan.evidence.push(format!(
                "{} was written for {}",
                journal_path.display(),
                journal.repository
            ));
            return Ok(scan);
        }
        let discovered = DiscoveredJournal {
            tx_dir: entry.path(),
            journal,
        };
        match discovered.journal.state {
            JournalState::Committed => scan.committed.push(discovered),
            JournalState::RolledBack => {}
            JournalState::InProgress | JournalState::RollbackFailed => {
                scan.incomplete.push(discovered)
            }
        }
    }
    Ok(scan)
}

/// The commit-monotonic journal write: rewrite and sync after every durable
/// boundary and every published operation.
fn write_journal(tx_dir: &Path, journal: &MigrationJournal) -> Result<(), PortError> {
    let bytes = serde_json::to_vec_pretty(journal)
        .map_err(|error| PortError::new(format!("could not encode journal: {error}")))?;
    let mut file = fs::File::create(tx_dir.join(JOURNAL_FILE)).map_err(io_error)?;
    use std::io::Write;
    file.write_all(&bytes).map_err(io_error)?;
    file.sync_all().map_err(io_error)
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn repository_name(root: &Path) -> Result<String, PortError> {
    Ok(fs::canonicalize(root)
        .unwrap_or_else(|_| root.to_path_buf())
        .display()
        .to_string())
}

fn repository_identity(root: &Path) -> Result<String, PortError> {
    Ok(format!(
        "{:x}",
        <sha2::Sha256 as sha2::Digest>::digest(repository_name(root)?.as_bytes())
    ))
}

fn read_bytes(root: &Path, relative: &str) -> Result<Vec<u8>, PortError> {
    let target = root.join(relative);
    reject_symlink(&target, relative)?;
    fs::read(&target).map_err(io_error)
}

fn read_hash(path: &Path) -> Result<Option<ContentHash>, PortError> {
    match fs::symlink_metadata(path) {
        Err(_) => Ok(None),
        Ok(metadata) if metadata.is_file() => {
            let bytes = fs::read(path).map_err(io_error)?;
            Ok(Some(hash_bytes(&bytes)?))
        }
        Ok(_) => Ok(None),
    }
}

fn hash_string(path: &Path) -> Result<Option<String>, PortError> {
    Ok(read_hash(path)?.map(|hash| hash.as_str().to_owned()))
}

fn transaction_id() -> Result<String, PortError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| PortError::new(error.to_string()))?
        .as_nanos();
    Ok(format!("{nanos}-{}", std::process::id()))
}

/// The frozen UTC timestamp `YYYYMMDDTHHMMSS.NNNNNNNNNZ` (D-07).
fn format_utc(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let seconds = duration.as_secs() as i64;
    let nanos = duration.subsec_nanos();
    let days = seconds.div_euclid(86_400);
    let remainder = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = remainder / 3600;
    let minute = (remainder % 3600) / 60;
    let second = remainder % 60;
    format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}.{nanos:09}Z")
}

/// Howard Hinnant's civil-from-days, so no date dependency is added.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Choose the transaction directory, adding a deterministic numeric suffix
/// when the frozen timestamp is already taken (D-07).
fn unique_tx_dir(backup_root: &Path, timestamp: &str) -> Result<PathBuf, PortError> {
    let mut candidate = backup_root.join(timestamp);
    let mut suffix = 0u32;
    while fs::symlink_metadata(&candidate).is_ok() {
        suffix += 1;
        candidate = backup_root.join(format!("{timestamp}-{suffix}"));
    }
    Ok(candidate)
}

/// Create every missing parent of `target` and record the transaction-created
/// directories so rollback removes exactly what the transaction added.
fn ensure_parent_dirs(
    root: &Path,
    target: &Path,
    created: &mut Vec<String>,
) -> Result<(), PortError> {
    let parent = target
        .parent()
        .ok_or_else(|| PortError::new(format!("{} has no parent", target.display())))?;
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| PortError::new("destination escaped the repository root"))?;
    let mut current = root.to_path_buf();
    let mut missing = Vec::new();
    for component in relative.components() {
        current.push(component);
        if fs::symlink_metadata(&current).is_err() {
            missing.push(current.clone());
        }
    }
    for directory in &missing {
        fs::create_dir_all(directory).map_err(io_error)?;
    }
    for directory in missing {
        if let Ok(relative) = directory.strip_prefix(root) {
            created.push(posix(relative));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Apply: revalidate, journal, verified backup, stage, publish (D-08, D-09, D-13)
// ---------------------------------------------------------------------------

fn execute_locked(
    root: &Path,
    backup_root: &Path,
    plan: &MigrationPlan,
) -> Result<MigrationExecution, PortError> {
    let conflicts = revalidate(root, plan)?;
    if !conflicts.is_empty() {
        let mut execution = MigrationExecution::new(MigrationState::Blocked);
        execution.reason = Some(MigrationReason::StaleInventory);
        execution.conflicts = conflicts;
        return Ok(execution);
    }

    let id = transaction_id()?;
    let created_at = format_utc(SystemTime::now());
    let tx_dir = unique_tx_dir(backup_root, &created_at)?;
    fs::create_dir_all(&tx_dir).map_err(io_error)?;

    let mut journal = initial_journal(root, plan, id.clone(), created_at)?;
    write_journal(&tx_dir, &journal)?;

    match run_transaction(root, &tx_dir, plan, &mut journal) {
        Ok(()) => {
            journal.state = JournalState::Committed;
            journal.phase = JournalPhase::Committed;
            write_journal(&tx_dir, &journal)?;
            let mut execution = MigrationExecution::new(MigrationState::Migrated);
            execution.backup_path = Some(format!("{}/", tx_dir.display()));
            execution.transaction_id = Some(id);
            execution.evidence = journal.verification.clone();
            Ok(execution)
        }
        Err(error) => {
            let mut execution = rollback(root, backup_root, &tx_dir, journal)?;
            execution
                .evidence
                .push(format!("transaction stopped: {error}"));
            Ok(execution)
        }
    }
}

fn initial_journal(
    root: &Path,
    plan: &MigrationPlan,
    id: String,
    created_at: String,
) -> Result<MigrationJournal, PortError> {
    Ok(MigrationJournal {
        schema_version: JOURNAL_SCHEMA_VERSION,
        transaction_id: id,
        repository: plan.repository.clone(),
        repository_identity: repository_identity(root)?,
        created_at,
        state: JournalState::InProgress,
        phase: JournalPhase::Inventoried,
        run_key: plan.run_key.clone(),
        legacy_roots: plan.legacy_roots.clone(),
        inventory: plan
            .inventory
            .iter()
            .map(|entry| JournalInventory {
                path: entry.path.clone(),
                kind: entry.kind.as_str().to_owned(),
                size: entry.size,
                hash: entry.hash.as_ref().map(|hash| hash.as_str().to_owned()),
            })
            .collect(),
        integration: Vec::new(),
        operations: Vec::new(),
        created_paths: Vec::new(),
        backup_relative: "backup".to_owned(),
        stage_relative: "stage".to_owned(),
        verification: Vec::new(),
    })
}

/// Revalidate the frozen plan against the repository after the lock is held,
/// so a file that changed between preview and apply blocks before mutation
/// (EXC-15, REQ-003).
fn revalidate(root: &Path, plan: &MigrationPlan) -> Result<Vec<String>, PortError> {
    let mut conflicts = Vec::new();
    for legacy in &plan.legacy_roots {
        if fs::symlink_metadata(root.join(legacy)).is_err() {
            conflicts.push(format!("{legacy} disappeared between plan and apply"));
        }
    }
    for entry in &plan.inventory {
        if entry.kind != InventoryKind::File {
            continue;
        }
        let current = hash_string(&root.join(&entry.path))?;
        let expected = entry.hash.as_ref().map(|hash| hash.as_str().to_owned());
        if current != expected {
            conflicts.push(format!("{} changed between plan and apply", entry.path));
        }
    }
    for operation in &plan.operations {
        let current = hash_string(&root.join(&operation.destination))?;
        let consistent = match operation.kind {
            OperationKind::Create => current.is_none(),
            OperationKind::Preserve => current.as_deref() == Some(operation.after_hash.as_str()),
            OperationKind::Replace => current.as_deref() != Some(operation.after_hash.as_str()),
        };
        if !consistent {
            conflicts.push(format!(
                "{} changed between plan and apply",
                operation.destination
            ));
        }
    }
    for integration in &plan.integration {
        let current = hash_string(&root.join(&integration.path))?;
        let expected = integration
            .before_hash
            .as_ref()
            .map(|hash| hash.as_str().to_owned());
        if current != expected {
            conflicts.push(format!(
                "{} changed between plan and apply",
                integration.path
            ));
        }
    }
    Ok(conflicts)
}

fn run_transaction(
    root: &Path,
    tx_dir: &Path,
    plan: &MigrationPlan,
    journal: &mut MigrationJournal,
) -> Result<(), PortError> {
    let id = journal.transaction_id.clone();
    let backup_dir = tx_dir.join(&journal.backup_relative);
    let stage_dir = tx_dir.join(&journal.stage_relative);

    // Verified backup of every legacy original before any destructive step.
    for entry in &plan.inventory {
        if entry.kind != InventoryKind::File {
            continue;
        }
        let source = root.join(&entry.path);
        let bytes = fs::read(&source).map_err(io_error)?;
        if Some(hash_bytes(&bytes)?) != entry.hash {
            return Err(PortError::new(format!(
                "{} changed while backing up",
                entry.path
            )));
        }
        let target = backup_dir.join(&entry.path);
        copy_bytes_atomic(&bytes, &target, &id)?;
        let copied = fs::read(&target).map_err(io_error)?;
        if Some(hash_bytes(&copied)?) != entry.hash {
            return Err(PortError::new(format!(
                "backup verification failed for {}",
                entry.path
            )));
        }
    }
    for integration in &plan.integration {
        if integration.before_hash.is_none() {
            continue;
        }
        let bytes = fs::read(root.join(&integration.path)).map_err(io_error)?;
        copy_bytes_atomic(
            &bytes,
            &backup_dir.join("integration").join(&integration.path),
            &id,
        )?;
    }
    journal.phase = JournalPhase::BackupVerified;
    write_journal(tx_dir, journal)?;
    #[cfg(test)]
    faults::trigger(root, faults::Point::AfterBackup)?;

    // Stage every intended byte and verify it independently.
    for operation in &plan.operations {
        if operation.kind == OperationKind::Preserve {
            continue;
        }
        let bytes = intended_bytes(root, operation)?;
        if hash_bytes(&bytes)? != operation.after_hash {
            return Err(PortError::new(format!(
                "{} does not match its planned hash",
                operation.destination
            )));
        }
        let target = stage_dir.join(&operation.destination);
        copy_bytes_atomic(&bytes, &target, &id)?;
        if hash_bytes(&fs::read(&target).map_err(io_error)?)? != operation.after_hash {
            return Err(PortError::new(format!(
                "stage verification failed for {}",
                operation.destination
            )));
        }
    }
    for integration in &plan.integration {
        copy_bytes_atomic(
            &integration.content,
            &stage_dir.join("integration").join(&integration.path),
            &id,
        )?;
    }
    journal.phase = JournalPhase::StageVerified;
    write_journal(tx_dir, journal)?;
    #[cfg(test)]
    faults::trigger(root, faults::Point::AfterStage)?;

    // Publish the namespaces, one journaled path at a time.
    for (index, operation) in plan.operations.iter().enumerate() {
        if operation.kind == OperationKind::Preserve {
            continue;
        }
        let bytes = fs::read(stage_dir.join(&operation.destination)).map_err(io_error)?;
        let destination = root.join(&operation.destination);
        ensure_parent_dirs(root, &destination, &mut journal.created_paths)?;
        let existed = fs::symlink_metadata(&destination).is_ok();
        copy_bytes_atomic(&bytes, &destination, &id)?;
        journal.operations.push(JournalOperation {
            destination: operation.destination.clone(),
            source: operation.source.clone(),
            kind: operation.kind.as_str().to_owned(),
            before_hash: operation
                .before_hash
                .as_ref()
                .map(|hash| hash.as_str().to_owned()),
            after_hash: operation.after_hash.as_str().to_owned(),
            created: !existed,
        });
        write_journal(tx_dir, journal)?;
        if index == 0 {
            #[cfg(test)]
            faults::trigger(root, faults::Point::DuringPublish)?;
        }
    }
    journal.phase = JournalPhase::NamespacesPublished;
    write_journal(tx_dir, journal)?;
    #[cfg(test)]
    faults::trigger(root, faults::Point::AfterPublish)?;

    // Integration writes: managed entrypoint blocks and the ignore repair.
    for (index, integration) in plan.integration.iter().enumerate() {
        let destination = root.join(&integration.path);
        ensure_parent_dirs(root, &destination, &mut journal.created_paths)?;
        copy_bytes_atomic(&integration.content, &destination, &id)?;
        journal.integration.push(JournalIntegration {
            path: integration.path.clone(),
            kind: integration.kind.as_str().to_owned(),
            before_hash: integration
                .before_hash
                .as_ref()
                .map(|hash| hash.as_str().to_owned()),
            after_hash: integration.after_hash.as_str().to_owned(),
        });
        write_journal(tx_dir, journal)?;
        if index == 0 {
            #[cfg(test)]
            faults::trigger(root, faults::Point::DuringIntegration)?;
        }
    }
    journal.phase = JournalPhase::IntegrationPublished;
    write_journal(tx_dir, journal)?;
    #[cfg(test)]
    faults::trigger(root, faults::Point::AfterIntegration)?;

    // Retire the legacy roots only after every destination and integration
    // write has verified.
    for legacy in &plan.legacy_roots {
        let target = root.join(legacy);
        if fs::symlink_metadata(&target).is_ok() {
            fs::remove_dir_all(&target).map_err(io_error)?;
        }
    }
    journal.phase = JournalPhase::LegacyRetired;
    write_journal(tx_dir, journal)?;
    #[cfg(test)]
    faults::trigger(root, faults::Point::AfterRetire)?;

    #[cfg(test)]
    faults::trigger(root, faults::Point::BeforeFinalVerify)?;
    final_verify(root, plan)?;
    journal.phase = JournalPhase::FinalVerified;
    journal
        .verification
        .push("destination and integration hashes verified".to_owned());
    journal
        .verification
        .push("legacy roots retired and verified absent".to_owned());
    write_journal(tx_dir, journal)?;
    Ok(())
}

fn intended_bytes(root: &Path, operation: &MigrationOperation) -> Result<Vec<u8>, PortError> {
    match &operation.content {
        Some(bytes) => Ok(bytes.clone()),
        None => {
            let source = operation.source.as_ref().ok_or_else(|| {
                PortError::new("migration operation has neither content nor source")
            })?;
            read_bytes(root, source)
        }
    }
}

fn final_verify(root: &Path, plan: &MigrationPlan) -> Result<(), PortError> {
    for legacy in &plan.legacy_roots {
        if fs::symlink_metadata(root.join(legacy)).is_ok() {
            return Err(PortError::new(format!("{legacy} survived retirement")));
        }
    }
    for operation in &plan.operations {
        if operation.kind == OperationKind::Preserve {
            continue;
        }
        let current = hash_string(&root.join(&operation.destination))?;
        if current.as_deref() != Some(operation.after_hash.as_str()) {
            return Err(PortError::new(format!(
                "{} did not verify after publication",
                operation.destination
            )));
        }
    }
    for integration in &plan.integration {
        let current = hash_string(&root.join(&integration.path))?;
        if current.as_deref() != Some(integration.after_hash.as_str()) {
            return Err(PortError::new(format!(
                "{} did not verify after the integration write",
                integration.path
            )));
        }
    }
    verify_installation_clean(root)
}

/// The post-rewrite cleanliness check: every manifest member and every
/// `base/` baseline exists with its recorded hash, so a moved tree cannot
/// report clean while `manifest.json` and `base/` still name the old root
/// (REQ-012).
fn verify_installation_clean(root: &Path) -> Result<(), PortError> {
    let manifest_path = root.join(NEW_CORE_ROOT).join("manifest.json");
    if !manifest_path.is_file() {
        return Ok(());
    }
    let bytes = fs::read(&manifest_path).map_err(io_error)?;
    let manifest: StateManifest = serde_json::from_slice(&bytes)
        .map_err(|error| PortError::new(format!("{}: {error}", manifest_path.display())))?;
    for file in &manifest.files {
        // A mixed entrypoint file is expected to carry consumer prose outside
        // the managed block, so the migration verifies its baseline copy but
        // never demands that the workspace copy match the installed baseline.
        if !is_mixed_entrypoint(&file.path) {
            let workspace = root.join(&file.path);
            let content = fs::read(&workspace).map_err(|error| {
                PortError::new(format!("{} is missing after migration: {error}", file.path))
            })?;
            if hash_bytes(&content)?.as_str() != file.upstream_sha256 {
                return Err(PortError::new(format!(
                    "{} does not match its recorded baseline hash",
                    file.path
                )));
            }
        }
        let base = root.join(NEW_CORE_ROOT).join("base").join(&file.path);
        let content = fs::read(&base).map_err(|error| {
            PortError::new(format!(
                "{} is missing after migration: {error}",
                base.display()
            ))
        })?;
        if hash_bytes(&content)?.as_str() != file.upstream_sha256 {
            return Err(PortError::new(format!(
                "{} does not match its recorded baseline hash",
                base.display()
            )));
        }
    }
    Ok(())
}

fn is_mixed_entrypoint(path: &str) -> bool {
    ENTRYPOINTS.contains(&path)
}

// ---------------------------------------------------------------------------
// Rollback and recovery (D-12, REQ-029, REQ-030, REQ-033)
// ---------------------------------------------------------------------------

/// Roll the transaction back: restore every recorded original, remove only
/// transaction-created paths, fence post-crash edits, verify, and retain
/// evidence. A content fence leaves the transaction incomplete and reports
/// `recovery_required`; an unrestorable original reports `rollback_failed`
/// with no safety claim.
fn rollback(
    root: &Path,
    _backup_root: &Path,
    tx_dir: &Path,
    mut journal: MigrationJournal,
) -> Result<MigrationExecution, PortError> {
    let id = journal.transaction_id.clone();
    let backup_dir = tx_dir.join(&journal.backup_relative);
    let mut conflicts: Vec<String> = Vec::new();
    let mut io_failure: Option<String> = None;

    for operation in journal.operations.iter().rev() {
        let destination = root.join(&operation.destination);
        let current = hash_string(&destination)?;
        if operation.created {
            match current {
                None => {}
                Some(hash) if hash == operation.after_hash => {
                    if let Err(error) = fs::remove_file(&destination) {
                        io_failure = Some(error.to_string());
                    }
                }
                Some(_) => conflicts.push(format!(
                    "post-crash edit at {}; the operator's bytes are kept",
                    operation.destination
                )),
            }
        } else {
            match current {
                Some(hash) if hash == operation.after_hash => {
                    let source = operation
                        .source
                        .clone()
                        .unwrap_or_else(|| operation.destination.clone());
                    restore(&backup_dir.join(source), &destination, &id, &mut io_failure);
                }
                Some(_) => conflicts.push(format!(
                    "post-crash edit at {}; the operator's bytes are kept",
                    operation.destination
                )),
                None => conflicts.push(format!(
                    "{} is missing; refusing to guess its restored content",
                    operation.destination
                )),
            }
        }
    }

    for integration in journal.integration.iter().rev() {
        let path = root.join(&integration.path);
        let current = hash_string(&path)?;
        match &integration.before_hash {
            None => match current {
                None => {}
                Some(hash) if hash == integration.after_hash => {
                    let _ = fs::remove_file(&path);
                }
                Some(_) => conflicts.push(format!(
                    "post-crash edit at {}; the operator's bytes are kept",
                    integration.path
                )),
            },
            Some(before) => match current {
                Some(hash) if hash == integration.after_hash => {
                    restore(
                        &backup_dir.join("integration").join(&integration.path),
                        &path,
                        &id,
                        &mut io_failure,
                    );
                }
                Some(hash) if &hash == before => {}
                Some(_) => conflicts.push(format!(
                    "post-crash edit at {}; the operator's bytes are kept",
                    integration.path
                )),
                None => conflicts.push(format!(
                    "{} is missing; refusing to guess its restored content",
                    integration.path
                )),
            },
        }
    }

    let mut created = journal.created_paths.clone();
    created.sort_by_key(|path| std::cmp::Reverse(path.len()));
    for relative in created {
        let directory = root.join(&relative);
        if fs::symlink_metadata(&directory).is_ok() {
            let _ = fs::remove_dir(&directory);
        }
    }

    for entry in &journal.inventory {
        if entry.kind == InventoryKind::Directory.as_str() {
            let _ = fs::create_dir_all(root.join(&entry.path));
        }
    }
    for entry in &journal.inventory {
        if entry.kind != InventoryKind::File.as_str() {
            continue;
        }
        let backup = backup_dir.join(&entry.path);
        match fs::read(&backup) {
            Ok(bytes) => {
                if let Err(error) = copy_bytes_atomic(&bytes, &root.join(&entry.path), &id) {
                    io_failure = Some(error.to_string());
                }
            }
            Err(error) => {
                io_failure = Some(format!(
                    "backup {} is unreadable: {error}",
                    backup.display()
                ));
            }
        }
    }

    let mut verified = io_failure.is_none() && conflicts.is_empty();
    if verified {
        for entry in &journal.inventory {
            if entry.kind != InventoryKind::File.as_str() {
                continue;
            }
            if hash_string(&root.join(&entry.path))?.as_deref() != entry.hash.as_deref() {
                verified = false;
            }
        }
        for integration in &journal.integration {
            if hash_string(&root.join(&integration.path))? != integration.before_hash {
                verified = false;
            }
        }
        for operation in &journal.operations {
            if operation.created && fs::symlink_metadata(root.join(&operation.destination)).is_ok()
            {
                verified = false;
            }
        }
        for legacy in &journal.legacy_roots {
            if fs::symlink_metadata(root.join(legacy)).is_err() {
                verified = false;
            }
        }
    }

    let journal_path = tx_dir.join(JOURNAL_FILE).display().to_string();
    let mut execution = if io_failure.is_some() || (conflicts.is_empty() && !verified) {
        journal.state = JournalState::RollbackFailed;
        MigrationExecution::new(MigrationState::RollbackFailed)
    } else if !conflicts.is_empty() {
        MigrationExecution::new(MigrationState::RecoveryRequired)
    } else {
        journal.state = JournalState::RolledBack;
        MigrationExecution::new(MigrationState::RolledBack)
    };
    write_journal(tx_dir, &journal)?;
    execution.transaction_id = Some(journal.transaction_id.clone());
    execution.conflicts = conflicts;
    execution.backup_path = Some(format!("{}/", tx_dir.display()));
    execution
        .evidence
        .push(format!("backup retained at {}", tx_dir.display()));
    execution
        .evidence
        .push(format!("journal retained at {journal_path}"));
    if let Some(error) = io_failure {
        execution.evidence.push(format!("restore error: {error}"));
    }
    Ok(execution)
}

fn restore(backup: &Path, destination: &Path, id: &str, failure: &mut Option<String>) {
    match fs::read(backup) {
        Ok(bytes) => {
            if let Err(error) = copy_bytes_atomic(&bytes, destination, id) {
                *failure = Some(error.to_string());
            }
        }
        Err(error) => {
            *failure = Some(format!(
                "backup {} is unreadable: {error}",
                backup.display()
            ));
        }
    }
}

fn recover(root: &Path) -> Result<MigrationExecution, PortError> {
    let backup_root = root.join(BACKUP_ROOT);
    let scan = scan_journals(root)?;
    if let Some(reason) = scan.blocked {
        let mut execution = MigrationExecution::new(MigrationState::Blocked);
        execution.reason = Some(reason);
        execution.conflicts = scan.evidence;
        return Ok(execution);
    }
    match scan.incomplete.len() {
        0 => Ok(MigrationExecution::new(MigrationState::RolledBack)),
        1 => {
            let found = scan.incomplete.into_iter().next().expect("one journal");
            rollback(root, &backup_root, &found.tx_dir, found.journal)
        }
        _ => {
            let mut execution = MigrationExecution::new(MigrationState::Blocked);
            execution.reason = Some(MigrationReason::MultipleJournals);
            Ok(execution)
        }
    }
}

/// Test-only deterministic failure hook, mirroring the proven
/// [`transaction`](super::transaction) pattern: no production surface, no
/// environment variable, and no dependency. Arming is keyed by the exact
/// repository root so parallel tests in one process cannot interfere.
#[cfg(test)]
mod faults {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use crate::application::PortError;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(super) enum Point {
        AfterBackup,
        AfterStage,
        DuringPublish,
        AfterPublish,
        DuringIntegration,
        AfterIntegration,
        AfterRetire,
        BeforeFinalVerify,
    }

    static ARMED: Mutex<Vec<(PathBuf, Point)>> = Mutex::new(Vec::new());

    pub(super) fn arm(root: &Path, point: Point) {
        let mut armed = ARMED.lock().unwrap_or_else(|error| error.into_inner());
        armed.retain(|(armed_root, _)| armed_root != root);
        armed.push((root.to_path_buf(), point));
    }

    pub(super) fn disarm(root: &Path) {
        let mut armed = ARMED.lock().unwrap_or_else(|error| error.into_inner());
        armed.retain(|(armed_root, _)| armed_root != root);
    }

    pub(super) fn trigger(root: &Path, point: Point) -> Result<(), PortError> {
        let mut armed = ARMED.lock().unwrap_or_else(|error| error.into_inner());
        match armed
            .iter()
            .position(|(armed_root, armed_point)| armed_root == root && *armed_point == point)
        {
            Some(index) => {
                armed.remove(index);
                Err(PortError::new(format!(
                    "injected migration failure at {point:?}"
                )))
            }
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::application::{
        CanonicalEntrypointsPort, CoreDistributionPort, MigrationApplication, MigrationInspection,
    };
    use crate::domain::backup_template;
    use crate::infrastructure::EmbeddedCoreDistribution;
    use sha2::{Digest, Sha256};

    fn write_file(root: &Path, relative: &str, content: &[u8]) {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, content).unwrap();
    }

    /// A complete legacy installation built from the embedded distribution,
    /// with every managed destination rewritten to the pre-0008 root.
    fn legacy_repository() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let distribution = EmbeddedCoreDistribution.current().unwrap();
        let mut files = Vec::new();
        for file in &distribution.files {
            let legacy = file.path.as_str().replace(".truss/core/", ".truss-core/");
            write_file(root, &legacy, &file.content);
            write_file(root, &format!(".truss-core/base/{legacy}"), &file.content);
            files.push(serde_json::json!({
                "path": legacy,
                "upstream_sha256": file.hash.as_str(),
            }));
        }
        let manifest = serde_json::json!({
            "schema_version": 1,
            "core_version": distribution.version,
            "files": files,
        });
        write_file(
            root,
            ".truss-core/manifest.json",
            &serde_json::to_vec_pretty(&manifest).unwrap(),
        );
        write_file(
            root,
            ".truss-core/.gitignore",
            b"/lock\n/transaction.json\n/base.next-*\n/update/\n/update-candidate/\n/addon-update/\n",
        );
        write_file(root, ".truss-core/lock", b"");
        tmp
    }

    fn blocks() -> CanonicalBlocks {
        EmbeddedCoreDistribution.blocks().unwrap()
    }

    fn inspect_fixture(root: &Path) -> MigrationInspection {
        inspect(root, &blocks()).unwrap()
    }

    fn plan_for(root: &Path) -> MigrationPlan {
        crate::application::plan_from(&inspect_fixture(root))
    }

    fn snapshot(root: &Path) -> Vec<String> {
        let mut lines = Vec::new();
        walk(root, root, &mut lines);
        lines.sort();
        lines
    }

    fn walk(root: &Path, directory: &Path, lines: &mut Vec<String>) {
        let mut entries = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = posix(path.strip_prefix(root).unwrap());
            if relative == BACKUP_ROOT || relative.starts_with(&format!("{BACKUP_ROOT}/")) {
                continue;
            }
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.file_type().is_symlink() {
                lines.push(format!("symlink {relative}"));
            } else if metadata.is_dir() {
                lines.push(format!("dir {relative}"));
                walk(root, &path, lines);
            } else if metadata.is_file() {
                let bytes = fs::read(&path).unwrap();
                lines.push(format!("file {relative} {:x}", Sha256::digest(&bytes)));
            }
        }
    }

    fn run(root: &Path, point: faults::Point) -> MigrationExecution {
        let plan = plan_for(root);
        faults::arm(root, point);
        let execution = FileSystemMigration.execute(root, &plan).unwrap();
        faults::disarm(root);
        execution
    }

    fn list_files(root: &Path) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        collect(root, root, &mut found);
        found
    }

    fn collect(root: &Path, directory: &Path, found: &mut BTreeSet<String>) {
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let relative = posix(path.strip_prefix(root).unwrap());
            if entry.file_type().unwrap().is_dir() {
                collect(root, &path, found);
            } else {
                found.insert(relative);
            }
        }
    }

    /// REQ-002 / REQ-037: a preview performs absolutely no mutation.
    #[test]
    fn preview_performs_absolutely_no_mutation() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let before = snapshot(root);
        let application = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution);
        let text = application.preview(root).unwrap();
        assert_eq!(text.state, MigrationState::Ready);
        assert!(text.backup_path.is_none(), "preview allocates no path");
        let json = application.preview(root).unwrap();
        assert_eq!(json.state, MigrationState::Ready);
        assert_eq!(snapshot(root), before, "a preview mutates nothing");
        assert!(
            !root.join(BACKUP_ROOT).exists(),
            "preview must not create the backup root or its lock"
        );
    }

    /// REQ-001 / REQ-037: the happy path publishes, retires, and is clean.
    #[test]
    fn apply_migrates_publishes_and_retires_legacy_roots() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let application = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution);
        let report = application.apply(root).unwrap();
        assert_eq!(report.state, MigrationState::Migrated);
        assert!(report.applied);
        assert!(root.join(".truss/core/manifest.json").is_file());
        assert!(
            !root.join(LEGACY_CORE_ROOT).exists(),
            "the legacy core root must be retired"
        );
        // The second invocation is idempotent and allocates no new backup.
        let backups = backup_directories(root);
        let again = application.apply(root).unwrap();
        assert_eq!(again.state, MigrationState::AlreadyMigrated);
        assert_eq!(backup_directories(root), backups);
    }

    fn backup_directories(root: &Path) -> Vec<String> {
        let mut found = Vec::new();
        if let Ok(entries) = fs::read_dir(root.join(BACKUP_ROOT)) {
            for entry in entries.flatten() {
                if entry.file_type().unwrap().is_dir() {
                    found.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
        found.sort();
        found
    }

    /// REQ-029 / REQ-037: an injected failure at every durable boundary
    /// restores the repository byte-identically.
    #[test]
    fn injected_failure_at_every_boundary_rolls_back_byte_identically() {
        for point in [
            faults::Point::AfterBackup,
            faults::Point::AfterStage,
            faults::Point::DuringPublish,
            faults::Point::AfterPublish,
            faults::Point::DuringIntegration,
            faults::Point::AfterIntegration,
            faults::Point::AfterRetire,
            faults::Point::BeforeFinalVerify,
        ] {
            let fixture = legacy_repository();
            let root = fixture.path();
            let before = snapshot(root);
            let execution = run(root, point);
            assert_eq!(
                execution.state,
                MigrationState::RolledBack,
                "{point:?} must roll back"
            );
            assert_eq!(snapshot(root), before, "{point:?} must restore every byte");
        }
    }

    /// REQ-030: an unrestorable original reports `rollback_failed` and never
    /// claims safety.
    #[test]
    fn unrestorable_original_reports_rollback_failed_without_a_safety_claim() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let tx_dir = root.join(BACKUP_ROOT).join("20260101T000000.000000000Z");
        fs::create_dir_all(tx_dir.join("backup")).unwrap();
        let after = hash_bytes(b"new").unwrap();
        write_file(root, ".truss/core/x.md", b"new");
        let journal = MigrationJournal {
            schema_version: JOURNAL_SCHEMA_VERSION,
            transaction_id: "tx".to_owned(),
            repository: repository_name(root).unwrap(),
            repository_identity: repository_identity(root).unwrap(),
            created_at: "20260101T000000.000000000Z".to_owned(),
            state: JournalState::InProgress,
            phase: JournalPhase::NamespacesPublished,
            run_key: None,
            legacy_roots: vec![LEGACY_CORE_ROOT.to_owned()],
            inventory: Vec::new(),
            integration: Vec::new(),
            operations: vec![JournalOperation {
                destination: ".truss/core/x.md".to_owned(),
                source: Some(".truss-core/x.md".to_owned()),
                kind: "create".to_owned(),
                before_hash: Some(after.as_str().to_owned()),
                after_hash: after.as_str().to_owned(),
                created: false,
            }],
            created_paths: Vec::new(),
            backup_relative: "backup".to_owned(),
            stage_relative: "stage".to_owned(),
            verification: Vec::new(),
        };
        write_journal(&tx_dir, &journal).unwrap();
        let execution = rollback(root, &root.join(BACKUP_ROOT), &tx_dir, journal).unwrap();
        assert_eq!(execution.state, MigrationState::RollbackFailed);
        let text = execution.evidence.join("\n").to_lowercase();
        assert!(!text.contains("safe"), "no safety claim: {text}");
        assert!(!text.contains("restored"), "no restoration claim: {text}");
        assert!(text.contains("backup retained"));
        assert!(text.contains("journal retained"));
    }

    /// REQ-031 / REQ-037: the committed journal records identity, phase,
    /// inventory, and every published path.
    #[test]
    fn committed_journal_records_identity_inventory_and_published_paths() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let plan = plan_for(root);
        let execution = FileSystemMigration.execute(root, &plan).unwrap();
        assert_eq!(execution.state, MigrationState::Migrated);
        let tx_dir = PathBuf::from(
            execution
                .backup_path
                .as_ref()
                .unwrap()
                .trim_end_matches('/'),
        );
        let journal: MigrationJournal =
            serde_json::from_slice(&fs::read(tx_dir.join(JOURNAL_FILE)).unwrap()).unwrap();
        assert_eq!(journal.state, JournalState::Committed);
        assert_eq!(journal.phase, JournalPhase::Committed);
        assert_eq!(
            journal.repository_identity,
            repository_identity(root).unwrap()
        );
        assert!(!journal.transaction_id.is_empty());
        assert!(!journal.created_at.is_empty());
        assert!(!journal.inventory.is_empty());
        let published: BTreeSet<&str> = journal
            .operations
            .iter()
            .map(|operation| operation.destination.as_str())
            .collect();
        let planned: BTreeSet<&str> = plan
            .operations
            .iter()
            .filter(|operation| operation.kind != OperationKind::Preserve)
            .map(|operation| operation.destination.as_str())
            .collect();
        assert_eq!(published, planned, "every published path is journaled");
    }

    /// REQ-025: the backup contains exactly the source population plus the
    /// changed integration originals, and never itself.
    #[test]
    fn backup_equals_source_and_integration_population() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let plan = plan_for(root);
        let execution = FileSystemMigration.execute(root, &plan).unwrap();
        let tx_dir = PathBuf::from(
            execution
                .backup_path
                .as_ref()
                .unwrap()
                .trim_end_matches('/'),
        );
        let backed_up = list_files(&tx_dir.join("backup"));
        let mut expected: BTreeSet<String> = plan
            .inventory
            .iter()
            .filter(|entry| entry.kind == InventoryKind::File)
            .map(|entry| entry.path.clone())
            .collect();
        for integration in &plan.integration {
            if integration.before_hash.is_some() {
                expected.insert(format!("integration/{}", integration.path));
            }
        }
        assert_eq!(backed_up, expected);
        assert!(
            !backed_up.contains("migration.json"),
            "the journal is excluded from its own backup"
        );
    }

    /// REQ-012 / REQ-015: the record rewrite moves the managed destinations and
    /// keeps every recorded hash, so `status` reports clean.
    #[test]
    fn manifest_rewrite_moves_destinations_and_keeps_hashes() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let legacy: StateManifest = serde_json::from_slice(
            &fs::read(root.join(LEGACY_CORE_ROOT).join("manifest.json")).unwrap(),
        )
        .unwrap();
        let application = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution);
        assert_eq!(
            application.apply(root).unwrap().state,
            MigrationState::Migrated
        );
        let rewritten: StateManifest = serde_json::from_slice(
            &fs::read(root.join(NEW_CORE_ROOT).join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(rewritten.schema_version, legacy.schema_version);
        assert_eq!(rewritten.core_version, legacy.core_version);
        assert_eq!(rewritten.files.len(), legacy.files.len());
        for (before, after) in legacy.files.iter().zip(rewritten.files.iter()) {
            assert_eq!(after.path, core_destination(&before.path));
            assert_eq!(
                after.upstream_sha256, before.upstream_sha256,
                "a path rewrite must never change a recorded hash"
            );
        }
        let moved = rewritten
            .files
            .iter()
            .filter(|file| file.path.starts_with(".truss/core/docs/"))
            .count();
        assert_eq!(moved, 13, "the 13 core destinations move under .truss/core");
        // Managed READMEs stay managed and never become project authority.
        assert!(root.join(".truss/core/docs/decisions/README.md").is_file());
        assert!(!root.join(".truss/authority/decisions/README.md").exists());
    }

    /// REQ-013 / REQ-014: recognized unmanaged documents move; an unknown one
    /// blocks with its path named and nothing written.
    #[test]
    fn authority_documents_move_and_an_unknown_document_blocks() {
        let fixture = legacy_repository();
        let root = fixture.path();
        write_file(root, ".truss-core/TRUSS.md", b"product\n");
        write_file(root, ".truss-core/ARCHITECTURE.md", b"arch\n");
        write_file(
            root,
            ".truss-core/docs/product/installation-profiles.md",
            b"p\n",
        );
        let application = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution);
        assert_eq!(
            application.apply(root).unwrap().state,
            MigrationState::Migrated
        );
        for path in [
            ".truss/authority/TRUSS.md",
            ".truss/authority/ARCHITECTURE.md",
            ".truss/authority/product/installation-profiles.md",
        ] {
            assert!(root.join(path).is_file(), "{path} must move to authority");
        }

        let fixture = legacy_repository();
        let root = fixture.path();
        write_file(root, ".truss-core/docs/stray.md", b"unknown\n");
        let before = snapshot(root);
        let report = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution)
            .preview(root)
            .unwrap();
        assert_eq!(report.state, MigrationState::Blocked);
        assert_eq!(report.reason, Some(MigrationReason::UnknownDocument));
        assert!(report
            .conflicts
            .iter()
            .any(|line| line.contains("stray.md")));
        assert_eq!(snapshot(root), before);
    }

    /// REQ-017 / REQ-018 / REQ-019: the narrow run-key predicate accepts one
    /// shared direct child and refuses every ambiguous shape.
    #[test]
    fn run_key_recognition_is_narrow() {
        let fixture = legacy_repository();
        let root = fixture.path();
        write_file(root, ".delivery/run-a/plan.md", b"plan\n");
        write_file(root, ".delivery-dispatch/run-a/prompt.md", b"prompt\n");
        let application = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution);
        assert_eq!(
            application.apply(root).unwrap().state,
            MigrationState::Migrated
        );
        assert!(root
            .join(".truss/delivery/runs/run-a/evidence/plan.md")
            .is_file());
        assert!(root
            .join(".truss/delivery/runs/run-a/dispatch/prompt.md")
            .is_file());

        for shape in ["two-children", "flat", "mismatched"] {
            let fixture = legacy_repository();
            let root = fixture.path();
            match shape {
                "two-children" => {
                    write_file(root, ".delivery/run-a/plan.md", b"a\n");
                    write_file(root, ".delivery/run-b/plan.md", b"b\n");
                }
                "flat" => write_file(root, ".delivery/plan.md", b"flat\n"),
                _ => {
                    write_file(root, ".delivery/run-a/plan.md", b"a\n");
                    write_file(root, ".delivery-dispatch/run-b/p.md", b"b\n");
                }
            }
            let before = snapshot(root);
            let report = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution)
                .preview(root)
                .unwrap();
            assert_eq!(report.state, MigrationState::Blocked, "{shape}");
            assert_eq!(
                report.reason,
                Some(MigrationReason::AmbiguousRunKey),
                "{shape}"
            );
            assert!(!report.conflicts.is_empty(), "{shape} lists candidates");
            assert_eq!(snapshot(root), before, "{shape}");
        }
    }

    /// REQ-020: a byte-different destination blocks before mutation and is
    /// never overwritten.
    #[test]
    fn different_byte_collision_blocks_before_mutation() {
        let fixture = legacy_repository();
        let root = fixture.path();
        write_file(root, ".truss/core/docs/WORKFLOW.md", b"operator bytes\n");
        let before = snapshot(root);
        let report = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution)
            .preview(root)
            .unwrap();
        assert_eq!(report.state, MigrationState::Blocked);
        assert_eq!(report.reason, Some(MigrationReason::Collision));
        assert_eq!(
            fs::read(root.join(".truss/core/docs/WORKFLOW.md")).unwrap(),
            b"operator bytes\n"
        );
        assert_eq!(snapshot(root), before);
    }

    /// REQ-022: a markerless entrypoint is a refusal, never a repair.
    #[test]
    fn invalid_entrypoint_markers_block_without_writing() {
        for content in [
            &b"no markers\n"[..],
            &b"<!-- TRUSS:BEGIN -->\n<!-- TRUSS:BEGIN -->\n<!-- TRUSS:END -->\n"[..],
            &b"<!-- TRUSS:END -->\n<!-- TRUSS:BEGIN -->\n"[..],
        ] {
            let fixture = legacy_repository();
            let root = fixture.path();
            write_file(root, "AGENTS.md", content);
            let before = snapshot(root);
            let report = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution)
                .preview(root)
                .unwrap();
            assert_eq!(report.state, MigrationState::Blocked);
            assert_eq!(report.reason, Some(MigrationReason::InvalidMarkers));
            assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), content);
            assert_eq!(snapshot(root), before);
        }
    }

    /// REQ-011: a moved file is byte-equal to its source.
    #[test]
    fn moved_files_are_byte_equal_to_their_sources() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let sources: Vec<(String, Vec<u8>)> = list_files(&root.join(LEGACY_CORE_ROOT))
            .into_iter()
            .map(|relative| {
                let bytes = fs::read(root.join(LEGACY_CORE_ROOT).join(&relative)).unwrap();
                (relative, bytes)
            })
            .collect();
        let application = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution);
        assert_eq!(
            application.apply(root).unwrap().state,
            MigrationState::Migrated
        );
        for (relative, bytes) in sources {
            let destination =
                root.join(core_destination(&format!("{LEGACY_CORE_ROOT}/{relative}")));
            if relative.ends_with("manifest.json") {
                continue;
            }
            assert_eq!(
                fs::read(&destination).unwrap(),
                bytes,
                "{} must be byte-equal",
                destination.display()
            );
        }
    }

    /// REQ-032: a crashed transaction makes preview recovery_required and
    /// apply rolls back, then migrates from the restored tree.
    #[test]
    fn crash_recovery_rolls_back_then_migrates_fresh() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let before = snapshot(root);
        crash_after(root, faults::Point::AfterPublish);
        let report = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution)
            .preview(root)
            .unwrap();
        assert_eq!(report.state, MigrationState::RecoveryRequired);
        let applied = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution)
            .apply(root)
            .unwrap();
        assert_eq!(applied.state, MigrationState::Migrated);
        assert!(applied
            .evidence
            .iter()
            .any(|line| line.contains("rolled back") || line.contains("backup retained")));
        assert_ne!(snapshot(root), before, "the repository migrated");
        assert!(root.join(".truss/core/manifest.json").is_file());
    }

    fn crash_after(root: &Path, point: faults::Point) {
        let plan = plan_for(root);
        let id = transaction_id().unwrap();
        let created_at = "20260101T000000.000000000Z".to_owned();
        let tx_dir = unique_tx_dir(&root.join(BACKUP_ROOT), &created_at).unwrap();
        fs::create_dir_all(&tx_dir).unwrap();
        let mut journal = initial_journal(root, &plan, id, created_at).unwrap();
        write_journal(&tx_dir, &journal).unwrap();
        faults::arm(root, point);
        let result = run_transaction(root, &tx_dir, &plan, &mut journal);
        faults::disarm(root);
        assert!(result.is_err(), "the injected fault stops the transaction");
    }

    /// REQ-033: a post-crash operator edit is never overwritten and is
    /// reported as a recovery conflict.
    #[test]
    fn post_crash_edit_is_kept_and_reported() {
        let fixture = legacy_repository();
        let root = fixture.path();
        crash_after(root, faults::Point::AfterPublish);
        let edited = root.join(".truss/core/docs/WORKFLOW.md");
        assert!(edited.is_file(), "the crash published the destination");
        fs::write(&edited, b"operator edited after the crash\n").unwrap();
        let report = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution)
            .apply(root)
            .unwrap();
        assert_eq!(report.state, MigrationState::RecoveryRequired);
        assert_eq!(
            fs::read(&edited).unwrap(),
            b"operator edited after the crash\n",
            "the operator's bytes must survive"
        );
        assert!(report
            .conflicts
            .iter()
            .any(|line| line.contains("post-crash edit")));
    }

    /// REQ-006 / REQ-007: a new-only repository is already migrated and an
    /// empty repository is a named refusal, not a success.
    #[test]
    fn already_migrated_and_not_installed_are_distinguished() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let application = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution);
        assert_eq!(
            application.apply(root).unwrap().state,
            MigrationState::Migrated
        );
        let again = application.preview(root).unwrap();
        assert_eq!(again.state, MigrationState::AlreadyMigrated);
        assert_eq!(again.state.exit_code(), 0);
        assert!(again.backup_path.is_some(), "the existing backup is named");

        let empty = tempfile::tempdir().unwrap();
        let report = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution)
            .preview(empty.path())
            .unwrap();
        assert_eq!(report.state, MigrationState::Blocked);
        assert_eq!(report.reason, Some(MigrationReason::NotInstalled));
        assert_eq!(report.state.exit_code(), 1);
        assert!(!empty.path().join(BACKUP_ROOT).exists());
    }

    /// REQ-010: a pending session and an unsafe symlink refuse before mutation.
    #[test]
    fn pending_session_and_symlink_block_before_mutation() {
        let fixture = legacy_repository();
        let root = fixture.path();
        write_file(root, ".truss-core/transaction.json", b"{}");
        let before = snapshot(root);
        let report = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution)
            .preview(root)
            .unwrap();
        assert_eq!(report.reason, Some(MigrationReason::PendingSession));
        assert_eq!(snapshot(root), before);

        let fixture = legacy_repository();
        let root = fixture.path();
        std::os::unix::fs::symlink(
            root.join(".truss-core/docs/README.md"),
            root.join(".truss-core/docs/evil.md"),
        )
        .unwrap();
        let before = snapshot(root);
        let report = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution)
            .preview(root)
            .unwrap();
        assert_eq!(report.reason, Some(MigrationReason::UnsafeSymlink));
        assert_eq!(snapshot(root), before);
    }

    /// D-07: preview prints a template with no concrete timestamp, and apply
    /// prints the exact created path.
    #[test]
    fn preview_prints_a_template_and_apply_prints_the_created_path() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let application = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution);
        let preview = application.preview(root).unwrap();
        assert_eq!(
            preview.backup_path_template,
            backup_template(&repository_name(root).unwrap())
        );
        assert!(preview.backup_path_template.contains("<UTC-timestamp>"));
        assert!(preview.backup_path.is_none());
        let applied = application.apply(root).unwrap();
        let path = applied.backup_path.unwrap();
        assert!(!path.contains("<UTC-timestamp>"));
        assert!(Path::new(path.trim_end_matches('/')).is_dir());
    }

    /// REQ-003 / REQ-010 (EXC-15): a plan invalidated between preflight and
    /// execution blocks with no repository mutation.
    #[test]
    fn a_plan_invalidated_after_preflight_blocks_without_mutation() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let plan = plan_for(root);
        write_file(
            root,
            ".truss-core/docs/WORKFLOW.md",
            b"edited after the preview\n",
        );
        let before = snapshot(root);
        let execution = FileSystemMigration.execute(root, &plan).unwrap();
        assert_eq!(execution.state, MigrationState::Blocked);
        assert_eq!(execution.reason, Some(MigrationReason::StaleInventory));
        assert!(!execution.conflicts.is_empty());
        assert_eq!(snapshot(root), before, "the stale plan mutates nothing");
    }

    /// REQ-028: the migration merges; it never replaces a namespace wholesale.
    #[test]
    fn pre_existing_namespace_contents_are_preserved() {
        let fixture = legacy_repository();
        let root = fixture.path();
        write_file(root, ".truss/authority/notes.md", b"mine\n");
        write_file(root, ".truss/delivery/keep.md", b"keep\n");
        let application = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution);
        assert_eq!(
            application.apply(root).unwrap().state,
            MigrationState::Migrated
        );
        assert_eq!(
            fs::read(root.join(".truss/authority/notes.md")).unwrap(),
            b"mine\n"
        );
        assert_eq!(
            fs::read(root.join(".truss/delivery/keep.md")).unwrap(),
            b"keep\n"
        );
    }

    /// REQ-026: migration has its own journal and never parameterizes the core
    /// update transaction or its `.truss-backup/` path.
    #[test]
    fn migration_never_touches_the_core_update_transaction() {
        let fixture = legacy_repository();
        let root = fixture.path();
        let application = MigrationApplication::new(FileSystemMigration, EmbeddedCoreDistribution);
        assert_eq!(
            application.apply(root).unwrap().state,
            MigrationState::Migrated
        );
        assert!(!root.join(".truss-backup").exists());
        assert!(!root.join(NEW_CORE_ROOT).join("transaction.json").exists());
        assert!(root.join(BACKUP_ROOT).is_dir());
        assert!(root.join(BACKUP_ROOT).join(BACKUP_LOCK).is_file());
    }

    /// REQ-027: the journal records each durable boundary, so a crash leaves a
    /// phase that names exactly how far the transaction got.
    #[test]
    fn journal_phase_advances_to_the_crash_boundary() {
        let cases = [
            (faults::Point::AfterBackup, "backup_verified"),
            (faults::Point::AfterStage, "stage_verified"),
            (faults::Point::AfterPublish, "namespaces_published"),
            (faults::Point::AfterIntegration, "integration_published"),
            (faults::Point::AfterRetire, "legacy_retired"),
        ];
        for (point, expected) in cases {
            let fixture = legacy_repository();
            let root = fixture.path();
            crash_after(root, point);
            let scan = scan_journals(root).unwrap();
            assert_eq!(scan.incomplete.len(), 1, "{point:?}");
            assert_eq!(
                scan.incomplete[0].journal.phase.as_str(),
                expected,
                "{point:?} must journal the boundary it completed"
            );
        }
    }
}
