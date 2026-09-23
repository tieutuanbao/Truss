use std::collections::BTreeSet;

use super::service::ApplicationError;
use super::{PortError, ThreeWayMergePort};
use crate::domain::{
    ConflictReason, FileChangeKind, FrozenWorkspaceFile, MergeOutcome, PlannedFileChange,
    RelativePath, ResolutionConflict, UpdateConflict, UpdatePlan, UpdatePlanInput,
    WorkspaceMutation,
};

/// Classify every path in `baselines ∪ upstream` into a plan.
///
/// The planner is neutral: it reads no core provenance and touches no
/// filesystem. Bytes come from `input`, the merge port decides overlapping
/// edits, and `validate_managed_path` is the caller's existing safety check.
pub fn plan_update<M, F>(
    input: &UpdatePlanInput,
    merger: &M,
    mut validate_managed_path: F,
) -> Result<UpdatePlan, ApplicationError>
where
    M: ThreeWayMergePort,
    F: FnMut(&RelativePath) -> Result<(), PortError>,
{
    let mut plan = UpdatePlan::default();
    let paths = input
        .baselines
        .keys()
        .chain(input.upstream.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    for path in paths {
        if let Err(error) = validate_managed_path(&path) {
            plan.conflicts.push(UpdateConflict {
                path: path.clone(),
                reason: ConflictReason::UnsafePath,
                detail: error.to_string(),
            });
            continue;
        }
        let local = input.local.get(&path).cloned();
        plan.frozen_files.push(FrozenWorkspaceFile {
            path: path.clone(),
            content: local.clone(),
        });
        let baseline = input.baselines.get(&path);
        let upstream = input.upstream.get(&path);
        match (baseline, upstream, local) {
            (Some(base), Some(next), Some(local)) => {
                if let Some(resolved) = input.resolutions.get(&path) {
                    let kind = if resolved == &local {
                        FileChangeKind::Preserve
                    } else {
                        plan.mutations.push(WorkspaceMutation::Write {
                            path: path.clone(),
                            content: resolved.clone(),
                        });
                        FileChangeKind::Update
                    };
                    plan.changes.push(PlannedFileChange { path, kind });
                    continue;
                }
                match merge_contents(merger, base, &local, next)? {
                    MergeOutcome::Clean(content) => {
                        let kind = if content == local {
                            FileChangeKind::Preserve
                        } else {
                            plan.mutations.push(WorkspaceMutation::Write {
                                path: path.clone(),
                                content,
                            });
                            FileChangeKind::Update
                        };
                        plan.changes.push(PlannedFileChange { path, kind });
                    }
                    MergeOutcome::Conflict { content, detail } => {
                        plan.resolution_conflicts.push(ResolutionConflict {
                            path: path.clone(),
                            base: base.clone(),
                            local,
                            incoming: next.clone(),
                            resolved: content,
                        });
                        plan.conflicts.push(UpdateConflict {
                            path,
                            reason: ConflictReason::OverlappingChanges,
                            detail,
                        });
                    }
                }
            }
            (Some(_), Some(_), None) => plan.conflicts.push(UpdateConflict {
                path,
                reason: ConflictReason::MissingManagedFile,
                detail: "managed file is missing from the consumer workspace".to_owned(),
            }),
            (None, Some(next), None) => {
                plan.changes.push(PlannedFileChange {
                    path: path.clone(),
                    kind: FileChangeKind::Create,
                });
                plan.mutations.push(WorkspaceMutation::Write {
                    path,
                    content: next.clone(),
                });
            }
            (None, Some(_), Some(_)) => plan.conflicts.push(UpdateConflict {
                path,
                reason: ConflictReason::ExistingUnmanagedPath,
                detail: "new upstream managed path already exists locally".to_owned(),
            }),
            (Some(base), None, Some(local)) if &local == base => {
                plan.changes.push(PlannedFileChange {
                    path: path.clone(),
                    kind: FileChangeKind::Delete,
                });
                plan.mutations.push(WorkspaceMutation::Delete { path });
            }
            (Some(_), None, Some(_)) => plan.conflicts.push(UpdateConflict {
                path,
                reason: ConflictReason::ModifiedRemovedFile,
                detail: "upstream removed a file that contains consumer changes".to_owned(),
            }),
            (Some(_), None, None) => plan.changes.push(PlannedFileChange {
                path,
                kind: FileChangeKind::Preserve,
            }),
            (None, None, _) => unreachable!("path union cannot contain an absent path"),
        }
    }

    Ok(plan)
}

fn merge_contents<M: ThreeWayMergePort>(
    merger: &M,
    base: &[u8],
    local: &[u8],
    upstream: &[u8],
) -> Result<MergeOutcome, ApplicationError> {
    if local == base {
        return Ok(MergeOutcome::Clean(upstream.to_vec()));
    }
    if upstream == base || local == upstream {
        return Ok(MergeOutcome::Clean(local.to_vec()));
    }
    merger.merge(base, local, upstream).map_err(Into::into)
}
