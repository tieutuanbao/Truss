//! S3b add-on planner adapter: the ten-tuple classification contract, dry run
//! only.
//!
//! Each row of the contract is one named fixture here, driven through
//! `FileSystemAddOnPlanner` with the real `GitThreeWayMerge` and a recorded
//! add-on baseline. Expected values are transcribed from the task contract, not
//! from the adapter output, so folding one reason into another
//! (`ModifiedRemovedFile` into `Delete`, or `ExistingUnmanagedPath` into
//! `Adopt`) trips the named row.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use truss::application::{
    AddOnInstallRequest, AddOnPayloadPort, AddOnPayloadSpec, AddOnPlanRequest, AddOnStatePort,
};
use truss::domain::{
    AddOnDescriptor, ConflictReason, FileChangeKind, RelativePath, UpdatePlan, WorkspaceMutation,
};
use truss::infrastructure::{FileSystemAddOnPayload, FileSystemAddOnPlanner, FileSystemAddOnState};

use common::{seed_core_state, snapshot_digest, workspace_snapshot, write_bytes};

const ADDON: &str = "demo";
const SOURCE_REF: &str = "truss-v0.1.13";
const SOURCE_CORE_VERSION: &str = "0.1.13";

/// The subject path of each row. Every fixture also carries one unchanged
/// carrier path, so a payload that drops the subject path is still a valid,
/// non-empty payload. The carrier always classifies as `Preserve`.
const SUBJECT: &str = ".agents/skills/demo/SKILL.md";
const CARRIER: &str = ".agents/skills/demo/agents/openai.yaml";
const CARRIER_BYTES: &[u8] = b"name: demo\n";

/// Row 3's non-overlapping edit pair, and the clean merge `git merge-file`
/// produces for it.
const BASE: &[u8] = b"one\ntwo\nthree\n";
const LOCAL_NON_OVERLAP: &[u8] = b"ONE\ntwo\nthree\n";
const NEXT_NON_OVERLAP: &[u8] = b"one\ntwo\nTHREE\n";
const MERGED_NON_OVERLAP: &[u8] = b"ONE\ntwo\nTHREE\n";
/// Row 2's payload bytes, which differ from the unchanged local bytes.
const NEXT_ROW2: &[u8] = b"upstream v2\n";
/// Row 4's payload bytes, which the local file already carries.
const NEXT_ROW4: &[u8] = b"ONE\ntwo\nTHREE\n";
/// Row 5's overlapping pair.
const LOCAL_OVERLAP: &[u8] = b"local overlap\n";
const NEXT_OVERLAP: &[u8] = b"next overlap\n";
/// A payload path that has no recorded baseline (rows 1 and 7).
const NEW_BYTES: &[u8] = b"new upstream\n";
/// A consumer edit that differs from every baseline (rows 7 and 9).
const CONSUMER_BYTES: &[u8] = b"consumer edit\n";

/// Dry-run fixture paths, one per covered row.
const DR_NEW: &str = ".agents/skills/demo/new.md";
const DR_MERGE: &str = ".agents/skills/demo/merge.md";
const DR_OVERLAP: &str = ".agents/skills/demo/overlap.md";
const DR_DELETE: &str = ".agents/skills/demo/delete.md";
const DR_MODIFIED: &str = ".agents/skills/demo/modified.md";

#[derive(Clone, Copy)]
enum ExpectedMutation {
    Write(&'static [u8]),
    Delete,
}

struct Expect {
    change: Option<FileChangeKind>,
    conflict: Option<ConflictReason>,
    mutation: Option<ExpectedMutation>,
    /// Subject entry in the frozen set: `None` = the subject is absent from the
    /// frozen set; `Some(None)` = frozen with absent content; `Some(Some(b))` =
    /// frozen with those local bytes.
    frozen: Option<Option<&'static [u8]>>,
    /// Row 5 additionally carries exactly one `ResolutionConflict`.
    resolution_conflict: bool,
}

struct Row {
    name: &'static str,
    /// Recorded baseline bytes for `SUBJECT`, or `None` when the payload path is
    /// new and has no baseline entry.
    base: Option<&'static [u8]>,
    /// Next payload bytes for `SUBJECT`, or `None` when the payload drops it.
    next: Option<&'static [u8]>,
    /// Workspace bytes for `SUBJECT`, or `None` when the path is absent.
    local: Option<&'static [u8]>,
    expect: Expect,
}

/// The ten-tuple contract as data, in contract order.
fn matrix() -> Vec<Row> {
    vec![
        Row {
            name: "row1_new_payload_path_without_local_creates",
            base: None,
            next: Some(NEW_BYTES),
            local: None,
            expect: Expect {
                change: Some(FileChangeKind::Create),
                conflict: None,
                mutation: Some(ExpectedMutation::Write(NEW_BYTES)),
                frozen: Some(None),
                resolution_conflict: false,
            },
        },
        Row {
            name: "row2_local_unchanged_takes_the_payload",
            base: Some(BASE),
            next: Some(NEXT_ROW2),
            local: Some(BASE),
            expect: Expect {
                change: Some(FileChangeKind::Update),
                conflict: None,
                mutation: Some(ExpectedMutation::Write(NEXT_ROW2)),
                frozen: Some(Some(BASE)),
                resolution_conflict: false,
            },
        },
        Row {
            name: "row3_non_overlapping_edits_merge_clean",
            base: Some(BASE),
            next: Some(NEXT_NON_OVERLAP),
            local: Some(LOCAL_NON_OVERLAP),
            expect: Expect {
                change: Some(FileChangeKind::Update),
                conflict: None,
                mutation: Some(ExpectedMutation::Write(MERGED_NON_OVERLAP)),
                frozen: Some(Some(LOCAL_NON_OVERLAP)),
                resolution_conflict: false,
            },
        },
        Row {
            name: "row4_local_already_equals_the_payload_preserves",
            base: Some(BASE),
            next: Some(NEXT_ROW4),
            local: Some(NEXT_ROW4),
            expect: Expect {
                change: Some(FileChangeKind::Preserve),
                conflict: None,
                mutation: None,
                frozen: Some(Some(NEXT_ROW4)),
                resolution_conflict: false,
            },
        },
        Row {
            name: "row5_overlapping_edits_conflict",
            base: Some(BASE),
            next: Some(NEXT_OVERLAP),
            local: Some(LOCAL_OVERLAP),
            expect: Expect {
                change: None,
                conflict: Some(ConflictReason::OverlappingChanges),
                mutation: None,
                frozen: Some(Some(LOCAL_OVERLAP)),
                resolution_conflict: true,
            },
        },
        Row {
            name: "row6_missing_managed_file_conflicts",
            base: Some(BASE),
            next: Some(NEXT_ROW2),
            local: None,
            expect: Expect {
                change: None,
                conflict: Some(ConflictReason::MissingManagedFile),
                mutation: None,
                frozen: Some(None),
                resolution_conflict: false,
            },
        },
        Row {
            name: "row7_existing_unmanaged_path_conflicts",
            base: None,
            next: Some(NEW_BYTES),
            local: Some(CONSUMER_BYTES),
            expect: Expect {
                change: None,
                conflict: Some(ConflictReason::ExistingUnmanagedPath),
                mutation: None,
                frozen: Some(Some(CONSUMER_BYTES)),
                resolution_conflict: false,
            },
        },
        Row {
            name: "row8_removed_path_equal_to_baseline_deletes",
            base: Some(BASE),
            next: None,
            local: Some(BASE),
            expect: Expect {
                change: Some(FileChangeKind::Delete),
                conflict: None,
                mutation: Some(ExpectedMutation::Delete),
                frozen: Some(Some(BASE)),
                resolution_conflict: false,
            },
        },
        Row {
            name: "row9_removed_path_modified_by_consumer_conflicts",
            base: Some(BASE),
            next: None,
            local: Some(CONSUMER_BYTES),
            expect: Expect {
                change: None,
                conflict: Some(ConflictReason::ModifiedRemovedFile),
                mutation: None,
                frozen: Some(Some(CONSUMER_BYTES)),
                resolution_conflict: false,
            },
        },
        Row {
            name: "row10_removed_path_already_missing_preserves",
            base: Some(BASE),
            next: None,
            local: None,
            expect: Expect {
                change: Some(FileChangeKind::Preserve),
                conflict: None,
                mutation: None,
                frozen: Some(None),
                resolution_conflict: false,
            },
        },
    ]
}

struct Fixture {
    workspace: PathBuf,
    plan: UpdatePlan,
    _tmp: tempfile::TempDir,
}

fn write_payload(root: &Path, files: &[(&str, &[u8])]) {
    for (path, content) in files {
        write_bytes(root, path, content);
    }
}

fn manifest_text(files: &[(&str, &[u8])]) -> String {
    let mut text = String::from("# add-on planner fixture\n");
    for (path, _) in files {
        text.push_str(path);
        text.push('\n');
    }
    text
}

fn describe(payload: &Path, manifest: &Path) -> AddOnDescriptor {
    let foreign: [PathBuf; 0] = [];
    FileSystemAddOnPayload
        .describe(&AddOnPayloadSpec {
            root: payload,
            manifest,
            name: ADDON,
            source_ref: SOURCE_REF,
            source_core_version: SOURCE_CORE_VERSION,
            foreign_manifests: &foreign,
        })
        .unwrap()
}

/// Seed valid core state, install the recorded baseline, and return the
/// workspace. The baseline bytes are recorded through the S2 state writer, so
/// `addons.json` and `.truss-core/base-addons/<name>/` hold exactly the payload
/// bytes the adapter later reads back.
fn install_baseline(workspace: &Path, payload: &Path, manifest: &Path, files: &[(&str, &[u8])]) {
    write_payload(payload, files);
    fs::write(manifest, manifest_text(files)).unwrap();
    seed_core_state(workspace);
    let descriptor = describe(payload, manifest);
    FileSystemAddOnState
        .apply(
            workspace,
            &AddOnInstallRequest {
                descriptor: &descriptor,
                payload_root: payload,
            },
        )
        .unwrap();
}

fn build(row: &Row) -> Fixture {
    let tmp = tempfile::tempdir().unwrap();
    let baseline_payload = tmp.path().join("baseline-payload");
    let next_payload = tmp.path().join("next-payload");
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let baseline_manifest = tmp.path().join("baseline-files.txt");
    let next_manifest = tmp.path().join("next-files.txt");

    let baseline_files: Vec<(&str, &[u8])> = match row.base {
        Some(base) => vec![(CARRIER, CARRIER_BYTES), (SUBJECT, base)],
        None => vec![(CARRIER, CARRIER_BYTES)],
    };
    install_baseline(
        &workspace,
        &baseline_payload,
        &baseline_manifest,
        &baseline_files,
    );

    let next_files: Vec<(&str, &[u8])> = match row.next {
        Some(next) => vec![(CARRIER, CARRIER_BYTES), (SUBJECT, next)],
        None => vec![(CARRIER, CARRIER_BYTES)],
    };
    write_payload(&next_payload, &next_files);
    fs::write(&next_manifest, manifest_text(&next_files)).unwrap();

    match row.local {
        Some(bytes) => write_bytes(&workspace, SUBJECT, bytes),
        None => {
            let target = workspace.join(SUBJECT);
            if target.exists() {
                fs::remove_file(target).unwrap();
            }
        }
    }

    let descriptor = describe(&next_payload, &next_manifest);
    let plan = FileSystemAddOnPlanner
        .plan(
            &workspace,
            &AddOnPlanRequest {
                descriptor: &descriptor,
                payload_root: &next_payload,
            },
        )
        .unwrap_or_else(|error| panic!("row {}: adapter failed: {error}", row.name));

    Fixture {
        workspace,
        plan,
        _tmp: tmp,
    }
}

fn subject() -> RelativePath {
    RelativePath::parse(SUBJECT).unwrap()
}

fn assert_row(row: &Row, plan: &UpdatePlan) {
    let subject = subject();

    let changes = plan
        .changes
        .iter()
        .filter(|change| change.path == subject)
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
        .filter(|conflict| conflict.path == subject)
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

    let mutations = plan
        .mutations
        .iter()
        .filter(|mutation| mutation.path() == &subject)
        .cloned()
        .collect::<Vec<_>>();
    match row.expect.mutation {
        Some(ExpectedMutation::Write(bytes)) => assert_eq!(
            mutations,
            vec![WorkspaceMutation::Write {
                path: subject.clone(),
                content: bytes.to_vec(),
            }],
            "row {}: mutations",
            row.name
        ),
        Some(ExpectedMutation::Delete) => assert_eq!(
            mutations,
            vec![WorkspaceMutation::Delete {
                path: subject.clone(),
            }],
            "row {}: mutations",
            row.name
        ),
        None => assert!(
            mutations.is_empty(),
            "row {}: expected no mutation, got {mutations:?}",
            row.name
        ),
    }

    let frozen = plan
        .frozen_files
        .iter()
        .find(|frozen| frozen.path == subject)
        .map(|frozen| frozen.content.clone());
    let expected_frozen = row.expect.frozen.map(|content| content.map(<[u8]>::to_vec));
    assert_eq!(frozen, expected_frozen, "row {}: frozen set", row.name);

    let resolution_conflicts = plan
        .resolution_conflicts
        .iter()
        .filter(|conflict| conflict.path == subject)
        .count();
    assert_eq!(
        resolution_conflicts,
        usize::from(row.expect.resolution_conflict),
        "row {}: resolution conflicts",
        row.name
    );

    // The unchanged carrier proves the union is planned per path, not per
    // payload: it is present in both baseline and payload and always preserves.
    let carrier = RelativePath::parse(CARRIER).unwrap();
    let carrier_changes = plan
        .changes
        .iter()
        .filter(|change| change.path == carrier)
        .map(|change| change.kind.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        carrier_changes,
        vec![FileChangeKind::Preserve],
        "row {}: unchanged carrier must preserve",
        row.name
    );
}

fn observed(plan: &UpdatePlan, path: &str) -> String {
    let path = RelativePath::parse(path).unwrap();
    let change = plan
        .changes
        .iter()
        .find(|change| change.path == path)
        .map(|change| format!("{:?}", change.kind))
        .unwrap_or_else(|| "none".to_owned());
    let conflict = plan
        .conflicts
        .iter()
        .find(|conflict| conflict.path == path)
        .map(|conflict| format!("{:?}", conflict.reason))
        .unwrap_or_else(|| "none".to_owned());
    let mutation = plan
        .mutations
        .iter()
        .filter(|mutation| mutation.path() == &path)
        .count();
    let frozen = plan
        .frozen_files
        .iter()
        .find(|frozen| frozen.path == path)
        .map(|frozen| match &frozen.content {
            Some(content) => format!("{:x}", Sha256::digest(content)),
            None => "absent".to_owned(),
        })
        .unwrap_or_else(|| "not-frozen".to_owned());
    format!("change={change} conflict={conflict} mutations={mutation} frozen={frozen}")
}

/// Acceptance row 1: all ten contract tuples classify exactly as the contract
/// states, through the extracted planner, with a real `GitThreeWayMerge`.
#[test]
fn every_classification_row_matches_the_contract() {
    let rows = matrix();
    assert_eq!(rows.len(), 10, "the contract has ten rows");
    let mut evidence = String::new();
    for row in &rows {
        let fixture = build(row);
        assert_row(row, &fixture.plan);
        evidence.push_str(&format!(
            "{}: subject[{}] carrier[Preserve]\n",
            row.name,
            observed(&fixture.plan, SUBJECT),
        ));
        assert!(
            fixture.workspace.join(".truss-core/addons.json").is_file(),
            "row {}: the fixture must have a recorded baseline",
            row.name
        );
    }
    write_evidence("s3b-table-rows.txt", &evidence);
}

/// Acceptance row 1, unsafe-path clause: a path failing managed-path
/// validation stays an `UnsafePath` conflict and is never frozen, and a path
/// that ends as a conflict is still frozen with its local content.
#[cfg(unix)]
#[test]
fn unsafe_path_is_an_unsafe_path_conflict_and_is_never_frozen() {
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().unwrap();
    let baseline_payload = tmp.path().join("baseline-payload");
    let next_payload = tmp.path().join("next-payload");
    let workspace = tmp.path().join("workspace");
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let baseline_manifest = tmp.path().join("baseline-files.txt");
    let next_manifest = tmp.path().join("next-files.txt");

    let baseline_files: Vec<(&str, &[u8])> = vec![(CARRIER, CARRIER_BYTES), (SUBJECT, BASE)];
    install_baseline(
        &workspace,
        &baseline_payload,
        &baseline_manifest,
        &baseline_files,
    );
    let next_files: Vec<(&str, &[u8])> = vec![(CARRIER, CARRIER_BYTES), (SUBJECT, NEXT_ROW2)];
    write_payload(&next_payload, &next_files);
    fs::write(&next_manifest, manifest_text(&next_files)).unwrap();

    // Replace the workspace payload root with a symlink so every managed path
    // under it fails validation.
    fs::remove_dir_all(workspace.join(".agents")).unwrap();
    symlink(&outside, workspace.join(".agents")).unwrap();

    let descriptor = describe(&next_payload, &next_manifest);
    let plan = FileSystemAddOnPlanner
        .plan(
            &workspace,
            &AddOnPlanRequest {
                descriptor: &descriptor,
                payload_root: &next_payload,
            },
        )
        .unwrap();

    assert!(
        plan.changes.is_empty() && plan.mutations.is_empty(),
        "an unsafe path must produce no change and no mutation"
    );
    assert!(
        plan.frozen_files.is_empty(),
        "an unsafe path must never be frozen"
    );
    assert_eq!(plan.conflicts.len(), 2);
    for conflict in &plan.conflicts {
        assert_eq!(conflict.reason, ConflictReason::UnsafePath);
    }
    write_evidence(
        "s3b-unsafe-path.txt",
        &format!(
            "conflicts={}\nfrozen={}\n",
            plan.conflicts.len(),
            plan.frozen_files.len()
        ),
    );
}

/// Acceptance row 2: a dry run over a fixture set that includes rows 1, 3, 5,
/// 8, and 9 mutates nothing — complete workspace, baseline tree, and
/// `addons.json` are byte-identical — while still reporting the plan.
#[test]
fn dry_run_reports_the_plan_and_mutates_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let baseline_payload = tmp.path().join("baseline-payload");
    let next_payload = tmp.path().join("next-payload");
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let baseline_manifest = tmp.path().join("baseline-files.txt");
    let next_manifest = tmp.path().join("next-files.txt");

    let baseline_files: Vec<(&str, &[u8])> = vec![
        (DR_MERGE, BASE),
        (DR_OVERLAP, BASE),
        (DR_DELETE, BASE),
        (DR_MODIFIED, BASE),
    ];
    install_baseline(
        &workspace,
        &baseline_payload,
        &baseline_manifest,
        &baseline_files,
    );
    let next_files: Vec<(&str, &[u8])> = vec![
        (DR_NEW, NEW_BYTES),
        (DR_MERGE, NEXT_NON_OVERLAP),
        (DR_OVERLAP, NEXT_OVERLAP),
    ];
    write_payload(&next_payload, &next_files);
    fs::write(&next_manifest, manifest_text(&next_files)).unwrap();

    // Row 1: new path, no local file. Row 3: non-overlapping local edit.
    // Row 5: overlapping local edit. Row 8: untouched removed path. Row 9:
    // consumer-modified removed path.
    write_bytes(&workspace, DR_MERGE, LOCAL_NON_OVERLAP);
    write_bytes(&workspace, DR_OVERLAP, LOCAL_OVERLAP);
    write_bytes(&workspace, DR_MODIFIED, CONSUMER_BYTES);

    let addons_path = workspace.join(".truss-core/addons.json");
    let baseline_root = workspace.join(".truss-core/base-addons");
    let before = workspace_snapshot(&workspace);
    let addons_before = fs::read(&addons_path).unwrap();
    let baseline_before = snapshot_digest(&workspace_snapshot(&baseline_root));

    let descriptor = describe(&next_payload, &next_manifest);
    let plan = FileSystemAddOnPlanner
        .plan(
            &workspace,
            &AddOnPlanRequest {
                descriptor: &descriptor,
                payload_root: &next_payload,
            },
        )
        .unwrap();

    let after = workspace_snapshot(&workspace);
    let addons_after = fs::read(&addons_path).unwrap();
    let baseline_after = snapshot_digest(&workspace_snapshot(&baseline_root));

    // The dry run must have classified the fixture set rather than trivially
    // returning an empty plan.
    let new_path = RelativePath::parse(DR_NEW).unwrap();
    assert_eq!(
        plan.changes
            .iter()
            .filter(|change| change.path == new_path)
            .map(|change| change.kind.clone())
            .collect::<Vec<_>>(),
        vec![FileChangeKind::Create]
    );
    assert!(
        plan.frozen_files
            .iter()
            .any(|frozen| frozen.path == new_path && frozen.content.is_none()),
        "the created path must be frozen as absent"
    );
    let merge_path = RelativePath::parse(DR_MERGE).unwrap();
    assert_eq!(
        plan.changes
            .iter()
            .filter(|change| change.path == merge_path)
            .map(|change| change.kind.clone())
            .collect::<Vec<_>>(),
        vec![FileChangeKind::Update]
    );
    assert!(
        plan.mutations.iter().any(|mutation| mutation.path() == &merge_path
            && matches!(mutation, WorkspaceMutation::Write { content, .. } if content.as_slice() == MERGED_NON_OVERLAP)),
        "the clean merge must be the planned write"
    );
    assert!(
        plan.frozen_files
            .iter()
            .any(|frozen| frozen.path == merge_path
                && frozen.content.as_deref() == Some(LOCAL_NON_OVERLAP)),
        "the merged path must be frozen with its local content"
    );
    assert!(
        plan.conflicts.iter().any(|conflict| conflict.path
            == RelativePath::parse(DR_OVERLAP).unwrap()
            && conflict.reason == ConflictReason::OverlappingChanges),
        "the dry run must report the overlapping row"
    );
    assert!(
        plan.mutations
            .iter()
            .any(|mutation| mutation.path() == &RelativePath::parse(DR_DELETE).unwrap()),
        "the dry run must plan the clean deletion"
    );
    assert!(
        plan.conflicts.iter().any(|conflict| conflict.path
            == RelativePath::parse(DR_MODIFIED).unwrap()
            && conflict.reason == ConflictReason::ModifiedRemovedFile),
        "the dry run must report the modified removal"
    );

    assert_eq!(
        before, after,
        "a dry run must not change the workspace tree"
    );
    assert_eq!(
        addons_before, addons_after,
        "a dry run must not change addons.json"
    );
    assert_eq!(
        baseline_before, baseline_after,
        "a dry run must not change the baseline tree"
    );
    assert_eq!(
        snapshot_digest(&before),
        snapshot_digest(&after),
        "snapshot digests must match"
    );

    write_evidence(
        "s3b-dry-run.txt",
        &format!(
            "before={}\nafter={}\naddons_unchanged={}\nbaseline_unchanged={}\nchanges={}\nconflicts={}\n",
            snapshot_digest(&before),
            snapshot_digest(&after),
            addons_before == addons_after,
            baseline_before == baseline_after,
            plan.changes.len(),
            plan.conflicts.len()
        ),
    );
}

/// Clause 8 for the adapter: it requires a valid pre-existing core state and
/// never creates or repairs one, even on refusal.
#[test]
fn adapter_refuses_without_core_state_and_creates_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let payload = tmp.path().join("payload");
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let manifest = tmp.path().join("files.txt");
    let files: Vec<(&str, &[u8])> = vec![(CARRIER, CARRIER_BYTES), (SUBJECT, BASE)];
    write_payload(&payload, &files);
    fs::write(&manifest, manifest_text(&files)).unwrap();
    let descriptor = describe(&payload, &manifest);

    let before = workspace_snapshot(&workspace);
    let error = FileSystemAddOnPlanner
        .plan(
            &workspace,
            &AddOnPlanRequest {
                descriptor: &descriptor,
                payload_root: &payload,
            },
        )
        .unwrap_err()
        .to_string();
    let after = workspace_snapshot(&workspace);

    assert!(
        error.contains("require an existing core state"),
        "expected a core-state refusal, got: {error}"
    );
    assert_eq!(
        before, after,
        "a refusal must not create or change core state"
    );
    assert!(
        !workspace.join(".truss-core").exists(),
        "the adapter must never create the core state root"
    );
    write_evidence(
        "s3b-no-core-state.txt",
        &format!("refusal={error}\ncore_state_present=false\n"),
    );
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
