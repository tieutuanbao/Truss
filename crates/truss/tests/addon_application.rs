//! S4b2 application boundary: the add-on facade orchestrates through ports only.
//!
//! Every fake-port row drives the facade with a workspace root that does not
//! exist on disk, so a facade that touched the filesystem itself would fail
//! instead of silently passing. The recorded call log is the instrument: it
//! names exactly which port method each operation reached, in which order, and
//! the core installation-state port is implemented by the fake as well, so a
//! facade that asked the core namespace would show a `core.` entry.

mod common;

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use truss::application::{
    AddOnApplication, AddOnApplyRequest, AddOnExecutionPort, AddOnInstallRequest, AddOnPayloadPort,
    AddOnPayloadSpec, AddOnPlanPort, AddOnPlanRequest, AddOnRecordReceipt, AddOnStageRequest,
    AddOnStatePort, InstallationStatePort, PortError,
};
use truss::domain::{
    AddOnDescriptor, AddOnInstallation, AddOnName, AddOnPayloadFile, AddOnState, ApplyReceipt,
    BaselineFile, ConflictReason, ContentHash, FileChangeKind, FrozenWorkspaceFile,
    InstallationState, PlannedFileChange, RelativePath, ResolutionConflict, SourceRef,
    UpdateConflict, UpdatePlan, UpdateResolutionSession, WorkspaceMutation,
};
use truss::infrastructure::{
    FileSystemAddOnApplier, FileSystemAddOnPayload, FileSystemAddOnPlanner, FileSystemAddOnState,
};

use common::{seed_core_state, snapshot_digest, workspace_snapshot, write_bytes};

const ADDON: &str = "demo";
const SUBJECT: &str = ".agents/skills/demo/SKILL.md";
const CLEAN: &str = ".agents/skills/demo/clean.md";
const SOURCE_REF: &str = "truss-v0.1.14";
/// A path the fake records to the core, so the ownership guard has a real
/// owner to compare against and the rows stay green only because this owner
/// does not collide with `SUBJECT` or `CLEAN`.
const FOREIGN: &str = ".agents/skills/truss/SKILL.md";
const CORE_OWNER: &str = "truss-core";

type Log = Rc<RefCell<Vec<String>>>;

fn record(log: &Log, event: &str) {
    log.borrow_mut().push(event.to_owned());
}

/// A workspace root that does not exist: the fake ports ignore it, so any
/// filesystem access by the facade itself would surface as a failure.
fn absent_root() -> &'static Path {
    Path::new("s4b2-absent-workspace-root")
}

/// A root the fake reports as holding both trees, so the refusal path is
/// reachable without a filesystem.
fn conflicting_root() -> &'static Path {
    Path::new("conflicting-root")
}

fn add_on() -> AddOnName {
    AddOnName::parse(ADDON).unwrap()
}

fn path(value: &str) -> RelativePath {
    RelativePath::parse(value).unwrap()
}

fn digest(byte: &str) -> ContentHash {
    ContentHash::parse(byte.repeat(64)).unwrap()
}

fn evidence(name: &str, body: &str) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/s4b2-evidence");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(name), body).unwrap();
}

struct FakePayload {
    log: Log,
    descriptor: AddOnDescriptor,
}

impl AddOnPayloadPort for FakePayload {
    fn describe(&self, _spec: &AddOnPayloadSpec<'_>) -> Result<AddOnDescriptor, PortError> {
        record(&self.log, "payload.describe");
        Ok(self.descriptor.clone())
    }

    fn verify(
        &self,
        _spec: &AddOnPayloadSpec<'_>,
        _descriptor: &AddOnDescriptor,
    ) -> Result<(), PortError> {
        record(&self.log, "payload.verify");
        Ok(())
    }
}

struct FakePlanner {
    log: Log,
    plan: UpdatePlan,
}

impl AddOnPlanPort for FakePlanner {
    fn plan(&self, _root: &Path, _request: &AddOnPlanRequest<'_>) -> Result<UpdatePlan, PortError> {
        record(&self.log, "planner.plan");
        Ok(self.plan.clone())
    }
}

struct FakeExecutor {
    log: Log,
    session_pending: bool,
    abort_removed: bool,
    apply_backup: Option<String>,
    resume_backup: Option<String>,
}

impl AddOnExecutionPort for FakeExecutor {
    fn apply(
        &self,
        _root: &Path,
        _request: &AddOnApplyRequest<'_>,
    ) -> Result<ApplyReceipt, PortError> {
        record(&self.log, "executor.apply");
        Ok(ApplyReceipt {
            backup_path: self.apply_backup.clone(),
        })
    }

    fn stage(&self, _root: &Path, _request: &AddOnStageRequest<'_>) -> Result<(), PortError> {
        record(&self.log, "executor.stage");
        Ok(())
    }

    fn resume(&self, _root: &Path, _name: &AddOnName) -> Result<ApplyReceipt, PortError> {
        record(&self.log, "executor.resume");
        Ok(ApplyReceipt {
            backup_path: self.resume_backup.clone(),
        })
    }

    fn abort(&self, _root: &Path, _name: &AddOnName) -> Result<bool, PortError> {
        record(&self.log, "executor.abort");
        Ok(self.abort_removed)
    }

    fn session_pending(&self, _root: &Path, _name: &AddOnName) -> Result<bool, PortError> {
        record(&self.log, "executor.session_pending");
        Ok(self.session_pending)
    }
}

struct FakeState {
    log: Log,
    installation: Option<AddOnInstallation>,
    adopted: bool,
    core_pending: bool,
}

impl AddOnStatePort for FakeState {
    /// Report the root resolution without recording a log entry: the log is the
    /// instrument for *which* port method an operation reached and in what
    /// order, and every row's expected vector names the operations that decide
    /// its contract. Root resolution is a precondition of each of them, so
    /// recording it would rewrite every row without proving anything new.
    fn resolve_state_root(&self, root: &Path) -> Result<PathBuf, PortError> {
        if root == conflicting_root() {
            return Err(PortError::new(
                "both .truss/core and .truss-core hold a Truss installation",
            ));
        }
        Ok(root.to_path_buf())
    }

    fn load(&self, _root: &Path) -> Result<Option<AddOnState>, PortError> {
        record(&self.log, "state.load");
        Ok(self.installation.clone().map(|installation| AddOnState {
            schema_version: AddOnState::SCHEMA_VERSION,
            addons: vec![installation],
        }))
    }

    fn apply(
        &self,
        _root: &Path,
        _request: &AddOnInstallRequest<'_>,
    ) -> Result<AddOnRecordReceipt, PortError> {
        record(&self.log, "state.apply");
        Ok(AddOnRecordReceipt {
            adopted: self.adopted,
        })
    }

    /// The recorded ownership set the real adapter reads from
    /// `.truss/core/manifest.json` and `.truss/core/addons.json`.
    ///
    /// The fake records the call so the S4b2 rows prove the ownership guard is
    /// consulted *before* the planner and *before* `apply`, and it reports one
    /// real owner that does not collide with the fixture descriptor, so a row
    /// stays green only because the guard admitted the payload.
    fn recorded_owners(
        &self,
        _root: &Path,
        _own_name: &AddOnName,
    ) -> Result<Vec<(String, Vec<RelativePath>)>, PortError> {
        record(&self.log, "state.recorded_owners");
        Ok(vec![(CORE_OWNER.to_owned(), vec![path(FOREIGN)])])
    }
}

impl FakeState {
    fn refuse_core_call(&self, method: &str) -> PortError {
        record(&self.log, &format!("core.{method}"));
        PortError::new(format!(
            "the add-on facade must not reach the core installation-state port: {method}"
        ))
    }
}

/// The core state port is implemented too, so the counterexample is possible in
/// principle: this fake *can* answer `resolution_pending` with a pending core
/// session. The facade is generic over `AddOnStatePort` alone, and every call
/// would be recorded, so a wrong-namespace facade is rejected by the log.
impl InstallationStatePort for FakeState {
    fn resolve_state_root(&self, _root: &Path) -> Result<PathBuf, PortError> {
        Err(self.refuse_core_call("resolve_state_root"))
    }

    fn recover_interrupted(&self, _root: &Path) -> Result<bool, PortError> {
        Err(self.refuse_core_call("recover_interrupted"))
    }

    fn transaction_pending(&self, _root: &Path) -> Result<bool, PortError> {
        Err(self.refuse_core_call("transaction_pending"))
    }

    fn load(&self, _root: &Path) -> Result<Option<InstallationState>, PortError> {
        Err(self.refuse_core_call("load"))
    }

    fn read_workspace_file(
        &self,
        _root: &Path,
        _path: &RelativePath,
    ) -> Result<Option<Vec<u8>>, PortError> {
        Err(self.refuse_core_call("read_workspace_file"))
    }

    fn validate_managed_path(&self, _root: &Path, _path: &RelativePath) -> Result<(), PortError> {
        Err(self.refuse_core_call("validate_managed_path"))
    }

    fn apply(
        &self,
        _root: &Path,
        _state: &InstallationState,
        _mutations: &[WorkspaceMutation],
    ) -> Result<ApplyReceipt, PortError> {
        Err(self.refuse_core_call("apply"))
    }

    fn apply_if_unchanged(
        &self,
        _root: &Path,
        _state: &InstallationState,
        _mutations: &[WorkspaceMutation],
        _expected: &[FrozenWorkspaceFile],
    ) -> Result<ApplyReceipt, PortError> {
        Err(self.refuse_core_call("apply_if_unchanged"))
    }

    fn resolution_pending(&self, _root: &Path) -> Result<bool, PortError> {
        record(&self.log, "core.resolution_pending");
        Ok(self.core_pending)
    }

    fn stage_resolution(
        &self,
        _root: &Path,
        _session: &UpdateResolutionSession,
    ) -> Result<(), PortError> {
        Err(self.refuse_core_call("stage_resolution"))
    }

    fn load_resolution(&self, _root: &Path) -> Result<Option<UpdateResolutionSession>, PortError> {
        Err(self.refuse_core_call("load_resolution"))
    }

    fn clear_resolution(&self, _root: &Path) -> Result<bool, PortError> {
        Err(self.refuse_core_call("clear_resolution"))
    }
}

struct Harness {
    log: Log,
    application: AddOnApplication<FakePayload, FakeState, FakePlanner, FakeExecutor>,
}

impl Harness {
    fn calls(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    fn assert_no_core_calls(&self) {
        let calls = self.calls();
        assert!(
            !calls.iter().any(|call| call.starts_with("core.")),
            "the facade reached the core installation-state port: {calls:?}"
        );
    }
}

fn harness_with(
    plan: UpdatePlan,
    installation: Option<AddOnInstallation>,
    adopted: bool,
    core_pending: bool,
    session_pending: bool,
    abort_removed: bool,
) -> Harness {
    let log: Log = Rc::new(RefCell::new(Vec::new()));
    let application = AddOnApplication::new(
        FakePayload {
            log: Rc::clone(&log),
            descriptor: descriptor(),
        },
        FakeState {
            log: Rc::clone(&log),
            installation,
            adopted,
            core_pending,
        },
        FakePlanner {
            log: Rc::clone(&log),
            plan,
        },
        FakeExecutor {
            log: Rc::clone(&log),
            session_pending,
            abort_removed,
            apply_backup: Some("backup/apply".to_owned()),
            resume_backup: Some("backup/resume".to_owned()),
        },
    );
    Harness { log, application }
}

fn fake_harness(plan: UpdatePlan) -> Harness {
    harness_with(plan, Some(installation()), false, false, false, true)
}

fn descriptor() -> AddOnDescriptor {
    AddOnDescriptor {
        name: add_on(),
        source_ref: SourceRef::parse(SOURCE_REF).unwrap(),
        source_core_version: "0.1.14".to_owned(),
        files: vec![AddOnPayloadFile {
            path: path(SUBJECT),
            sha256: digest("a"),
        }],
    }
}

fn installation() -> AddOnInstallation {
    AddOnInstallation {
        name: add_on(),
        source_ref: SourceRef::parse("truss-v0.1.13").unwrap(),
        source_core_version: "0.1.13".to_owned(),
        files: vec![BaselineFile {
            path: path(SUBJECT),
            content: b"base\n".to_vec(),
            hash: digest("b"),
        }],
    }
}

fn clean_plan() -> UpdatePlan {
    UpdatePlan {
        changes: vec![PlannedFileChange {
            path: path(SUBJECT),
            kind: FileChangeKind::Update,
        }],
        mutations: vec![WorkspaceMutation::Write {
            path: path(SUBJECT),
            content: b"next\n".to_vec(),
        }],
        frozen_files: vec![FrozenWorkspaceFile {
            path: path(SUBJECT),
            content: Some(b"local\n".to_vec()),
        }],
        ..UpdatePlan::default()
    }
}

/// One conflicted path plus one clean create: the clean subset must never be
/// applied by an `update`.
fn conflicted_plan() -> UpdatePlan {
    UpdatePlan {
        changes: vec![
            PlannedFileChange {
                path: path(SUBJECT),
                kind: FileChangeKind::Update,
            },
            PlannedFileChange {
                path: path(CLEAN),
                kind: FileChangeKind::Create,
            },
        ],
        conflicts: vec![UpdateConflict {
            path: path(SUBJECT),
            reason: ConflictReason::OverlappingChanges,
            detail: "both sides changed".to_owned(),
        }],
        resolution_conflicts: vec![ResolutionConflict {
            path: path(SUBJECT),
            base: b"base\n".to_vec(),
            local: b"local\n".to_vec(),
            incoming: b"next\n".to_vec(),
            resolved: b"<<<<<<< local\nlocal\n=======\nnext\n>>>>>>> next\n".to_vec(),
        }],
        mutations: vec![WorkspaceMutation::Write {
            path: path(CLEAN),
            content: b"clean\n".to_vec(),
        }],
        frozen_files: vec![
            FrozenWorkspaceFile {
                path: path(SUBJECT),
                content: Some(b"local\n".to_vec()),
            },
            FrozenWorkspaceFile {
                path: path(CLEAN),
                content: None,
            },
        ],
    }
}

fn spec<'a>(root: &'a Path, manifest: &'a Path) -> AddOnPayloadSpec<'a> {
    AddOnPayloadSpec {
        root,
        manifest,
        name: ADDON,
        source_ref: SOURCE_REF,
        source_core_version: "0.1.14",
        foreign_manifests: &[],
    }
}

/// Every mutating entry point resolves the root before it reaches any other
/// port: a repository holding both trees is refused, and the refusal is the
/// first thing that happens rather than something between two mutations.
#[test]
fn every_mutating_entry_point_refuses_a_conflicting_root_first() {
    let harness = fake_harness(clean_plan());
    let root = conflicting_root();
    let payload = spec(root, root);

    let refusals = [
        harness
            .application
            .install(root, &payload, false)
            .map(|_| ()),
        harness
            .application
            .update(root, &payload, false)
            .map(|_| ()),
        harness
            .application
            .continue_update(root, &add_on())
            .map(|_| ()),
        harness.application.abort(root, &add_on()).map(|_| ()),
    ];

    for refusal in refusals {
        let error = refusal.unwrap_err().to_string();
        assert!(error.contains(".truss/core"), "names the new root: {error}");
        assert!(
            error.contains(".truss-core"),
            "names the legacy root: {error}"
        );
    }
    assert!(
        harness.calls().is_empty(),
        "a refused operation reaches no other port: {:?}",
        harness.calls()
    );
}

/// Acceptance row 1: `install` reaches only the payload port for a dry run and
/// the payload plus state ports for a real install, and reports the state
/// port's adoption receipt.
#[test]
fn install_reaches_only_the_payload_and_state_ports() {
    let harness = harness_with(UpdatePlan::default(), None, true, false, false, true);

    let preview = harness
        .application
        .install(absent_root(), &spec(absent_root(), absent_root()), true)
        .unwrap();
    assert_eq!(
        harness.calls(),
        vec!["payload.describe", "state.recorded_owners"]
    );
    assert!(preview.dry_run && !preview.applied && !preview.resolution_staged);
    assert_eq!(preview.adopted, None);
    assert!(preview.changes.is_empty() && preview.conflicts.is_empty());
    assert_eq!(preview.source_ref.unwrap().as_str(), SOURCE_REF);

    let applied = harness
        .application
        .install(absent_root(), &spec(absent_root(), absent_root()), false)
        .unwrap();
    // The log is cumulative: one `payload.describe` plus its ownership guard
    // from the preview above, then describe, guard, and apply for the real
    // install.
    assert_eq!(
        harness.calls(),
        vec![
            "payload.describe",
            "state.recorded_owners",
            "payload.describe",
            "state.recorded_owners",
            "state.apply"
        ]
    );
    assert!(!applied.dry_run && applied.applied);
    assert_eq!(applied.adopted, Some(true));
    harness.assert_no_core_calls();

    evidence(
        "s4b2-row1-install.txt",
        &format!(
            "dry_run_calls={:?}\nreal_install_calls={:?}\nadopted={:?}\n",
            vec!["payload.describe", "state.recorded_owners"],
            vec!["payload.describe", "state.recorded_owners", "state.apply"],
            applied.adopted,
        ),
    );
}

/// Acceptance row 1 and row 2: a clean `update` dry run plans and never
/// applies, and the real update applies only through the execution port.
#[test]
fn update_dry_run_plans_and_never_applies() {
    let harness = fake_harness(clean_plan());
    let preview = harness
        .application
        .update(absent_root(), &spec(absent_root(), absent_root()), true)
        .unwrap();
    assert_eq!(
        harness.calls(),
        vec!["payload.describe", "state.recorded_owners", "planner.plan"]
    );
    assert!(preview.dry_run);
    assert_eq!(preview.changes, clean_plan().changes);
    assert!(preview.conflicts.is_empty());
    assert!(!preview.applied && !preview.resolution_staged);
    assert_eq!(preview.backup_path, None);

    let harness = fake_harness(clean_plan());
    let applied = harness
        .application
        .update(absent_root(), &spec(absent_root(), absent_root()), false)
        .unwrap();
    assert_eq!(
        harness.calls(),
        vec![
            "payload.describe",
            "state.recorded_owners",
            "planner.plan",
            "executor.apply"
        ]
    );
    assert!(applied.applied && !applied.dry_run && !applied.resolution_staged);
    assert_eq!(applied.backup_path.as_deref(), Some("backup/apply"));
    assert_eq!(applied.changes, clean_plan().changes);
    harness.assert_no_core_calls();

    evidence(
        "s4b2-row2-dry-run.txt",
        &format!(
            "dry_run_calls={:?}\ndry_run_applied={}\nreal_calls={:?}\nreal_applied={}\n",
            vec!["payload.describe", "state.recorded_owners", "planner.plan"],
            preview.applied,
            harness.calls(),
            applied.applied,
        ),
    );
}

/// Acceptance row 2's counterexample: a plan carrying a conflict is staged, not
/// applied — not even its clean subset — and a dry run stages nothing either.
#[test]
fn update_stages_a_conflicted_plan_instead_of_applying_it() {
    let harness = fake_harness(conflicted_plan());
    let preview = harness
        .application
        .update(absent_root(), &spec(absent_root(), absent_root()), true)
        .unwrap();
    assert_eq!(
        harness.calls(),
        vec!["payload.describe", "state.recorded_owners", "planner.plan"]
    );
    assert!(preview.dry_run && !preview.applied && !preview.resolution_staged);
    assert_eq!(preview.conflicts, conflicted_plan().conflicts);
    assert_eq!(preview.changes, conflicted_plan().changes);

    let harness = fake_harness(conflicted_plan());
    let staged = harness
        .application
        .update(absent_root(), &spec(absent_root(), absent_root()), false)
        .unwrap();
    assert_eq!(
        harness.calls(),
        vec![
            "payload.describe",
            "state.recorded_owners",
            "planner.plan",
            "executor.stage"
        ]
    );
    assert!(!staged.applied, "a conflicted plan must not be applied");
    assert!(staged.resolution_staged);
    assert_eq!(staged.conflicts, conflicted_plan().conflicts);
    assert_eq!(staged.changes, conflicted_plan().changes);
    assert_eq!(staged.backup_path, None);
    harness.assert_no_core_calls();

    evidence(
        "s4b2-row2-conflict.txt",
        &format!(
            "dry_run_calls={:?}\ndry_run_staged={}\nconflict_calls={:?}\nconflict_applied={}\nconflict_staged={}\n",
            vec!["payload.describe", "state.recorded_owners", "planner.plan"],
            preview.resolution_staged,
            harness.calls(),
            staged.applied,
            staged.resolution_staged,
        ),
    );
}

/// Acceptance row 1: `continue` reaches the execution port's `resume` and
/// nothing else — never the planner and never the payload port.
#[test]
fn continue_resumes_without_the_payload_or_the_planner() {
    let harness = fake_harness(UpdatePlan::default());
    let report = harness
        .application
        .continue_update(absent_root(), &add_on())
        .unwrap();
    assert_eq!(harness.calls(), vec!["executor.resume"]);
    assert!(report.applied && !report.dry_run && !report.resolution_staged);
    assert_eq!(report.source_ref, None);
    assert!(report.changes.is_empty() && report.conflicts.is_empty());
    assert_eq!(report.backup_path.as_deref(), Some("backup/resume"));
    harness.assert_no_core_calls();

    evidence(
        "s4b2-row1-continue.txt",
        &format!(
            "continue_calls={:?}\napplied={}\nsource_ref_absent={}\nbackup={:?}\n",
            harness.calls(),
            report.applied,
            report.source_ref.is_none(),
            report.backup_path,
        ),
    );
}

/// Acceptance row 1: `abort` reaches only the execution port and returns its
/// scoped idempotent answer.
#[test]
fn abort_reaches_only_the_execution_port() {
    let harness = harness_with(
        UpdatePlan::default(),
        Some(installation()),
        false,
        true,
        true,
        true,
    );
    assert!(harness.application.abort(absent_root(), &add_on()).unwrap());
    assert_eq!(harness.calls(), vec!["executor.abort"]);
    harness.assert_no_core_calls();

    let harness = harness_with(
        UpdatePlan::default(),
        Some(installation()),
        false,
        true,
        true,
        false,
    );
    assert!(!harness.application.abort(absent_root(), &add_on()).unwrap());
    assert_eq!(harness.calls(), vec!["executor.abort"]);
}

/// Acceptance row 1 and row 3: `status` reports the recorded provenance from the
/// add-on state port.
#[test]
fn status_reports_the_recorded_provenance_from_the_add_on_state_port() {
    let harness = harness_with(
        UpdatePlan::default(),
        Some(installation()),
        false,
        false,
        false,
        true,
    );
    let report = harness
        .application
        .status(absent_root(), &add_on())
        .unwrap();
    assert_eq!(report.name, add_on());
    assert!(!report.session_pending);
    let record = report.record.expect("a recorded installation");
    assert_eq!(record.source_ref.as_str(), "truss-v0.1.13");
    assert_eq!(record.source_core_version, "0.1.13");
    assert_eq!(
        record.files,
        vec![AddOnPayloadFile {
            path: path(SUBJECT),
            sha256: digest("b"),
        }]
    );
    assert_eq!(
        harness.calls(),
        vec!["state.load", "executor.session_pending"]
    );
    harness.assert_no_core_calls();

    let harness = harness_with(UpdatePlan::default(), None, false, false, false, true);
    let absent = harness
        .application
        .status(absent_root(), &add_on())
        .unwrap();
    assert!(absent.record.is_none());
    assert!(!absent.session_pending);
    assert_eq!(
        harness.calls(),
        vec!["state.load", "executor.session_pending"]
    );
}

/// Acceptance row 3 and its counterexample: with a pending core session, a
/// pending add-on session, and neither, `status` answers from the add-on
/// namespace alone. The fake core port would answer `resolution_pending` with
/// `true`, and the log proves it is never asked.
#[test]
fn status_reports_pending_from_the_add_on_namespace_only() {
    // A pending add-on session and a pending core session.
    let harness = harness_with(
        UpdatePlan::default(),
        Some(installation()),
        false,
        true,
        true,
        true,
    );
    let both = harness
        .application
        .status(absent_root(), &add_on())
        .unwrap();
    assert!(both.session_pending);
    assert_eq!(
        harness.calls(),
        vec!["state.load", "executor.session_pending"]
    );
    harness.assert_no_core_calls();

    // Only a core session is pending: the add-on answer must stay false.
    let harness = harness_with(
        UpdatePlan::default(),
        Some(installation()),
        false,
        true,
        false,
        true,
    );
    let core_only = harness
        .application
        .status(absent_root(), &add_on())
        .unwrap();
    assert!(
        !core_only.session_pending,
        "a pending core session must not be reported as an add-on session"
    );
    assert_eq!(
        harness.calls(),
        vec!["state.load", "executor.session_pending"]
    );
    harness.assert_no_core_calls();

    // Neither is pending.
    let harness = harness_with(UpdatePlan::default(), None, false, false, false, true);
    let neither = harness
        .application
        .status(absent_root(), &add_on())
        .unwrap();
    assert!(!neither.session_pending);
    assert_eq!(
        harness.calls(),
        vec!["state.load", "executor.session_pending"]
    );
    harness.assert_no_core_calls();

    evidence(
        "s4b2-row3-status.txt",
        &format!(
            "addon_pending_and_core_pending={}\ncore_pending_only={}\nneither={}\ncalls={:?}\ncore_port_calls=0\n",
            both.session_pending,
            core_only.session_pending,
            neither.session_pending,
            harness.calls(),
        ),
    );
}

fn real_application() -> AddOnApplication<
    FileSystemAddOnPayload,
    FileSystemAddOnState,
    FileSystemAddOnPlanner,
    FileSystemAddOnApplier,
> {
    AddOnApplication::new(
        FileSystemAddOnPayload,
        FileSystemAddOnState,
        FileSystemAddOnPlanner,
        FileSystemAddOnApplier,
    )
}

fn manifest(files: &[&str]) -> String {
    let mut text = String::from("# s4b2 facade fixture\n");
    for file in files {
        text.push_str(file);
        text.push('\n');
    }
    text
}

/// The real adapters satisfy the ports: the same facade drives install, status,
/// a dry-run preview, a staged conflict, scoped abort, and a payload-free
/// continue against a real workspace, and the add-on pending answer never
/// depends on the seeded core session.
#[test]
fn real_adapters_satisfy_the_ports_end_to_end() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    seed_core_state(&workspace);

    let baseline_payload = tmp.path().join("payload-a");
    let next_payload = tmp.path().join("payload-b");
    let baseline_manifest = tmp.path().join("files-a.txt");
    let next_manifest = tmp.path().join("files-b.txt");
    write_bytes(&baseline_payload, SUBJECT, b"one\ntwo\nthree\n");
    fs::write(&baseline_manifest, manifest(&[SUBJECT])).unwrap();

    let application = real_application();
    let installed = application
        .install(
            &workspace,
            &spec(&baseline_payload, &baseline_manifest),
            false,
        )
        .unwrap();
    assert!(installed.applied && !installed.dry_run);
    assert_eq!(installed.adopted, Some(false));
    assert_eq!(installed.source_ref.unwrap().as_str(), SOURCE_REF);

    // A core session in the core-only namespace must not look like an add-on
    // session, and the record must still be reported.
    let state_root = workspace.join(".truss/core");
    fs::create_dir_all(state_root.join("update")).unwrap();
    fs::write(
        state_root.join("update/session.json"),
        b"{\"schema_version\":1}\n",
    )
    .unwrap();
    let core_session = fs::read(state_root.join("update/session.json")).unwrap();
    let addons_before = fs::read(state_root.join("addons.json")).unwrap();
    let status = application.status(&workspace, &add_on()).unwrap();
    assert!(status.record.is_some());
    assert!(
        !status.session_pending,
        "the core session is not add-on state"
    );

    // A conflicting update previews the plan and mutates nothing.
    write_bytes(&workspace, SUBJECT, b"local overlap\n");
    write_bytes(&next_payload, SUBJECT, b"next overlap\n");
    fs::write(&next_manifest, manifest(&[SUBJECT])).unwrap();
    let before = workspace_snapshot(&workspace);
    let before_digest = snapshot_digest(&before);
    let preview = application
        .update(&workspace, &spec(&next_payload, &next_manifest), true)
        .unwrap();
    assert!(preview.dry_run && !preview.applied && !preview.resolution_staged);
    assert_eq!(preview.conflicts.len(), 1);
    let after_dry_run = workspace_snapshot(&workspace);
    assert_eq!(
        after_dry_run, before,
        "an update dry run must mutate nothing"
    );
    assert!(
        !application
            .status(&workspace, &add_on())
            .unwrap()
            .session_pending
    );

    // The real update stages instead of applying, and changes no managed file
    // or provenance byte.
    let staged = application
        .update(&workspace, &spec(&next_payload, &next_manifest), false)
        .unwrap();
    assert!(!staged.applied && staged.resolution_staged);
    assert_eq!(staged.conflicts.len(), 1);
    assert_eq!(
        fs::read(workspace.join(SUBJECT)).unwrap(),
        b"local overlap\n"
    );
    assert_eq!(
        fs::read(state_root.join("addons.json")).unwrap(),
        addons_before
    );
    assert!(
        application
            .status(&workspace, &add_on())
            .unwrap()
            .session_pending
    );

    // Abort is scoped to the add-on session, idempotent, and leaves the core
    // session byte-identical.
    assert!(application.abort(&workspace, &add_on()).unwrap());
    assert!(!application.abort(&workspace, &add_on()).unwrap());
    assert!(
        !application
            .status(&workspace, &add_on())
            .unwrap()
            .session_pending
    );
    assert_eq!(
        fs::read(state_root.join("update/session.json")).unwrap(),
        core_session
    );

    // Re-stage, resolve the session's own copy, and continue by name alone.
    let restaged = application
        .update(&workspace, &spec(&next_payload, &next_manifest), false)
        .unwrap();
    assert!(restaged.resolution_staged);
    write_bytes(
        &workspace,
        &format!(".truss/core/addon-update/{ADDON}/resolved/{SUBJECT}"),
        b"resolved\n",
    );
    let continued = application.continue_update(&workspace, &add_on()).unwrap();
    assert!(continued.applied && !continued.resolution_staged);
    assert_eq!(continued.source_ref, None);
    assert_eq!(fs::read(workspace.join(SUBJECT)).unwrap(), b"resolved\n");
    assert!(
        !application
            .status(&workspace, &add_on())
            .unwrap()
            .session_pending
    );
    assert_eq!(
        fs::read(state_root.join("update/session.json")).unwrap(),
        core_session
    );

    // The new baseline is the payload bytes, never the resolved workspace bytes.
    let recorded = FileSystemAddOnState.load(&workspace).unwrap().unwrap();
    let installation = recorded.installation(&add_on()).unwrap();
    assert_eq!(installation.files[0].content, b"next overlap\n");
    assert_eq!(installation.source_ref.as_str(), SOURCE_REF);

    evidence(
        "s4b2-real-adapters.txt",
        &format!(
            "install_applied={}\nstatus_recorded={}\nstatus_pending_with_core_session={}\ndry_run_snapshot_before={}\ndry_run_snapshot_after={}\ndry_run_equal={}\nstaged={}\nabort_first={}\nabort_second={}\ncontinued={}\nworkspace_subject={:?}\nbaseline_subject={:?}\ncore_session_untouched={}\n",
            installed.applied,
            status.record.is_some(),
            status.session_pending,
            before_digest,
            snapshot_digest(&after_dry_run),
            after_dry_run == before,
            staged.resolution_staged,
            true,
            false,
            continued.applied,
            String::from_utf8_lossy(&fs::read(workspace.join(SUBJECT)).unwrap()),
            String::from_utf8_lossy(&installation.files[0].content),
            fs::read(state_root.join("update/session.json")).unwrap() == core_session,
        ),
    );
}
