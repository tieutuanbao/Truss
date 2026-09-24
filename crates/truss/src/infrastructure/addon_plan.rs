use std::collections::BTreeSet;
use std::path::Path;

use fs2::FileExt;

use super::addon_payload::read_declared_file;
use super::state_io::{
    acquire_existing_lock, io_error, state_root, validate_core_state, validate_workspace_root,
};
use super::{FileSystemAddOnState, FileSystemInstallationState, GitThreeWayMerge};
use crate::application::{
    plan_update, AddOnPlanRequest, AddOnStatePort, ApplicationError, InstallationStatePort,
};
use crate::domain::{UpdatePlan, UpdatePlanInput};

/// Dry-run add-on planner adapter.
///
/// It translates recorded add-on state (`.truss-core/addons.json` plus the
/// digest-checked baselines under `.truss-core/base-addons/<name>/`), the
/// staged S1 payload, and the consumer workspace into the neutral
/// [`UpdatePlanInput`] and returns whatever the extracted planner classifies.
/// It applies nothing: no managed file, baseline, or provenance byte changes,
/// and no conflict session is staged.
///
/// Decision 0003 clause 8 applies unchanged: a valid pre-existing core state is
/// required and is only ever validated, never created or repaired, and the
/// authoritative state load, workspace observation, and planning run while
/// holding the existing shared lock.
#[derive(Clone, Copy, Default)]
pub struct FileSystemAddOnPlanner;

impl FileSystemAddOnPlanner {
    /// Plan one named add-on payload against the workspace. Read-only.
    pub fn plan(
        &self,
        root: &Path,
        request: &AddOnPlanRequest<'_>,
    ) -> Result<UpdatePlan, ApplicationError> {
        validate_workspace_root(root)?;
        let state_root = state_root(root);
        validate_core_state(&state_root)?;
        // The lock is opened without `create`, so a missing lock is a refusal
        // rather than a reason to bootstrap core state.
        let lock = acquire_existing_lock(&state_root)?;
        let result = plan_locked(root, request);
        FileExt::unlock(&lock).map_err(io_error)?;
        result
    }
}

fn plan_locked(
    root: &Path,
    request: &AddOnPlanRequest<'_>,
) -> Result<UpdatePlan, ApplicationError> {
    let input = plan_input(root, request)?;
    classify(root, &input)
}

/// Classify one prepared add-on planner input with the accepted merge owner.
///
/// The resume path prepares the same input, adds human resolutions, and calls
/// this, so both planning paths share exactly one classification and one merge
/// implementation.
pub(crate) fn classify(
    root: &Path,
    input: &UpdatePlanInput,
) -> Result<UpdatePlan, ApplicationError> {
    plan_update(input, &GitThreeWayMerge, |path| {
        FileSystemInstallationState.validate_managed_path(root, path)
    })
}

/// Build the neutral planner input from the recorded add-on baseline, the
/// staged payload, and the current workspace, without classifying anything.
///
/// `FileSystemAddOnPlanner::plan` classifies this input directly; the conflict
/// resume path adds `resolutions` to the same input and reuses the same
/// classification, so there is exactly one add-on planning implementation.
pub(crate) fn plan_input(
    root: &Path,
    request: &AddOnPlanRequest<'_>,
) -> Result<UpdatePlanInput, ApplicationError> {
    let descriptor = request.descriptor;
    descriptor.validate()?;

    let mut input = UpdatePlanInput::default();
    // Baselines are the recorded payload bytes, read from
    // `.truss-core/base-addons/<name>/` and digest-checked against
    // `addons.json` by the state port. An add-on with no record contributes no
    // baselines, so every declared path plans as a fresh create.
    if let Some(state) = FileSystemAddOnState.load(root)? {
        if let Some(installation) = state.installation(&descriptor.name) {
            for file in &installation.files {
                input
                    .baselines
                    .insert(file.path.clone(), file.content.clone());
            }
        }
    }
    for file in &descriptor.files {
        let bytes = read_declared_file(request.payload_root, &descriptor.name, file)?;
        input.upstream.insert(file.path.clone(), bytes);
    }

    let paths = input
        .baselines
        .keys()
        .chain(input.upstream.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for path in paths {
        // An unsafe path is left to the planner so it stays an `UnsafePath`
        // conflict, exactly as the core path does; it is never read, so it is
        // never frozen.
        if FileSystemInstallationState
            .validate_managed_path(root, &path)
            .is_err()
        {
            continue;
        }
        if let Some(content) = FileSystemInstallationState.read_workspace_file(root, &path)? {
            input.local.insert(path, content);
        }
    }

    Ok(input)
}
