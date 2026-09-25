use std::path::Path;

use super::{
    AddOnApplyRequest, AddOnExecutionPort, AddOnInstallRequest, AddOnPayloadPort, AddOnPayloadSpec,
    AddOnPlanPort, AddOnPlanRequest, AddOnStageRequest, AddOnStatePort, PortError,
};
use crate::domain::{
    AddOnDescriptor, AddOnInstallation, AddOnName, AddOnPayloadFile, PlannedFileChange, SourceRef,
    UpdateConflict,
};

/// Add-on use cases over the four add-on ports.
///
/// The facade is orchestration only: it asks the payload port for the
/// descriptor, the planner port for a plan, and the execution port to apply,
/// stage, resume, or abort, then maps the results into report values. It reads
/// no file, opens no session, holds no lock, and owns no classification, path,
/// apply, or session rule; each of those stays with the port that already
/// implements it.
pub struct AddOnApplication<P, S, L, E> {
    payload: P,
    state: S,
    planner: L,
    executor: E,
}

impl<P, S, L, E> AddOnApplication<P, S, L, E>
where
    P: AddOnPayloadPort,
    S: AddOnStatePort,
    L: AddOnPlanPort,
    E: AddOnExecutionPort,
{
    pub fn new(payload: P, state: S, planner: L, executor: E) -> Self {
        Self {
            payload,
            state,
            planner,
            executor,
        }
    }

    /// Report one add-on's recorded provenance and pending session.
    ///
    /// The record comes from the add-on state port (`<state-root>/addons.json`)
    /// and the pending flag from the add-on execution port
    /// (`<state-root>/addon-update/<name>/`). The core state port's
    /// `resolution_pending`, which reads `<state-root>/update/`, is never
    /// consulted, so a pending core session cannot be reported as an add-on
    /// session.
    pub fn status(&self, root: &Path, name: &AddOnName) -> Result<AddOnStatusReport, PortError> {
        let record = self
            .state
            .load(root)?
            .and_then(|state| state.installation(name).map(AddOnRecordStatus::recorded));
        let session_pending = self.executor.session_pending(root, name)?;
        Ok(AddOnStatusReport {
            name: name.clone(),
            record,
            session_pending,
        })
    }

    /// Install one add-on payload, or preview it when `dry_run`.
    ///
    /// The descriptor is built by the payload port, so the payload path set and
    /// every digest come from the payload bytes and the membership manifest,
    /// and the recorded-ownership guard refuses a path the core manifest or
    /// another recorded add-on already owns before anything is planned or
    /// applied. A dry run stops after describing and guarding: nothing is
    /// written. A real install
    /// delegates creation or exact adoption to the state port, which writes the
    /// managed files, the baseline, and the provenance record and refuses a
    /// workspace that does not match the payload exactly.
    pub fn install(
        &self,
        root: &Path,
        payload_spec: &AddOnPayloadSpec<'_>,
        dry_run: bool,
    ) -> Result<AddOnUpdateReport, PortError> {
        self.state.resolve_state_root(root)?;
        let descriptor = self.payload.describe(payload_spec)?;
        self.reject_recorded_ownership(root, &descriptor)?;
        let mut report = AddOnUpdateReport::preview(&descriptor, dry_run);
        if !dry_run {
            let receipt = self.state.apply(
                root,
                &AddOnInstallRequest {
                    descriptor: &descriptor,
                    payload_root: payload_spec.root,
                },
            )?;
            report.applied = true;
            report.adopted = Some(receipt.adopted);
        }
        Ok(report)
    }

    /// Preview or apply one add-on update against the recorded baseline.
    ///
    /// A dry run builds the descriptor and the plan and returns immediately:
    /// nothing is applied and no session is staged. The recorded-ownership
    /// guard runs before the planner, so a descriptor path owned by the core
    /// manifest or by another recorded add-on is refused before any planning.
    /// A real update applies only
    /// a conflict-free plan through the execution port. A plan carrying any
    /// conflict is never applied, not even its clean subset: the same port
    /// stages the conflicted plan under the owned add-on session instead, and
    /// refuses a conflict it cannot stage.
    pub fn update(
        &self,
        root: &Path,
        payload_spec: &AddOnPayloadSpec<'_>,
        dry_run: bool,
    ) -> Result<AddOnUpdateReport, PortError> {
        self.state.resolve_state_root(root)?;
        let descriptor = self.payload.describe(payload_spec)?;
        self.reject_recorded_ownership(root, &descriptor)?;
        let plan = self.planner.plan(
            root,
            &AddOnPlanRequest {
                descriptor: &descriptor,
                payload_root: payload_spec.root,
            },
        )?;
        let mut report = AddOnUpdateReport::preview(&descriptor, dry_run);
        report.changes = plan.changes.clone();
        report.conflicts = plan.conflicts.clone();
        if dry_run {
            return Ok(report);
        }
        if plan.conflicts.is_empty() {
            let receipt = self.executor.apply(
                root,
                &AddOnApplyRequest {
                    descriptor: &descriptor,
                    payload_root: payload_spec.root,
                    plan: &plan,
                },
            )?;
            report.applied = true;
            report.backup_path = receipt.backup_path;
        } else {
            self.executor.stage(
                root,
                &AddOnStageRequest {
                    descriptor: &descriptor,
                    payload_root: payload_spec.root,
                    plan: &plan,
                },
            )?;
            report.resolution_staged = true;
        }
        Ok(report)
    }

    /// Apply a staged add-on conflict session by add-on name alone.
    ///
    /// The payload is deliberately absent: the session is self-contained, so
    /// the execution port reads the candidate payload, the descriptor identity,
    /// the materialised plan, and the operator-edited resolutions out of it.
    /// The planner is never consulted, because resume applies a frozen decision
    /// instead of re-classifying it.
    pub fn continue_update(
        &self,
        root: &Path,
        name: &AddOnName,
    ) -> Result<AddOnUpdateReport, PortError> {
        self.state.resolve_state_root(root)?;
        let receipt = self.executor.resume(root, name)?;
        let mut report = AddOnUpdateReport::for_name(name);
        report.applied = true;
        report.backup_path = receipt.backup_path;
        Ok(report)
    }

    /// Remove only the owned add-on conflict session for `name`.
    pub fn abort(&self, root: &Path, name: &AddOnName) -> Result<bool, PortError> {
        self.state.resolve_state_root(root)?;
        self.executor.abort(root, name)
    }

    /// Refuse a descriptor that declares a path another managed distribution
    /// already owns, before anything is planned or applied.
    ///
    /// The ownership set is the workspace's own recorded state, read through
    /// the add-on state port: the core `<state-root>/manifest.json` entry list
    /// and the recorded paths of every other add-on in
    /// `<state-root>/addons.json`. No caller supplies it and no command-line flag
    /// exists for it, so an empty caller-supplied list can never disable the
    /// check, and an incomplete core state is a refusal rather than an empty
    /// ownership set. The guard runs on dry runs too, so a preview cannot
    /// report a collision as applicable.
    fn reject_recorded_ownership(
        &self,
        root: &Path,
        descriptor: &AddOnDescriptor,
    ) -> Result<(), PortError> {
        let owners = self.state.recorded_owners(root, &descriptor.name)?;
        descriptor
            .reject_owned_paths(&owners)
            .map_err(|error| PortError::new(error.to_string()))
    }
}

/// What one add-on operation observed and did.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddOnUpdateReport {
    /// Add-on the operation names.
    pub name: AddOnName,
    /// Immutable payload ref the operation targeted. `None` for `continue`,
    /// which takes the identity out of the staged session instead of a
    /// caller-supplied payload.
    pub source_ref: Option<SourceRef>,
    /// True when the caller asked for a preview.
    pub dry_run: bool,
    /// True when the operation reached a mutation (`apply` or `resume`).
    pub applied: bool,
    /// `install`'s receipt: true when every managed path already matched the
    /// payload exactly, `None` when no install ran.
    pub adopted: Option<bool>,
    /// True when a conflicted plan was staged for `continue` instead of being
    /// applied.
    pub resolution_staged: bool,
    /// Classified changes. Empty for `install` (the state port owns its own
    /// create-or-adopt classification) and for `continue` (the frozen plan is
    /// applied by the execution port).
    pub changes: Vec<PlannedFileChange>,
    /// Classified conflicts, reported whether or not they were staged.
    pub conflicts: Vec<UpdateConflict>,
    /// Backup directory the transaction writer reported, when it wrote one.
    pub backup_path: Option<String>,
}

impl AddOnUpdateReport {
    fn preview(descriptor: &AddOnDescriptor, dry_run: bool) -> Self {
        Self {
            name: descriptor.name.clone(),
            source_ref: Some(descriptor.source_ref.clone()),
            dry_run,
            applied: false,
            adopted: None,
            resolution_staged: false,
            changes: Vec::new(),
            conflicts: Vec::new(),
            backup_path: None,
        }
    }

    fn for_name(name: &AddOnName) -> Self {
        Self {
            name: name.clone(),
            source_ref: None,
            dry_run: false,
            applied: false,
            adopted: None,
            resolution_staged: false,
            changes: Vec::new(),
            conflicts: Vec::new(),
            backup_path: None,
        }
    }
}

/// One add-on's status as the add-on state port reports it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddOnStatusReport {
    /// Add-on whose status was requested.
    pub name: AddOnName,
    /// Recorded installation, or `None` when `<state-root>/addons.json` holds no
    /// record for this add-on.
    pub record: Option<AddOnRecordStatus>,
    /// True when an add-on conflict session is pending under
    /// `<state-root>/addon-update/<name>/`.
    pub session_pending: bool,
}

/// The recorded provenance of one installed add-on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddOnRecordStatus {
    /// Immutable ref the payload was installed from.
    pub source_ref: SourceRef,
    /// Core version the payload was acquired with.
    pub source_core_version: String,
    /// Ordered payload path set with the recorded payload digest per path.
    pub files: Vec<AddOnPayloadFile>,
}

impl AddOnRecordStatus {
    fn recorded(installation: &AddOnInstallation) -> Self {
        Self {
            source_ref: installation.source_ref.clone(),
            source_core_version: installation.source_core_version.clone(),
            files: installation
                .files
                .iter()
                .map(|file| AddOnPayloadFile {
                    path: file.path.clone(),
                    sha256: file.hash.clone(),
                })
                .collect(),
        }
    }
}
