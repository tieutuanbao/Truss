//! Shared journal-and-backup transaction engine for `.truss-core/` writers.
//!
//! The core distribution state and the installed add-on record use one lock,
//! one journal, and one backup-and-restore sequence. The workspace-file half of
//! a transaction is generic here; the state half is supplied by a
//! [`ProvenanceWriter`] (what to snapshot, and the provenance write that must
//! run last) and is restored by [`recover`] from the [`ProvenanceKind`] the
//! journal recorded, so a caller never re-implements the mechanism.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::addon_state::{ADDONS_FILE, BASE_ADDONS_DIR};
use super::state_io::{
    copy_bytes_atomic, copy_file, copy_file_atomic, copy_tree, io_error, read_json,
    remove_dir_if_exists, remove_if_exists, transaction_id, validate_path, write_json_atomic,
};
use crate::application::PortError;
use crate::domain::{AddOnName, ApplyReceipt, RelativePath, WorkspaceMutation};

pub(crate) const JOURNAL_FILE: &str = "transaction.json";
const JOURNAL_SCHEMA_VERSION: u32 = 1;

/// Where a transaction's non-workspace state lives.
///
/// Recorded in the journal so [`recover`] can restore the right state surface
/// without the caller that started the transaction.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ProvenanceKind {
    #[default]
    Core,
    AddOn {
        name: String,
    },
}

impl ProvenanceKind {
    /// Stable label used in the backup directory name.
    fn label(&self) -> String {
        match self {
            Self::Core => "core".to_owned(),
            Self::AddOn { name } => format!("addon-{name}"),
        }
    }
}

/// The state half of one transaction: what exists now, how to snapshot it, and
/// the provenance write that runs after every workspace mutation.
pub(crate) trait ProvenanceWriter {
    fn kind(&self) -> ProvenanceKind;

    /// True when there is existing state to snapshot and restore.
    fn exists(&self) -> bool;

    /// Snapshot the existing state into `target`.
    fn backup(&self, target: &Path) -> Result<(), PortError>;

    /// Write the new provenance. Runs last, after every workspace mutation.
    fn write(&self, id: &str) -> Result<(), PortError>;
}

/// The restore half of one transaction. It is reconstructed from the journal
/// so recovery needs no live caller state.
trait ProvenanceRestore {
    fn remove_state(&self, state_root: &Path) -> Result<(), PortError>;
    fn restore_state(&self, state_root: &Path, backup: &Path, id: &str) -> Result<(), PortError>;
    fn discard_staging(&self, state_root: &Path, id: &str) -> Result<(), PortError>;
}

struct CoreRestore;

impl ProvenanceRestore for CoreRestore {
    fn remove_state(&self, state_root: &Path) -> Result<(), PortError> {
        remove_if_exists(&state_root.join("manifest.json"))?;
        remove_dir_if_exists(&state_root.join("base"))
    }

    fn restore_state(&self, state_root: &Path, backup: &Path, id: &str) -> Result<(), PortError> {
        if backup.join("manifest.json").exists() {
            copy_file_atomic(
                &backup.join("manifest.json"),
                &state_root.join("manifest.json"),
                id,
            )?;
        }
        if backup.join("base").exists() {
            copy_tree(&backup.join("base"), &state_root.join("base"))?;
        }
        Ok(())
    }

    fn discard_staging(&self, state_root: &Path, id: &str) -> Result<(), PortError> {
        remove_dir_if_exists(&state_root.join(format!("base.next-{id}")))
    }
}

struct AddOnRestore {
    name: AddOnName,
}

impl AddOnRestore {
    fn baseline(&self, state_root: &Path) -> PathBuf {
        state_root.join(BASE_ADDONS_DIR).join(self.name.as_str())
    }
}

impl ProvenanceRestore for AddOnRestore {
    fn remove_state(&self, state_root: &Path) -> Result<(), PortError> {
        remove_if_exists(&state_root.join(ADDONS_FILE))?;
        remove_dir_if_exists(&self.baseline(state_root))
    }

    fn restore_state(&self, state_root: &Path, backup: &Path, id: &str) -> Result<(), PortError> {
        let addons = backup.join(ADDONS_FILE);
        if addons.exists() {
            copy_file_atomic(&addons, &state_root.join(ADDONS_FILE), id)?;
        }
        let baseline = backup.join(BASE_ADDONS_DIR).join(self.name.as_str());
        if baseline.exists() {
            copy_tree(&baseline, &self.baseline(state_root))?;
        }
        Ok(())
    }

    fn discard_staging(&self, state_root: &Path, id: &str) -> Result<(), PortError> {
        remove_dir_if_exists(
            &state_root
                .join(BASE_ADDONS_DIR)
                .join(format!("{}.next-{id}", self.name)),
        )
    }
}

fn restore_for(kind: &ProvenanceKind) -> Result<Box<dyn ProvenanceRestore>, PortError> {
    Ok(match kind {
        ProvenanceKind::Core => Box::new(CoreRestore),
        ProvenanceKind::AddOn { name } => Box::new(AddOnRestore {
            name: AddOnName::parse(name.clone()).map_err(|error| {
                PortError::new(format!(
                    "transaction journal names an unsafe add-on: {error}"
                ))
            })?,
        }),
    })
}

/// Run one transaction: snapshot every mutated workspace file and the existing
/// provenance, apply the mutations, then write the new provenance last.
///
/// A failure at any point rolls the workspace and the provenance back to the
/// snapshot before the error is returned. The caller already holds the shared
/// `.truss-core/lock` and has already recovered any interrupted transaction.
pub(crate) fn run(
    root: &Path,
    state_root: &Path,
    mutations: &[WorkspaceMutation],
    provenance: &dyn ProvenanceWriter,
) -> Result<ApplyReceipt, PortError> {
    let id = transaction_id()?;
    let backup_relative = format!(".truss-backup/truss-{}-{id}", provenance.kind().label());
    let backup_root = root.join(&backup_relative);
    let state_existed = provenance.exists();
    let mut records = Vec::new();

    for mutation in mutations {
        let path = mutation.path();
        validate_path(root, path)?;
        let target = root.join(path.as_str());
        let existed = target.exists();
        if existed {
            let metadata = fs::symlink_metadata(&target).map_err(io_error)?;
            if !metadata.is_file() {
                return Err(PortError::new(format!(
                    "cannot back up non-file managed path: {path}"
                )));
            }
            let backup = backup_root.join("files").join(path.as_str());
            copy_file(&target, &backup)?;
        }
        records.push(JournalFile {
            path: path.as_str().to_owned(),
            existed,
        });
    }

    if state_existed {
        provenance.backup(&backup_root.join("state"))?;
    }

    let mut journal = TransactionJournal {
        schema_version: JOURNAL_SCHEMA_VERSION,
        id: id.clone(),
        phase: TransactionPhase::Applying,
        backup_relative: backup_relative.clone(),
        state_existed,
        provenance: provenance.kind(),
        files: records,
    };
    write_json_atomic(&state_root.join(JOURNAL_FILE), &journal, &id)?;

    let applied = (|| -> Result<(), PortError> {
        for (index, mutation) in mutations.iter().enumerate() {
            match mutation {
                WorkspaceMutation::Write { path, content } => {
                    write_workspace_atomic(root, path, content, &id)?;
                }
                WorkspaceMutation::Delete { path } => {
                    let target = root.join(path.as_str());
                    if target.exists() {
                        fs::remove_file(target).map_err(io_error)?;
                    }
                }
            }
            if index == 0 {
                #[cfg(test)]
                faults::trigger(root, faults::InjectionPoint::AfterFirstMutation)?;
            }
        }
        provenance.write(&id)
    })();

    if let Err(error) = applied {
        return match recover(root, state_root) {
            Ok(_) => Err(error),
            Err(recovery_error) => Err(PortError::new(format!(
                "{error}; automatic recovery also failed: {recovery_error}"
            ))),
        };
    }

    journal.phase = TransactionPhase::Committed;
    write_json_atomic(&state_root.join(JOURNAL_FILE), &journal, &id)?;
    fs::remove_file(state_root.join(JOURNAL_FILE)).map_err(io_error)?;

    let backup_has_content = state_existed
        || mutations.iter().any(|mutation| {
            backup_root
                .join("files")
                .join(mutation.path().as_str())
                .exists()
        });
    if !backup_has_content && backup_root.exists() {
        fs::remove_dir_all(&backup_root).map_err(io_error)?;
    }
    Ok(ApplyReceipt {
        backup_path: backup_has_content.then_some(backup_relative),
    })
}

/// Roll an interrupted transaction forward when it was committed, or back to
/// its snapshot when it was still applying.
///
/// Returns `true` when a journal was present and resolved. The caller holds the
/// shared lock.
pub(crate) fn recover(root: &Path, state_root: &Path) -> Result<bool, PortError> {
    let journal_path = state_root.join(JOURNAL_FILE);
    if !journal_path.exists() {
        return Ok(false);
    }
    let journal: TransactionJournal = read_json(&journal_path)?;
    if journal.schema_version != JOURNAL_SCHEMA_VERSION {
        return Err(PortError::new(format!(
            "unsupported transaction journal schema: {}",
            journal.schema_version
        )));
    }
    if journal.phase == TransactionPhase::Committed {
        fs::remove_file(journal_path).map_err(io_error)?;
        return Ok(true);
    }

    let provenance = restore_for(&journal.provenance)?;
    let backup_root = root.join(&journal.backup_relative);
    for record in &journal.files {
        let path = RelativePath::parse(record.path.clone())
            .map_err(|error| PortError::new(error.to_string()))?;
        validate_path(root, &path)?;
        let target = root.join(path.as_str());
        if record.existed {
            let backup = backup_root.join("files").join(path.as_str());
            if !backup.is_file() {
                return Err(PortError::new(format!(
                    "transaction backup is missing: {}",
                    backup.display()
                )));
            }
            copy_file_atomic(&backup, &target, &journal.id)?;
        } else if target.exists() {
            fs::remove_file(target).map_err(io_error)?;
        }
    }

    provenance.remove_state(state_root)?;
    if journal.state_existed {
        provenance.restore_state(state_root, &backup_root.join("state"), &journal.id)?;
    }
    provenance.discard_staging(state_root, &journal.id)?;
    // The transaction is fully rolled back, so its snapshot must not survive as
    // an orphan directory inside the workspace: a failed apply leaves all three
    // surfaces byte-identical to before the call.
    remove_dir_if_exists(&backup_root)?;
    if let Some(parent) = backup_root.parent() {
        // Remove `.truss-backup/` only when this rollback emptied it, so a
        // snapshot kept for another transaction is never touched.
        let _ = fs::remove_dir(parent);
    }
    fs::remove_file(journal_path).map_err(io_error)?;
    Ok(true)
}

fn write_workspace_atomic(
    root: &Path,
    path: &RelativePath,
    content: &[u8],
    id: &str,
) -> Result<(), PortError> {
    validate_path(root, path)?;
    let target = root.join(path.as_str());
    copy_bytes_atomic(content, &target, id)
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct TransactionJournal {
    pub(crate) schema_version: u32,
    pub(crate) id: String,
    pub(crate) phase: TransactionPhase,
    pub(crate) backup_relative: String,
    pub(crate) state_existed: bool,
    #[serde(default)]
    pub(crate) provenance: ProvenanceKind,
    pub(crate) files: Vec<JournalFile>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TransactionPhase {
    Applying,
    Committed,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JournalFile {
    pub(crate) path: String,
    pub(crate) existed: bool,
}

/// Test-only deterministic failure hook.
///
/// It adds no production surface: the whole module is compiled only under
/// `cfg(test)`, and it is inert until a unit test arms a point. Arming is keyed
/// by the exact workspace root so parallel tests in one process cannot
/// interfere. No environment variable and no dependency are involved.
#[cfg(test)]
pub(crate) mod faults {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use crate::application::PortError;

    /// The two deterministic injection points acceptance row 2 requires.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) enum InjectionPoint {
        /// After the first staged workspace mutation.
        AfterFirstMutation,
        /// Immediately before the add-on provenance record is written.
        BeforeProvenanceWrite,
    }

    static ARMED: Mutex<Vec<(PathBuf, InjectionPoint)>> = Mutex::new(Vec::new());

    /// Arm one injection point for `root`.
    pub(crate) fn arm(root: &Path, point: InjectionPoint) {
        let mut armed = ARMED.lock().unwrap_or_else(|error| error.into_inner());
        armed.retain(|(armed_root, _)| armed_root != root);
        armed.push((root.to_path_buf(), point));
    }

    /// Disarm every injection point for `root`.
    pub(crate) fn disarm(root: &Path) {
        let mut armed = ARMED.lock().unwrap_or_else(|error| error.into_inner());
        armed.retain(|(armed_root, _)| armed_root != root);
    }

    /// Fail once when `point` is armed for `root`, consuming the arming.
    pub(crate) fn trigger(root: &Path, point: InjectionPoint) -> Result<(), PortError> {
        let mut armed = ARMED.lock().unwrap_or_else(|error| error.into_inner());
        match armed
            .iter()
            .position(|(armed_root, armed_point)| armed_root == root && *armed_point == point)
        {
            Some(index) => {
                armed.remove(index);
                Err(PortError::new(format!(
                    "injected add-on apply failure at {point:?}"
                )))
            }
            None => Ok(()),
        }
    }
}
