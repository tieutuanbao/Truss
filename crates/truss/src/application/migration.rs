//! The `truss migrate` application facade (D-06, D-07, D-14).
//!
//! The facade sequences preflight, planning, apply, and recovery. It owns no
//! filesystem handle: every byte-level action and every JSON record goes
//! through [`MigrationPort`], and the canonical blocks through
//! [`CanonicalEntrypointsPort`]. Preview and apply share exactly one plan
//! builder, so the report an operator previews is the plan an apply executes.

use std::path::Path;

use super::{
    CanonicalEntrypointsPort, MigrationExecution, MigrationInspection, MigrationPort, PortError,
};
use crate::domain::{
    backup_template, MigrationPlan, MigrationReason, MigrationReport, MigrationState,
};

pub struct MigrationApplication<P, C> {
    port: P,
    canonical: C,
}

impl<P, C> MigrationApplication<P, C>
where
    P: MigrationPort,
    C: CanonicalEntrypointsPort,
{
    pub fn new(port: P, canonical: C) -> Self {
        Self { port, canonical }
    }

    /// Preview: classify and plan, with no filesystem mutation whatsoever.
    pub fn preview(&self, root: &Path) -> Result<MigrationReport, PortError> {
        let blocks = self.canonical.blocks()?;
        let inspection = self.port.inspect(root, &blocks)?;
        Ok(self.preview_report(inspection))
    }

    /// Apply: recover any incomplete transaction first, then preflight, then
    /// execute the frozen plan as one transaction.
    pub fn apply(&self, root: &Path) -> Result<MigrationReport, PortError> {
        let blocks = self.canonical.blocks()?;
        let mut inspection = self.port.inspect(root, &blocks)?;
        let mut recovery_evidence = Vec::new();

        if inspection.recovery.is_some() {
            let recovery = self.port.recover(root)?;
            if recovery.state != MigrationState::RolledBack {
                return Ok(from_execution(&inspection, recovery, true));
            }
            recovery_evidence = recovery.evidence;
            inspection = self.port.inspect(root, &blocks)?;
        }

        if inspection.not_installed {
            return Ok(blocked(&inspection, MigrationReason::NotInstalled, true));
        }
        if inspection.already_migrated {
            let mut report = already_migrated(&inspection, true);
            report.evidence.extend(recovery_evidence);
            return Ok(report);
        }
        if let Some(reason) = inspection.blocked {
            return Ok(blocked(&inspection, reason, true));
        }
        if !self.port.supported() {
            return Ok(blocked(
                &inspection,
                MigrationReason::UnsupportedApplyPlatform,
                true,
            ));
        }

        let plan = plan_from(&inspection);
        let execution = self.port.execute(root, &plan)?;
        let mut report = from_execution(&inspection, execution, true);
        report.evidence.extend(recovery_evidence);
        Ok(report)
    }

    fn preview_report(&self, inspection: MigrationInspection) -> MigrationReport {
        if inspection.recovery.is_some() {
            let mut report = base_report(&inspection, MigrationState::RecoveryRequired, None);
            report.evidence.extend(recovery_evidence(&inspection));
            return report;
        }
        if inspection.not_installed {
            return blocked(&inspection, MigrationReason::NotInstalled, false);
        }
        if inspection.already_migrated {
            return already_migrated(&inspection, false);
        }
        if let Some(reason) = inspection.blocked {
            return blocked(&inspection, reason, false);
        }
        if !self.port.supported() {
            return blocked(
                &inspection,
                MigrationReason::UnsupportedApplyPlatform,
                false,
            );
        }
        base_report(&inspection, MigrationState::Ready, None)
    }
}

/// Build the one immutable plan both verbs share.
pub fn plan_from(inspection: &MigrationInspection) -> MigrationPlan {
    MigrationPlan {
        repository: inspection.repository.clone(),
        backup_template: backup_template(&inspection.repository),
        run_key: inspection.run_key.clone(),
        legacy_roots: inspection.legacy_roots.clone(),
        inventory: inspection.inventory.clone(),
        operations: inspection.operations.clone(),
        integration: inspection.integration.clone(),
        evidence: inspection.evidence.clone(),
    }
}

fn base_report(
    inspection: &MigrationInspection,
    state: MigrationState,
    reason: Option<MigrationReason>,
) -> MigrationReport {
    MigrationReport {
        state,
        reason,
        applied: false,
        repository: inspection.repository.clone(),
        backup_path: None,
        backup_path_template: backup_template(&inspection.repository),
        transaction_id: None,
        run_key: inspection.run_key.clone(),
        legacy_roots: inspection.legacy_roots.clone(),
        operations: inspection.operations.clone(),
        conflicts: inspection.conflicts.clone(),
        evidence: inspection.evidence.clone(),
    }
}

fn blocked(
    inspection: &MigrationInspection,
    reason: MigrationReason,
    applied: bool,
) -> MigrationReport {
    let mut report = base_report(inspection, MigrationState::Blocked, Some(reason));
    report.applied = applied;
    report
}

fn already_migrated(inspection: &MigrationInspection, applied: bool) -> MigrationReport {
    let mut report = base_report(inspection, MigrationState::AlreadyMigrated, None);
    report.applied = applied;
    report.backup_path = inspection.existing_backup.clone();
    report
}

fn from_execution(
    inspection: &MigrationInspection,
    execution: MigrationExecution,
    applied: bool,
) -> MigrationReport {
    let mut report = base_report(inspection, execution.state, execution.reason);
    report.applied = applied && execution.state == MigrationState::Migrated;
    report.backup_path = execution.backup_path.clone();
    report.transaction_id = execution.transaction_id.clone();
    report.conflicts = execution.conflicts.clone();
    report.evidence.extend(execution.evidence);
    report
}

fn recovery_evidence(inspection: &MigrationInspection) -> Vec<String> {
    match &inspection.recovery {
        Some(journal) => vec![format!(
            "incomplete transaction {} reached phase {} (journal {})",
            journal.transaction_id, journal.phase, journal.path
        )],
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::application::CanonicalBlocks;
    use crate::domain::{ContentHash, MigrationOperation, OperationKind};

    /// A port double: it answers `supported` from a flag, records whether
    /// `execute` was reached, and never touches a filesystem. It is how the
    /// Windows refusal is discriminated on a Linux host.
    struct FakePort {
        supported: bool,
        executed: std::cell::Cell<bool>,
        blocked: Option<MigrationReason>,
    }

    impl MigrationPort for FakePort {
        fn supported(&self) -> bool {
            self.supported
        }

        fn inspect(
            &self,
            root: &Path,
            _blocks: &CanonicalBlocks,
        ) -> Result<MigrationInspection, PortError> {
            let mut inspection = MigrationInspection::empty(root.display().to_string());
            inspection.run_key = Some("run-a".to_owned());
            inspection.legacy_roots = vec![".truss-core".to_owned()];
            inspection.blocked = self.blocked;
            inspection.operations.push(MigrationOperation {
                source: Some(".truss-core/manifest.json".to_owned()),
                destination: ".truss/core/manifest.json".to_owned(),
                kind: OperationKind::Create,
                source_hash: Some(ContentHash::parse("a".repeat(64)).unwrap()),
                before_hash: None,
                after_hash: ContentHash::parse("b".repeat(64)).unwrap(),
                content: None,
            });
            Ok(inspection)
        }

        fn execute(
            &self,
            _root: &Path,
            plan: &MigrationPlan,
        ) -> Result<MigrationExecution, PortError> {
            self.executed.set(true);
            let mut execution = MigrationExecution::new(MigrationState::Migrated);
            execution.backup_path = Some(format!(
                "{}/.truss-migration-backup/20260101T000000.000000000Z/",
                plan.repository
            ));
            execution.transaction_id = Some("tx".to_owned());
            Ok(execution)
        }

        fn recover(&self, _root: &Path) -> Result<MigrationExecution, PortError> {
            Ok(MigrationExecution::new(MigrationState::RolledBack))
        }
    }

    struct FakeCanonical;

    impl CanonicalEntrypointsPort for FakeCanonical {
        fn blocks(&self) -> Result<CanonicalBlocks, PortError> {
            Ok(CanonicalBlocks {
                agents: b"<!-- TRUSS:BEGIN -->\nA\n<!-- TRUSS:END -->".to_vec(),
                claude: b"<!-- TRUSS:BEGIN -->\nC\n<!-- TRUSS:END -->".to_vec(),
            })
        }
    }

    fn app(supported: bool) -> MigrationApplication<FakePort, FakeCanonical> {
        MigrationApplication::new(
            FakePort {
                supported,
                executed: std::cell::Cell::new(false),
                blocked: None,
            },
            FakeCanonical,
        )
    }

    /// REQ-035 / D-04: the plan is owned data and execution never mutates it.
    #[test]
    fn execution_leaves_the_frozen_plan_byte_identical() {
        let application = app(true);
        let inspection = application
            .port
            .inspect(
                Path::new("/repo"),
                &CanonicalBlocks {
                    agents: Vec::new(),
                    claude: Vec::new(),
                },
            )
            .unwrap();
        let plan = plan_from(&inspection);
        let before = plan.clone();
        application.port.execute(Path::new("/repo"), &plan).unwrap();
        assert_eq!(before, plan, "the port must not mutate the plan");
    }

    /// REQ-004 / D-04: an unsupported platform refuses before any execution.
    #[test]
    fn unsupported_platform_blocks_preview_and_apply_without_executing() {
        let application = app(false);
        let preview = application.preview(Path::new("/repo")).unwrap();
        assert_eq!(preview.state, MigrationState::Blocked);
        assert_eq!(
            preview.reason,
            Some(MigrationReason::UnsupportedApplyPlatform)
        );
        let apply = application.apply(Path::new("/repo")).unwrap();
        assert_eq!(apply.state, MigrationState::Blocked);
        assert_eq!(
            apply.reason,
            Some(MigrationReason::UnsupportedApplyPlatform)
        );
        assert!(
            !application.port.executed.get(),
            "a refusal must never reach execute, so no lock or mutation happens"
        );
    }

    #[test]
    fn supported_platform_reaches_execute_and_exits_zero() {
        let application = app(true);
        let apply = application.apply(Path::new("/repo")).unwrap();
        assert!(application.port.executed.get());
        assert_eq!(apply.state, MigrationState::Migrated);
        assert_eq!(apply.state.exit_code(), 0);
        assert!(apply.backup_path.is_some());
    }

    #[test]
    fn a_blocked_inspection_never_reaches_execute() {
        let application = MigrationApplication::new(
            FakePort {
                supported: true,
                executed: std::cell::Cell::new(false),
                blocked: Some(MigrationReason::UnknownDocument),
            },
            FakeCanonical,
        );
        let preview = application.preview(Path::new("/repo")).unwrap();
        assert_eq!(preview.state, MigrationState::Blocked);
        assert_eq!(preview.reason, Some(MigrationReason::UnknownDocument));
        let apply = application.apply(Path::new("/repo")).unwrap();
        assert_eq!(apply.state, MigrationState::Blocked);
        assert!(!application.port.executed.get());
    }
}
