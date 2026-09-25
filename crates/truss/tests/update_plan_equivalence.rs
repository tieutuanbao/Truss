//! Behaviour-equivalence table for the extracted update planner.
//!
//! Every row of the S3a equivalence matrix is one fixture here, and every
//! expected value is transcribed from the matrix rather than taken from the
//! planner's own output. A change that folds one reason into another (say
//! `ModifiedRemovedFile` into `Delete`, or `ExistingUnmanagedPath` into
//! `Adopt`) trips the named row.

use std::collections::BTreeMap;

use truss::application::{plan_update, PortError, ThreeWayMergePort};
use truss::domain::{
    ConflictReason, FileChangeKind, FrozenWorkspaceFile, MergeOutcome, RelativePath,
    ResolutionConflict, UpdatePlanInput, WorkspaceMutation,
};

const PATH: &str = ".truss/core/docs/plan.txt";

const BASE: &[u8] = b"base\n";
const NEXT: &[u8] = b"next\n";
const LOCAL: &[u8] = b"local\n";
const RESOLVED: &[u8] = b"resolved\n";
const MERGED: &[u8] = b"merged\n";
const CONFLICT_CONTENT: &[u8] = b"<<<<<<< local\nlocal\n=======\nnext\n>>>>>>> upstream\n";

struct ScriptedMerge(MergeOutcome);

impl ThreeWayMergePort for ScriptedMerge {
    fn available(&self) -> Result<bool, PortError> {
        Ok(true)
    }

    fn merge(
        &self,
        _base: &[u8],
        _local: &[u8],
        _upstream: &[u8],
    ) -> Result<MergeOutcome, PortError> {
        Ok(self.0.clone())
    }
}

#[derive(Clone, Copy)]
enum ExpectedMutation {
    Write(&'static [u8]),
    Delete,
}

#[derive(Clone, Copy)]
struct ExpectConflict {
    base: &'static [u8],
    local: &'static [u8],
    incoming: &'static [u8],
    resolved: &'static [u8],
}

struct Expect {
    change: Option<FileChangeKind>,
    conflict: Option<ConflictReason>,
    mutation: Option<ExpectedMutation>,
    /// `None` means the path is absent from the frozen set; `Some(content)`
    /// means it is frozen with that local content (`None` bytes = locally
    /// absent).
    frozen: Option<Option<&'static [u8]>>,
    resolution_conflict: Option<ExpectConflict>,
}

struct Row {
    name: &'static str,
    baseline: Option<&'static [u8]>,
    upstream: Option<&'static [u8]>,
    local: Option<&'static [u8]>,
    resolution: Option<&'static [u8]>,
    merge: MergeOutcome,
    unsafe_path: bool,
    expect: Expect,
}

fn no_merge() -> MergeOutcome {
    MergeOutcome::Conflict {
        content: CONFLICT_CONTENT.to_vec(),
        detail: "overlap".to_owned(),
    }
}

fn conflict_merge() -> MergeOutcome {
    MergeOutcome::Conflict {
        content: CONFLICT_CONTENT.to_vec(),
        detail: "overlap".to_owned(),
    }
}

fn clean(bytes: &'static [u8]) -> MergeOutcome {
    MergeOutcome::Clean(bytes.to_vec())
}

/// The S3a equivalence matrix as data, in matrix order.
fn matrix() -> Vec<Row> {
    vec![
        Row {
            name: "row1_unsafe_path",
            baseline: Some(BASE),
            upstream: Some(NEXT),
            local: Some(LOCAL),
            resolution: None,
            merge: conflict_merge(),
            unsafe_path: true,
            expect: Expect {
                change: None,
                conflict: Some(ConflictReason::UnsafePath),
                mutation: None,
                frozen: None,
                resolution_conflict: None,
            },
        },
        Row {
            name: "row2_removed_upstream_without_local_preserves",
            baseline: Some(BASE),
            upstream: None,
            local: None,
            resolution: None,
            merge: no_merge(),
            unsafe_path: false,
            expect: Expect {
                change: Some(FileChangeKind::Preserve),
                conflict: None,
                mutation: None,
                frozen: Some(None),
                resolution_conflict: None,
            },
        },
        Row {
            name: "row3_removed_upstream_unchanged_local_deletes",
            baseline: Some(BASE),
            upstream: None,
            local: Some(BASE),
            resolution: None,
            merge: no_merge(),
            unsafe_path: false,
            expect: Expect {
                change: Some(FileChangeKind::Delete),
                conflict: None,
                mutation: Some(ExpectedMutation::Delete),
                frozen: Some(Some(BASE)),
                resolution_conflict: None,
            },
        },
        Row {
            name: "row4_removed_upstream_modified_local_conflicts",
            baseline: Some(BASE),
            upstream: None,
            local: Some(LOCAL),
            resolution: None,
            merge: no_merge(),
            unsafe_path: false,
            expect: Expect {
                change: None,
                conflict: Some(ConflictReason::ModifiedRemovedFile),
                mutation: None,
                frozen: Some(Some(LOCAL)),
                resolution_conflict: None,
            },
        },
        Row {
            name: "row5_baseline_and_upstream_without_local_conflicts",
            baseline: Some(BASE),
            upstream: Some(NEXT),
            local: None,
            resolution: None,
            merge: no_merge(),
            unsafe_path: false,
            expect: Expect {
                change: None,
                conflict: Some(ConflictReason::MissingManagedFile),
                mutation: None,
                frozen: Some(None),
                resolution_conflict: None,
            },
        },
        Row {
            name: "row6_resolution_matching_local_preserves",
            baseline: Some(BASE),
            upstream: Some(NEXT),
            local: Some(LOCAL),
            resolution: Some(LOCAL),
            merge: no_merge(),
            unsafe_path: false,
            expect: Expect {
                change: Some(FileChangeKind::Preserve),
                conflict: None,
                mutation: None,
                frozen: Some(Some(LOCAL)),
                resolution_conflict: None,
            },
        },
        Row {
            name: "row7_resolution_differing_from_local_updates",
            baseline: Some(BASE),
            upstream: Some(NEXT),
            local: Some(LOCAL),
            resolution: Some(RESOLVED),
            merge: no_merge(),
            unsafe_path: false,
            expect: Expect {
                change: Some(FileChangeKind::Update),
                conflict: None,
                mutation: Some(ExpectedMutation::Write(RESOLVED)),
                frozen: Some(Some(LOCAL)),
                resolution_conflict: None,
            },
        },
        Row {
            name: "row8_clean_merge_matching_local_preserves",
            baseline: Some(BASE),
            upstream: Some(NEXT),
            local: Some(LOCAL),
            resolution: None,
            merge: clean(LOCAL),
            unsafe_path: false,
            expect: Expect {
                change: Some(FileChangeKind::Preserve),
                conflict: None,
                mutation: None,
                frozen: Some(Some(LOCAL)),
                resolution_conflict: None,
            },
        },
        Row {
            name: "row9_clean_merge_differing_from_local_updates",
            baseline: Some(BASE),
            upstream: Some(NEXT),
            local: Some(LOCAL),
            resolution: None,
            merge: clean(MERGED),
            unsafe_path: false,
            expect: Expect {
                change: Some(FileChangeKind::Update),
                conflict: None,
                mutation: Some(ExpectedMutation::Write(MERGED)),
                frozen: Some(Some(LOCAL)),
                resolution_conflict: None,
            },
        },
        Row {
            name: "row10_overlapping_merge_conflicts_and_freezes",
            baseline: Some(BASE),
            upstream: Some(NEXT),
            local: Some(LOCAL),
            resolution: None,
            merge: conflict_merge(),
            unsafe_path: false,
            expect: Expect {
                change: None,
                conflict: Some(ConflictReason::OverlappingChanges),
                mutation: None,
                frozen: Some(Some(LOCAL)),
                resolution_conflict: Some(ExpectConflict {
                    base: BASE,
                    local: LOCAL,
                    incoming: NEXT,
                    resolved: CONFLICT_CONTENT,
                }),
            },
        },
        Row {
            name: "row11_new_upstream_without_local_creates",
            baseline: None,
            upstream: Some(NEXT),
            local: None,
            resolution: None,
            merge: no_merge(),
            unsafe_path: false,
            expect: Expect {
                change: Some(FileChangeKind::Create),
                conflict: None,
                mutation: Some(ExpectedMutation::Write(NEXT)),
                frozen: Some(None),
                resolution_conflict: None,
            },
        },
        Row {
            name: "row12_new_upstream_with_local_conflicts",
            baseline: None,
            upstream: Some(NEXT),
            local: Some(LOCAL),
            resolution: None,
            merge: no_merge(),
            unsafe_path: false,
            expect: Expect {
                change: None,
                conflict: Some(ConflictReason::ExistingUnmanagedPath),
                mutation: None,
                frozen: Some(Some(LOCAL)),
                resolution_conflict: None,
            },
        },
    ]
}

#[test]
fn every_matrix_row_matches_the_extracted_planner() {
    let rows = matrix();
    assert_eq!(rows.len(), 12, "the matrix has twelve drivable rows");
    for row in &rows {
        check_row(row);
    }
}

/// The matrix's unreachable row: a path with no baseline and no upstream can
/// never enter the union, so a local-only path is never classified, frozen, or
/// mutated.
#[test]
fn path_outside_the_baseline_upstream_union_is_never_planned() {
    let planned = RelativePath::parse(PATH).unwrap();
    let local_only = RelativePath::parse(".truss/core/docs/local-only.txt").unwrap();
    let mut input = UpdatePlanInput {
        resolutions: BTreeMap::new(),
        ..UpdatePlanInput::default()
    };
    input.baselines.insert(planned.clone(), BASE.to_vec());
    input.upstream.insert(planned.clone(), NEXT.to_vec());
    input.local.insert(planned.clone(), LOCAL.to_vec());
    input
        .local
        .insert(local_only.clone(), b"consumer only\n".to_vec());

    let plan = plan_update(&input, &ScriptedMerge(clean(LOCAL)), |_| Ok(())).unwrap();

    let frozen_paths = plan
        .frozen_files
        .iter()
        .map(|frozen| frozen.path.clone())
        .collect::<Vec<_>>();
    let changed_paths = plan
        .changes
        .iter()
        .map(|change| change.path.clone())
        .collect::<Vec<_>>();
    let mutated_paths = plan
        .mutations
        .iter()
        .map(|mutation| mutation.path().clone())
        .collect::<Vec<_>>();
    for candidate in frozen_paths
        .iter()
        .chain(&changed_paths)
        .chain(&mutated_paths)
    {
        assert_ne!(candidate, &local_only, "local-only path entered the plan");
    }
    assert_eq!(plan.changes.len(), 1, "only the union path is planned");
    assert_eq!(plan.frozen_files.len(), 1, "only the union path is frozen");
}

fn check_row(row: &Row) {
    let path = RelativePath::parse(PATH).unwrap();
    let mut input = UpdatePlanInput::default();
    if let Some(bytes) = row.baseline {
        input.baselines.insert(path.clone(), bytes.to_vec());
    }
    if let Some(bytes) = row.upstream {
        input.upstream.insert(path.clone(), bytes.to_vec());
    }
    if let Some(bytes) = row.local {
        input.local.insert(path.clone(), bytes.to_vec());
    }
    if let Some(bytes) = row.resolution {
        input.resolutions.insert(path.clone(), bytes.to_vec());
    }

    let validate = |candidate: &RelativePath| -> Result<(), PortError> {
        assert_eq!(
            candidate, &path,
            "row {}: validator saw an unexpected path",
            row.name
        );
        if row.unsafe_path {
            Err(PortError::new(format!(
                "refusing symlink for managed path {candidate}"
            )))
        } else {
            Ok(())
        }
    };
    let plan = plan_update(&input, &ScriptedMerge(row.merge.clone()), validate)
        .unwrap_or_else(|error| panic!("row {}: planner failed: {error}", row.name));

    let changes = plan
        .changes
        .iter()
        .map(|change| change.kind.clone())
        .collect::<Vec<_>>();
    match row.expect.change.clone() {
        Some(kind) => assert_eq!(changes, vec![kind], "row {}: change kind", row.name),
        None => assert!(
            changes.is_empty(),
            "row {}: expected no change, got {changes:?}",
            row.name
        ),
    }

    let reasons = plan
        .conflicts
        .iter()
        .map(|conflict| conflict.reason.clone())
        .collect::<Vec<_>>();
    match row.expect.conflict.clone() {
        Some(reason) => assert_eq!(reasons, vec![reason], "row {}: conflict reason", row.name),
        None => assert!(
            reasons.is_empty(),
            "row {}: expected no conflict, got {reasons:?}",
            row.name
        ),
    }

    match row.expect.mutation {
        Some(ExpectedMutation::Write(bytes)) => assert_eq!(
            plan.mutations,
            vec![WorkspaceMutation::Write {
                path: path.clone(),
                content: bytes.to_vec(),
            }],
            "row {}: mutations",
            row.name
        ),
        Some(ExpectedMutation::Delete) => assert_eq!(
            plan.mutations,
            vec![WorkspaceMutation::Delete { path: path.clone() }],
            "row {}: mutations",
            row.name
        ),
        None => assert!(
            plan.mutations.is_empty(),
            "row {}: expected no mutation, got {:?}",
            row.name,
            plan.mutations
        ),
    }

    match row.expect.frozen {
        Some(content) => assert_eq!(
            plan.frozen_files,
            vec![FrozenWorkspaceFile {
                path: path.clone(),
                content: content.map(|bytes| bytes.to_vec()),
            }],
            "row {}: frozen set",
            row.name
        ),
        None => assert!(
            plan.frozen_files.is_empty(),
            "row {}: unsafe path must not be frozen",
            row.name
        ),
    }

    match row.expect.resolution_conflict {
        Some(conflict) => assert_eq!(
            plan.resolution_conflicts,
            vec![ResolutionConflict {
                path: path.clone(),
                base: conflict.base.to_vec(),
                local: conflict.local.to_vec(),
                incoming: conflict.incoming.to_vec(),
                resolved: conflict.resolved.to_vec(),
            }],
            "row {}: resolution conflict payload",
            row.name
        ),
        None => assert!(
            plan.resolution_conflicts.is_empty(),
            "row {}: expected no resolution conflict",
            row.name
        ),
    }
}
