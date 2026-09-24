use std::collections::BTreeMap;

use super::{
    FileChangeKind, FrozenWorkspaceFile, PlannedFileChange, RelativePath, ResolutionConflict,
    UpdateConflict, WorkspaceMutation,
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

impl UpdatePlan {
    /// Incorporate the operator's resolutions into a staged plan without
    /// re-planning.
    ///
    /// Every resolution conflict must carry a resolution for its path; a
    /// missing one returns `None`. A resolution that equals the observed local
    /// bytes is a `Preserve`, and any other resolution becomes one `Update`
    /// plus its write, mirroring the classifier's resolution rule. The
    /// returned plan carries no conflict and no resolution input, so it is
    /// exactly what an applier consumes: resume applies a frozen decision and
    /// never re-classifies.
    pub fn resolved(&self, resolutions: &BTreeMap<RelativePath, Vec<u8>>) -> Option<Self> {
        let mut plan = Self {
            changes: self.changes.clone(),
            conflicts: Vec::new(),
            resolution_conflicts: Vec::new(),
            mutations: self.mutations.clone(),
            frozen_files: self.frozen_files.clone(),
        };
        for conflict in &self.resolution_conflicts {
            let resolution = resolutions.get(&conflict.path)?;
            let kind = if resolution == &conflict.local {
                FileChangeKind::Preserve
            } else {
                plan.mutations.push(WorkspaceMutation::Write {
                    path: conflict.path.clone(),
                    content: resolution.clone(),
                });
                FileChangeKind::Update
            };
            plan.changes.push(PlannedFileChange {
                path: conflict.path.clone(),
                kind,
            });
        }
        Some(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(value: &str) -> RelativePath {
        RelativePath::parse(value).unwrap()
    }

    fn staged_plan() -> UpdatePlan {
        let target = path("a/b.md");
        UpdatePlan {
            resolution_conflicts: vec![ResolutionConflict {
                path: target.clone(),
                base: b"base\n".to_vec(),
                local: b"local\n".to_vec(),
                incoming: b"incoming\n".to_vec(),
                resolved: b"conflicted\n".to_vec(),
            }],
            frozen_files: vec![FrozenWorkspaceFile {
                path: target,
                content: Some(b"local\n".to_vec()),
            }],
            ..UpdatePlan::default()
        }
    }

    #[test]
    fn resolving_replaces_a_conflict_with_one_write_or_a_preserve() {
        let plan = staged_plan();
        let target = path("a/b.md");

        let mut kept = BTreeMap::new();
        kept.insert(target.clone(), b"local\n".to_vec());
        let preserved = plan.resolved(&kept).unwrap();
        assert!(preserved.mutations.is_empty());
        assert_eq!(
            preserved.changes,
            vec![PlannedFileChange {
                path: target.clone(),
                kind: FileChangeKind::Preserve,
            }]
        );

        let mut edited = BTreeMap::new();
        edited.insert(target.clone(), b"edited\n".to_vec());
        let updated = plan.resolved(&edited).unwrap();
        assert!(updated.conflicts.is_empty());
        assert!(updated.resolution_conflicts.is_empty());
        assert_eq!(
            updated.mutations,
            vec![WorkspaceMutation::Write {
                path: target.clone(),
                content: b"edited\n".to_vec(),
            }]
        );
        assert_eq!(updated.changes[0].kind, FileChangeKind::Update);

        assert!(plan.resolved(&BTreeMap::new()).is_none());
    }
}
