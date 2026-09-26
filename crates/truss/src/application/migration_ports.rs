//! Ports owned by `truss migrate` (D-05, D-06).
//!
//! `MigrationPort` is the only surface that touches the filesystem during a
//! migration: it inventories, journals, backs up, stages, publishes, rolls
//! back, and discovers recovery. `CanonicalEntrypointsPort` supplies the
//! compile-time canonical managed blocks, so the application never reads a
//! source-checkout path and never imports infrastructure.

use std::path::Path;

use super::PortError;
use crate::domain::{
    IncompleteJournal, IntegrationOperation, InventoryEntry, MigrationOperation, MigrationPlan,
    MigrationReason, MigrationState,
};

/// The exact marker-delimited managed blocks compiled into the executable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalBlocks {
    pub agents: Vec<u8>,
    pub claude: Vec<u8>,
}

impl CanonicalBlocks {
    /// The canonical block for one optional mixed entrypoint file.
    pub fn for_path(&self, path: &str) -> Option<&[u8]> {
        match path {
            "AGENTS.md" => Some(&self.agents),
            "CLAUDE.md" => Some(&self.claude),
            _ => None,
        }
    }
}

pub trait CanonicalEntrypointsPort {
    fn blocks(&self) -> Result<CanonicalBlocks, PortError>;
}

/// The bytes of the executable this process is running.
///
/// `truss migrate --apply` publishes exactly these bytes as the bundled
/// entrypoint `.truss/core/bin/truss` (ADR 0008 amendment "Bundled executable
/// refresh", owner decision A). Resolving the payload through the port keeps
/// the application free of process and filesystem concerns and lets unit tests
/// inject deterministic bytes instead of the test-harness image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutablePayload {
    pub bytes: Vec<u8>,
}

impl ExecutablePayload {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

/// The complete read-only classification of one repository (D-08, D-11–D-13).
///
/// The adapter resolves every path transform and comparison so the application
/// stays free of JSON and filesystem concerns; the application adds only the
/// platform and state vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationInspection {
    pub repository: String,
    pub legacy_roots: Vec<String>,
    pub run_key: Option<String>,
    pub inventory: Vec<InventoryEntry>,
    pub operations: Vec<MigrationOperation>,
    pub integration: Vec<IntegrationOperation>,
    pub evidence: Vec<String>,
    pub conflicts: Vec<String>,
    pub blocked: Option<MigrationReason>,
    pub not_installed: bool,
    pub already_migrated: bool,
    pub existing_backup: Option<String>,
    pub recovery: Option<IncompleteJournal>,
}

impl MigrationInspection {
    pub fn empty(repository: String) -> Self {
        Self {
            repository,
            legacy_roots: Vec::new(),
            run_key: None,
            inventory: Vec::new(),
            operations: Vec::new(),
            integration: Vec::new(),
            evidence: Vec::new(),
            conflicts: Vec::new(),
            blocked: None,
            not_installed: false,
            already_migrated: false,
            existing_backup: None,
            recovery: None,
        }
    }
}

/// The immutable plan and the observed transaction result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationExecution {
    pub state: MigrationState,
    pub reason: Option<MigrationReason>,
    pub backup_path: Option<String>,
    pub transaction_id: Option<String>,
    pub conflicts: Vec<String>,
    pub evidence: Vec<String>,
}

impl MigrationExecution {
    pub fn new(state: MigrationState) -> Self {
        Self {
            state,
            reason: None,
            backup_path: None,
            transaction_id: None,
            conflicts: Vec::new(),
            evidence: Vec::new(),
        }
    }
}

pub trait MigrationPort {
    /// Whether this build and host may apply a migration (D-04: Linux only).
    fn supported(&self) -> bool;

    /// The bytes of the currently running executable, which the apply publishes
    /// as the bundled entrypoint inside the same transaction (ADR 0008
    /// amendment, owner decision A).
    fn running_executable(&self) -> Result<ExecutablePayload, PortError>;

    /// Classify the repository without mutating anything, including no lock
    /// and no backup directory (REQ-002).
    fn inspect(
        &self,
        root: &Path,
        blocks: &CanonicalBlocks,
    ) -> Result<MigrationInspection, PortError>;

    /// Run the frozen plan as one journaled transaction (D-08, D-09, D-13).
    fn execute(&self, root: &Path, plan: &MigrationPlan) -> Result<MigrationExecution, PortError>;

    /// Roll the single incomplete journal back and report the honest outcome
    /// (D-12, REQ-032).
    fn recover(&self, root: &Path) -> Result<MigrationExecution, PortError>;
}
