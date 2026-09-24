use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use fs2::FileExt;

use super::addon_payload::read_declared_file;
use super::addon_plan::{classify, plan_input};
use super::addon_session::{clear_addon_session, load_addon_session, stage_addon_session};
use super::addon_state::{
    base_addons_root, publish_baseline, write_addons_record, FileSystemAddOnState, ADDONS_FILE,
    BASE_ADDONS_DIR,
};
use super::filesystem_state::verify_frozen_locked;
use super::state_io::{
    acquire_existing_lock, copy_file, copy_tree, io_error, state_root, validate_core_state,
    validate_workspace_root,
};
use super::transaction::{self, ProvenanceKind, ProvenanceWriter};
use crate::application::{
    AddOnApplyRequest, AddOnPlanRequest, AddOnResumeRequest, AddOnStageRequest, AddOnStatePort,
    ApplicationError, PortError,
};
use crate::domain::{
    AddOnInstallation, AddOnName, AddOnState, ApplyReceipt, BaselineFile, UpdateResolutionSession,
};

/// Transactional add-on apply adapter.
///
/// It consumes a conflict-free S3b [`crate::domain::UpdatePlan`], stages every
/// workspace mutation, publishes the payload baseline under
/// `.truss-core/base-addons/<name>/`, and writes `.truss-core/addons.json`
/// last. It reuses the shared journal-and-backup engine, so a failure at any
/// point restores the workspace, the baseline, and `addons.json` before the
/// error is returned.
///
/// A plan carrying any conflict is refused before the lock is taken, with no
/// mutation at all. A valid pre-existing core state is required and is only
/// ever validated, never created or repaired, and the existing shared lock is
/// reused rather than created (decision 0003 clause 8). No conflict session is
/// staged, resumed, or cleared, and `.truss-core/update/` is never touched.
#[derive(Clone, Copy, Default)]
pub struct FileSystemAddOnApplier;

impl FileSystemAddOnApplier {
    /// Apply one conflict-free add-on plan to the workspace.
    pub fn apply(
        &self,
        root: &Path,
        request: &AddOnApplyRequest<'_>,
    ) -> Result<ApplyReceipt, ApplicationError> {
        validate_workspace_root(root)?;
        let state_root = state_root(root);
        validate_core_state(&state_root)?;
        if !request.plan.conflicts.is_empty() || !request.plan.resolution_conflicts.is_empty() {
            return Err(PortError::new(format!(
                "refusing to apply an add-on plan carrying {} conflict(s) and {} resolution conflict(s)",
                request.plan.conflicts.len(),
                request.plan.resolution_conflicts.len()
            ))
            .into());
        }
        // The lock is opened without `create`, so a missing lock is a refusal
        // rather than a reason to bootstrap core state.
        let lock = acquire_existing_lock(&state_root)?;
        let result = apply_locked(root, &state_root, request);
        FileExt::unlock(&lock).map_err(io_error)?;
        result
    }

    /// Stage one conflicted add-on plan under the owned add-on namespace.
    ///
    /// No managed file, baseline byte, or `addons.json` byte changes: only
    /// `.truss-core/addon-update/<name>/` is written. The complete frozen
    /// workspace observation is re-checked under the existing shared lock, so a
    /// plan that went stale since the S3b planning lock is refused instead of
    /// being persisted. `.truss-core/update/` is never read or written.
    pub fn stage(
        &self,
        root: &Path,
        request: &AddOnStageRequest<'_>,
    ) -> Result<(), ApplicationError> {
        validate_workspace_root(root)?;
        let state_root = state_root(root);
        validate_core_state(&state_root)?;
        let lock = acquire_existing_lock(&state_root)?;
        let result = stage_locked(root, &state_root, request);
        FileExt::unlock(&lock).map_err(io_error)?;
        result
    }

    /// Resume one staged add-on conflict session.
    ///
    /// Every frozen observation in the session is re-checked against the
    /// workspace under the shared lock, not only the conflicted path, and the
    /// resolved plan is applied through the same [`Self::apply`] engine, which
    /// writes provenance last. On success only the owned add-on session is
    /// cleared. A competing change to any frozen managed path refuses before
    /// mutation, leaving the competing bytes intact.
    pub fn resume(
        &self,
        root: &Path,
        request: &AddOnResumeRequest<'_>,
    ) -> Result<ApplyReceipt, ApplicationError> {
        validate_workspace_root(root)?;
        let state_root = state_root(root);
        validate_core_state(&state_root)?;
        let lock = acquire_existing_lock(&state_root)?;
        let result = resume_locked(root, &state_root, request);
        FileExt::unlock(&lock).map_err(io_error)?;
        result
    }

    /// Remove only the owned add-on session, and nothing else.
    ///
    /// The workspace, the baseline, `addons.json`, and any core session are
    /// untouched, and a repeat call removes nothing and returns `false`.
    pub fn abort(&self, root: &Path, name: &AddOnName) -> Result<bool, ApplicationError> {
        validate_workspace_root(root)?;
        let state_root = state_root(root);
        validate_core_state(&state_root)?;
        let lock = acquire_existing_lock(&state_root)?;
        let result = clear_addon_session(&state_root, name).map_err(Into::into);
        FileExt::unlock(&lock).map_err(io_error)?;
        result
    }
}

fn stage_locked(
    root: &Path,
    state_root: &Path,
    request: &AddOnStageRequest<'_>,
) -> Result<(), ApplicationError> {
    let descriptor = request.descriptor;
    descriptor.validate()?;
    transaction::recover(root, state_root)?;
    let existing = FileSystemAddOnState.load(root)?.ok_or_else(|| {
        PortError::new(format!(
            "no installed add-on record at .truss-core/addons.json; install {} before updating it",
            descriptor.name
        ))
    })?;
    let installation = existing.installation(&descriptor.name).ok_or_else(|| {
        PortError::new(format!(
            "add-on {} is not recorded; install it before updating",
            descriptor.name
        ))
    })?;
    let plan = request.plan;
    if plan.resolution_conflicts.is_empty() {
        return Err(PortError::new(format!(
            "refusing to stage an add-on update for {} with no resolvable conflict",
            descriptor.name
        ))
        .into());
    }
    if plan.conflicts.len() != plan.resolution_conflicts.len() {
        return Err(PortError::new(format!(
            "refusing to stage an add-on update for {}: every conflict must carry resolution inputs",
            descriptor.name
        ))
        .into());
    }
    // The plan was observed under the S3b planner lock, which has since been
    // released. Re-check every frozen observation under this lock so a stale
    // plan is refused rather than persisted.
    verify_frozen_locked(root, &plan.frozen_files)?;
    let session = UpdateResolutionSession {
        from_version: installation.source_ref.as_str().to_owned(),
        to_version: descriptor.source_ref.as_str().to_owned(),
        conflicts: plan.resolution_conflicts.clone(),
        frozen_files: plan.frozen_files.clone(),
    };
    stage_addon_session(state_root, &descriptor.name, &session)?;
    Ok(())
}

fn resume_locked(
    root: &Path,
    state_root: &Path,
    request: &AddOnResumeRequest<'_>,
) -> Result<ApplyReceipt, ApplicationError> {
    let descriptor = request.descriptor;
    descriptor.validate()?;
    transaction::recover(root, state_root)?;
    let session = load_addon_session(state_root, &descriptor.name)?.ok_or_else(|| {
        PortError::new(format!(
            "no add-on conflict session is pending for {}",
            descriptor.name
        ))
    })?;
    let existing = FileSystemAddOnState.load(root)?.ok_or_else(|| {
        PortError::new(format!(
            "no installed add-on record at .truss-core/addons.json; install {} before updating it",
            descriptor.name
        ))
    })?;
    let installation = existing.installation(&descriptor.name).ok_or_else(|| {
        PortError::new(format!(
            "add-on {} is not recorded; install it before updating",
            descriptor.name
        ))
    })?;
    if session.from_version != installation.source_ref.as_str()
        || session.to_version != descriptor.source_ref.as_str()
    {
        return Err(PortError::new(format!(
            "staged add-on session for {} no longer matches the recorded refs (recorded={}, session={}->{}, candidate={}); abort and restart the update",
            descriptor.name,
            installation.source_ref.as_str(),
            session.from_version,
            session.to_version,
            descriptor.source_ref.as_str(),
        ))
        .into());
    }

    // Rebuild the same neutral input the S3b planner uses, so the resolution
    // classification shares one implementation.
    let mut input = plan_input(
        root,
        &AddOnPlanRequest {
            descriptor,
            payload_root: request.payload_root,
        },
    )?;
    let expected_paths = input
        .baselines
        .keys()
        .chain(input.upstream.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let frozen_paths = session
        .frozen_files
        .iter()
        .map(|frozen| frozen.path.clone())
        .collect::<BTreeSet<_>>();
    if frozen_paths.len() != session.frozen_files.len() || frozen_paths != expected_paths {
        return Err(PortError::new(format!(
            "staged add-on plan for {} no longer matches the recorded/candidate managed paths; abort and restart the update",
            descriptor.name
        ))
        .into());
    }
    // Validate every frozen observation, conflicted or not, against the current
    // workspace under the lock that also commits; a competing change to any
    // managed path is refused before any mutation.
    verify_frozen_locked(root, &session.frozen_files)?;

    let mut resolutions = BTreeMap::new();
    for conflict in &session.conflicts {
        let current = input.local.get(&conflict.path);
        let base_matches = input
            .baselines
            .get(&conflict.path)
            .is_some_and(|base| base == &conflict.base);
        let incoming_matches = input
            .upstream
            .get(&conflict.path)
            .is_some_and(|incoming| incoming == &conflict.incoming);
        if current.map(Vec::as_slice) != Some(conflict.local.as_slice())
            || !base_matches
            || !incoming_matches
        {
            return Err(PortError::new(format!(
                "workspace or recorded inputs changed after conflict detection for {}; abort and restart the update",
                conflict.path
            ))
            .into());
        }
        if contains_conflict_markers(&conflict.resolved) {
            return Err(PortError::new(format!(
                "resolution still contains conflict markers for {}",
                conflict.path
            ))
            .into());
        }
        resolutions.insert(conflict.path.clone(), conflict.resolved.clone());
    }
    input.resolutions = resolutions;
    let resolved_plan = classify(root, &input)?;
    if !resolved_plan.conflicts.is_empty() || !resolved_plan.resolution_conflicts.is_empty() {
        return Err(PortError::new(format!(
            "refusing to resume an add-on update for {} that still plans {} conflict(s)",
            descriptor.name,
            resolved_plan.conflicts.len()
        ))
        .into());
    }
    let apply_request = AddOnApplyRequest {
        descriptor,
        payload_root: request.payload_root,
        plan: &resolved_plan,
    };
    let receipt = apply_locked(root, state_root, &apply_request)?;
    // Only this add-on's session is cleared, and only after provenance is
    // written.
    clear_addon_session(state_root, &descriptor.name)?;
    Ok(receipt)
}

fn contains_conflict_markers(content: &[u8]) -> bool {
    content.split(|byte| *byte == b'\n').any(|line| {
        line.starts_with(b"<<<<<<< ")
            || line.starts_with(b"||||||| ")
            || line == b"======="
            || line.starts_with(b">>>>>>> ")
    })
}

fn apply_locked(
    root: &Path,
    state_root: &Path,
    request: &AddOnApplyRequest<'_>,
) -> Result<ApplyReceipt, ApplicationError> {
    request.descriptor.validate()?;
    // Clear any interrupted transaction using the same journal the core writer
    // uses, then read the recorded state and the payload bytes under the lock.
    transaction::recover(root, state_root)?;
    let existing = FileSystemAddOnState.load(root)?.ok_or_else(|| {
        PortError::new(format!(
            "no installed add-on record at .truss-core/addons.json; install {} before updating it",
            request.descriptor.name
        ))
    })?;
    if existing.installation(&request.descriptor.name).is_none() {
        return Err(PortError::new(format!(
            "add-on {} is not recorded; install it before updating",
            request.descriptor.name
        ))
        .into());
    }
    let installation = build_installation(request)?;
    let mut state = existing;
    state.upsert(installation.clone());
    state.validate()?;
    let writer = AddOnProvenanceWriter {
        #[cfg(test)]
        root,
        state_root,
        installation: &installation,
        state: &state,
    };
    // The S3b plan was observed under a lock that has since been released. The
    // frozen set is the planner's complete per-path view of the validated
    // union, so re-checking every one of them under this lock joins the
    // planning observation and the commit into one authoritative section: a
    // competing writer that changed a managed path in between is refused here
    // instead of being overwritten by the stale plan.
    verify_frozen_locked(root, &request.plan.frozen_files)?;
    transaction::run(root, state_root, &request.plan.mutations, &writer).map_err(Into::into)
}

/// Build the new baseline record from the payload bytes, never from the
/// workspace: the digest and the bytes both come from the descriptor's payload.
fn build_installation(
    request: &AddOnApplyRequest<'_>,
) -> Result<AddOnInstallation, ApplicationError> {
    let descriptor = request.descriptor;
    let mut files = Vec::with_capacity(descriptor.files.len());
    for file in &descriptor.files {
        let content = read_declared_file(request.payload_root, &descriptor.name, file)?;
        files.push(BaselineFile {
            path: file.path.clone(),
            content,
            hash: file.sha256.clone(),
        });
    }
    let installation = AddOnInstallation {
        name: descriptor.name.clone(),
        source_ref: descriptor.source_ref.clone(),
        source_core_version: descriptor.source_core_version.clone(),
        files,
    };
    installation.validate()?;
    Ok(installation)
}

/// The add-on state half of a transaction: `.truss-core/addons.json` plus the
/// `.truss-core/base-addons/<name>/` baseline tree.
struct AddOnProvenanceWriter<'a> {
    #[cfg(test)]
    root: &'a Path,
    state_root: &'a Path,
    installation: &'a AddOnInstallation,
    state: &'a AddOnState,
}

impl ProvenanceWriter for AddOnProvenanceWriter<'_> {
    fn kind(&self) -> ProvenanceKind {
        ProvenanceKind::AddOn {
            name: self.installation.name.as_str().to_owned(),
        }
    }

    fn exists(&self) -> bool {
        self.state_root.join(ADDONS_FILE).exists()
            || base_addons_root(self.state_root)
                .join(self.installation.name.as_str())
                .exists()
    }

    fn backup(&self, target: &Path) -> Result<(), PortError> {
        fs::create_dir_all(target).map_err(io_error)?;
        let addons = self.state_root.join(ADDONS_FILE);
        if addons.exists() {
            copy_file(&addons, &target.join(ADDONS_FILE))?;
        }
        let baseline = base_addons_root(self.state_root).join(self.installation.name.as_str());
        if baseline.exists() {
            copy_tree(
                &baseline,
                &target
                    .join(BASE_ADDONS_DIR)
                    .join(self.installation.name.as_str()),
            )?;
        }
        Ok(())
    }

    fn write(&self, id: &str) -> Result<(), PortError> {
        publish_baseline(self.state_root, self.installation, id)?;
        // Acceptance row 2's second injection point: the provenance record is
        // about to be written, with the workspace and the baseline already in
        // place.
        #[cfg(test)]
        super::transaction::faults::trigger(
            self.root,
            super::transaction::faults::InjectionPoint::BeforeProvenanceWrite,
        )?;
        write_addons_record(self.state_root, self.state, id)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use sha2::{Digest, Sha256};

    use super::super::addon_payload::FileSystemAddOnPayload;
    use super::super::addon_plan::FileSystemAddOnPlanner;
    use super::super::addon_state::FileSystemAddOnState;
    use super::super::transaction::faults::{self, InjectionPoint};
    use super::{AddOnApplyRequest, FileSystemAddOnApplier};
    use crate::application::{
        AddOnInstallRequest, AddOnPayloadPort, AddOnPayloadSpec, AddOnPlanRequest,
        AddOnResumeRequest, AddOnStageRequest, AddOnStatePort,
    };
    use crate::domain::AddOnDescriptor;

    const ADDON: &str = "demo";
    const OLD_REF: &str = "truss-v0.1.13";
    const NEW_REF: &str = "truss-v0.1.14";
    const SUBJECT: &str = ".agents/skills/demo/SKILL.md";
    const NEW_PATH: &str = ".agents/skills/demo/new.md";
    const CORE_STATE_IGNORE: &str =
        "/lock\n/transaction.json\n/base.next-*\n/update/\n/update-candidate/\n/addon-update/\n";

    fn write_bytes(root: &Path, relative: &str, content: &[u8]) {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, content).unwrap();
    }

    fn seed_core_state(workspace: &Path) {
        let state_root = workspace.join(".truss-core");
        fs::create_dir_all(&state_root).unwrap();
        fs::write(state_root.join(".gitignore"), CORE_STATE_IGNORE).unwrap();
        fs::write(state_root.join("lock"), b"").unwrap();
    }

    fn manifest(files: &[&str]) -> String {
        let mut text = String::from("# add-on apply fixture\n");
        for path in files {
            text.push_str(path);
            text.push('\n');
        }
        text
    }

    fn describe(payload: &Path, manifest: &Path, source_ref: &str) -> AddOnDescriptor {
        let foreign: [PathBuf; 0] = [];
        FileSystemAddOnPayload
            .describe(&AddOnPayloadSpec {
                root: payload,
                manifest,
                name: ADDON,
                source_ref,
                source_core_version: "0.1.13",
                foreign_manifests: &foreign,
            })
            .unwrap()
    }

    /// Complete path/type/content snapshot, so a leftover backup directory or a
    /// partial write is visible.
    fn snapshot(root: &Path) -> String {
        let mut lines = Vec::new();
        walk(root, root, &mut lines);
        lines.sort();
        lines.join("\n")
    }

    fn walk(root: &Path, directory: &Path, lines: &mut Vec<String>) {
        let mut entries = fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                lines.push(format!("dir {relative}"));
                walk(root, &path, lines);
            } else if metadata.is_file() {
                let bytes = fs::read(&path).unwrap();
                lines.push(format!("file {relative} {:x}", Sha256::digest(&bytes)));
            } else {
                lines.push(format!("other {relative}"));
            }
        }
    }

    /// Acceptance row 2: the deterministic injection points after the first
    /// staged mutation and immediately before the provenance record each leave
    /// the complete workspace, the baseline tree, and `addons.json`
    /// byte-identical to the pre-apply snapshot, and provenance is written
    /// last on the success path.
    #[test]
    fn injected_failure_leaves_workspace_baseline_and_record_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let baseline_payload = tmp.path().join("baseline-payload");
        let next_payload = tmp.path().join("next-payload");
        let baseline_manifest = tmp.path().join("baseline-files.txt");
        let next_manifest = tmp.path().join("next-files.txt");

        write_bytes(&baseline_payload, SUBJECT, b"base\n");
        fs::write(&baseline_manifest, manifest(&[SUBJECT])).unwrap();
        seed_core_state(&workspace);
        let old = describe(&baseline_payload, &baseline_manifest, OLD_REF);
        FileSystemAddOnState
            .apply(
                &workspace,
                &AddOnInstallRequest {
                    descriptor: &old,
                    payload_root: &baseline_payload,
                },
            )
            .unwrap();

        write_bytes(&next_payload, SUBJECT, b"next\n");
        write_bytes(&next_payload, NEW_PATH, b"new\n");
        fs::write(&next_manifest, manifest(&[SUBJECT, NEW_PATH])).unwrap();
        let next = describe(&next_payload, &next_manifest, NEW_REF);
        let plan = FileSystemAddOnPlanner
            .plan(
                &workspace,
                &AddOnPlanRequest {
                    descriptor: &next,
                    payload_root: &next_payload,
                },
            )
            .unwrap();
        assert!(plan.conflicts.is_empty());
        assert!(
            plan.mutations.len() >= 2,
            "the fixture must stage more than one mutation"
        );

        let addons = workspace.join(".truss-core/addons.json");
        let baseline_root = workspace.join(".truss-core/base-addons");
        let before_workspace = snapshot(&workspace);
        let before_addons = fs::read(&addons).unwrap();
        let before_baseline = snapshot(&baseline_root);

        let mut evidence = String::new();
        for point in [
            InjectionPoint::AfterFirstMutation,
            InjectionPoint::BeforeProvenanceWrite,
        ] {
            let before_workspace_digest = bytes_digest(snapshot(&workspace).as_bytes());
            let before_baseline_digest = bytes_digest(snapshot(&baseline_root).as_bytes());
            let before_addons_digest = bytes_digest(&fs::read(&addons).unwrap());

            faults::arm(&workspace, point);
            let error = FileSystemAddOnApplier
                .apply(
                    &workspace,
                    &AddOnApplyRequest {
                        descriptor: &next,
                        payload_root: &next_payload,
                        plan: &plan,
                    },
                )
                .unwrap_err();
            faults::disarm(&workspace);

            let after_workspace = snapshot(&workspace);
            let after_addons = fs::read(&addons).unwrap();
            let after_baseline = snapshot(&baseline_root);

            assert!(
                error.to_string().contains("injected add-on apply failure"),
                "unexpected error at {point:?}: {error}"
            );
            assert_eq!(
                after_workspace, before_workspace,
                "workspace changed after the {point:?} injection"
            );
            assert_eq!(
                after_addons, before_addons,
                "addons.json changed after the {point:?} injection"
            );
            assert_eq!(
                after_baseline, before_baseline,
                "the baseline changed after the {point:?} injection"
            );

            evidence.push_str(&format!(
                "injection={point:?}\nbefore_workspace={before_workspace_digest}\nafter_workspace={}\nworkspace_equal={}\nbefore_baseline={before_baseline_digest}\nafter_baseline={}\nbaseline_equal={}\nbefore_addons={before_addons_digest}\nafter_addons={}\naddons_equal={}\n\n",
                bytes_digest(after_workspace.as_bytes()),
                after_workspace == before_workspace,
                bytes_digest(after_baseline.as_bytes()),
                after_baseline == before_baseline,
                bytes_digest(&after_addons),
                after_addons == before_addons,
            ));
        }
        write_evidence("s3c-row2-snapshots.txt", &evidence);
    }

    /// Acceptance row 2: the shared transaction engine writes provenance last,
    /// so a deterministic failure immediately before the `addons.json` write
    /// during a *resume* rolls back the workspace and the baseline and leaves
    /// the staged session intact. The complete workspace snapshot is compared,
    /// so a partial write, a leftover backup, or a cleared session is visible.
    #[test]
    fn resume_injected_failure_before_provenance_leaves_all_surfaces_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let baseline_payload = tmp.path().join("baseline-payload");
        let next_payload = tmp.path().join("next-payload");
        let baseline_manifest = tmp.path().join("baseline-files.txt");
        let next_manifest = tmp.path().join("next-files.txt");

        let base = b"one\ntwo\nthree\n";
        let local = b"local overlap\n";
        let next = b"next overlap\n";

        write_bytes(&baseline_payload, SUBJECT, base);
        fs::write(&baseline_manifest, manifest(&[SUBJECT])).unwrap();
        seed_core_state(&workspace);
        let old = describe(&baseline_payload, &baseline_manifest, OLD_REF);
        FileSystemAddOnState
            .apply(
                &workspace,
                &AddOnInstallRequest {
                    descriptor: &old,
                    payload_root: &baseline_payload,
                },
            )
            .unwrap();

        write_bytes(&workspace, SUBJECT, local);
        write_bytes(&next_payload, SUBJECT, next);
        write_bytes(&next_payload, NEW_PATH, b"new\n");
        fs::write(&next_manifest, manifest(&[SUBJECT, NEW_PATH])).unwrap();
        let next = describe(&next_payload, &next_manifest, NEW_REF);
        let plan = FileSystemAddOnPlanner
            .plan(
                &workspace,
                &AddOnPlanRequest {
                    descriptor: &next,
                    payload_root: &next_payload,
                },
            )
            .unwrap();
        assert_eq!(plan.conflicts.len(), 1, "the fixture must conflict");
        assert_eq!(plan.resolution_conflicts.len(), 1);

        FileSystemAddOnApplier
            .stage(
                &workspace,
                &AddOnStageRequest {
                    descriptor: &next,
                    plan: &plan,
                },
            )
            .unwrap();
        write_bytes(
            &workspace,
            &format!(".truss-core/addon-update/{ADDON}/resolved/{SUBJECT}"),
            b"resolved\n",
        );

        let session = workspace.join(".truss-core/addon-update").join(ADDON);
        let before = snapshot(&workspace);
        faults::arm(&workspace, InjectionPoint::BeforeProvenanceWrite);
        let error = FileSystemAddOnApplier
            .resume(
                &workspace,
                &AddOnResumeRequest {
                    descriptor: &next,
                    payload_root: &next_payload,
                },
            )
            .unwrap_err();
        faults::disarm(&workspace);
        let after = snapshot(&workspace);

        assert!(
            error.to_string().contains("injected add-on apply failure"),
            "unexpected error: {error}"
        );
        assert_eq!(
            after, before,
            "a failure before the provenance write must leave every surface unchanged"
        );
        assert_eq!(fs::read(workspace.join(SUBJECT)).unwrap(), local);
        assert!(!workspace.join(NEW_PATH).exists());
        assert!(
            session.join("session.json").is_file(),
            "the staged session must survive a refused resume"
        );
        write_s4_evidence(
            "s4-row2-provenance-last.txt",
            &format!(
                "error={error}\nbefore={}\nafter={}\nworkspace_equal={}\nsession_alive={}\n",
                bytes_digest(before.as_bytes()),
                bytes_digest(after.as_bytes()),
                before == after,
                session.join("session.json").is_file(),
            ),
        );
    }

    fn bytes_digest(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn write_evidence(name: &str, body: &str) {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target/s3-evidence");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(name), body).unwrap();
    }

    fn write_s4_evidence(name: &str, body: &str) {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("target/s4-evidence");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(name), body).unwrap();
    }
}
