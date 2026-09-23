use std::collections::BTreeMap;

use super::{
    FrozenWorkspaceFile, PlannedFileChange, RelativePath, ResolutionConflict, UpdateConflict,
    WorkspaceMutation,
};

/// Neutral inputs to the update planner.
///
/// Every field is bytes keyed by path, so the same classification can serve
/// the core update path and an add-on payload without either one's provenance
/// types: `baselines` are the installed (or add-on baseline) bytes per path,
/// `upstream` the incoming bytes per path, `local` the workspace bytes present
/// for a path (a missing key means the path is absent locally), and
/// `resolutions` an optional per-path human-resolved byte string.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UpdatePlanInput {
    pub baselines: BTreeMap<RelativePath, Vec<u8>>,
    pub upstream: BTreeMap<RelativePath, Vec<u8>>,
    pub local: BTreeMap<RelativePath, Vec<u8>>,
    pub resolutions: BTreeMap<RelativePath, Vec<u8>>,
}

/// The classified comparison of every path in the baseline/upstream union.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UpdatePlan {
    pub changes: Vec<PlannedFileChange>,
    pub conflicts: Vec<UpdateConflict>,
    pub resolution_conflicts: Vec<ResolutionConflict>,
    pub mutations: Vec<WorkspaceMutation>,
    pub frozen_files: Vec<FrozenWorkspaceFile>,
}
