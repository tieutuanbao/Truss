//! S3c add-on apply: acceptance rows 1 and 3.
//!
//! Row 1 drives a conflict-free plan with a `Create`, a merged `Update`, a
//! `Delete`, and a `Preserve` through `FileSystemAddOnApplier` and reads the
//! workspace, the baseline tree, and `addons.json` back against the descriptor
//! and the plan. Row 3 drives a plan carrying one conflict plus one clean
//! change and requires refusal with all three surfaces byte-identical.
//!
//! Row 2's injected failures need the `#[cfg(test)]`-only deterministic hook,
//! which an integration test cannot reach; that row lives in
//! `truss::infrastructure::addon_apply`'s unit test.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use truss::application::{
    AddOnApplyRequest, AddOnInstallRequest, AddOnPayloadPort, AddOnPayloadSpec, AddOnPlanRequest,
    AddOnStatePort,
};
use truss::domain::{AddOnDescriptor, FileChangeKind, RelativePath, UpdatePlan, WorkspaceMutation};
use truss::infrastructure::{
    FileSystemAddOnApplier, FileSystemAddOnPayload, FileSystemAddOnPlanner, FileSystemAddOnState,
};

use common::{seed_core_state, snapshot_digest, workspace_snapshot, write_bytes};

const ADDON: &str = "demo";
const OLD_REF: &str = "truss-v0.1.13";
const OLD_CORE: &str = "0.1.13";
const NEW_REF: &str = "truss-v0.1.14";
const NEW_CORE: &str = "0.1.14";

const CARRIER: &str = ".agents/skills/demo/agents/openai.yaml";
const CARRIER_BYTES: &[u8] = b"name: demo\n";

const MERGE: &str = ".agents/skills/demo/merge.md";
const DELETE: &str = ".agents/skills/demo/delete.md";
const PRESERVE: &str = ".agents/skills/demo/preserve.md";
const CREATE: &str = ".agents/skills/demo/create.md";

const BASE: &[u8] = b"one\ntwo\nthree\n";
const LOCAL_NON_OVERLAP: &[u8] = b"ONE\ntwo\nthree\n";
const NEXT_NON_OVERLAP: &[u8] = b"one\ntwo\nTHREE\n";
const MERGED_NON_OVERLAP: &[u8] = b"ONE\ntwo\nTHREE\n";
const PRESERVED: &[u8] = b"preserved\n";
const CREATED: &[u8] = b"created upstream\n";

const CONFLICT_PATH: &str = ".agents/skills/demo/conflict.md";
const CLEAN_PATH: &str = ".agents/skills/demo/clean.md";
const LOCAL_OVERLAP: &[u8] = b"local overlap\n";
const NEXT_OVERLAP: &[u8] = b"next overlap\n";
const CLEAN_NEXT: &[u8] = b"clean next\n";

fn manifest_text(files: &[&str]) -> String {
    let mut text = String::from("# add-on apply fixture\n");
    for path in files {
        text.push_str(path);
        text.push('\n');
    }
    text
}

fn write_payload(root: &Path, files: &[(&str, &[u8])]) {
    for (path, content) in files {
        write_bytes(root, path, content);
    }
}

fn describe(payload: &Path, manifest: &Path, source_ref: &str, core: &str) -> AddOnDescriptor {
    let foreign: [PathBuf; 0] = [];
    FileSystemAddOnPayload
        .describe(&AddOnPayloadSpec {
            root: payload,
            manifest,
            name: ADDON,
            source_ref,
            source_core_version: core,
            foreign_manifests: &foreign,
        })
        .unwrap()
}

/// Install the recorded baseline through the S2 state writer, so the recorded
/// bytes are exactly the payload bytes the applier later replaces.
fn install_baseline(workspace: &Path, payload: &Path, manifest: &Path, files: &[(&str, &[u8])]) {
    write_payload(payload, files);
    let paths = files.iter().map(|(path, _)| *path).collect::<Vec<_>>();
    fs::write(manifest, manifest_text(&paths)).unwrap();
    seed_core_state(workspace);
    let descriptor = describe(payload, manifest, OLD_REF, OLD_CORE);
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

fn plan(
    workspace: &Path,
    next_payload: &Path,
    next_manifest: &Path,
    next_files: &[(&str, &[u8])],
) -> (AddOnDescriptor, UpdatePlan) {
    write_payload(next_payload, next_files);
    let paths = next_files.iter().map(|(path, _)| *path).collect::<Vec<_>>();
    fs::write(next_manifest, manifest_text(&paths)).unwrap();
    let descriptor = describe(next_payload, next_manifest, NEW_REF, NEW_CORE);
    let plan = FileSystemAddOnPlanner
        .plan(
            workspace,
            &AddOnPlanRequest {
                descriptor: &descriptor,
                payload_root: next_payload,
            },
        )
        .unwrap();
    (descriptor, plan)
}

fn kind(plan: &UpdatePlan, path: &str) -> Vec<FileChangeKind> {
    let path = RelativePath::parse(path).unwrap();
    plan.changes
        .iter()
        .filter(|change| change.path == path)
        .map(|change| change.kind.clone())
        .collect()
}

fn mutation(plan: &UpdatePlan, path: &str) -> Vec<WorkspaceMutation> {
    let path = RelativePath::parse(path).unwrap();
    plan.mutations
        .iter()
        .filter(|mutation| mutation.path() == &path)
        .cloned()
        .collect()
}

fn baseline_bytes(workspace: &Path, path: &str) -> Option<Vec<u8>> {
    let target = workspace
        .join(".truss-core/base-addons")
        .join(ADDON)
        .join(path);
    fs::read(target).ok()
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Acceptance row 1: every planned write and delete lands, the baseline mirrors
/// the payload bytes per path (not the merged workspace bytes), and
/// `addons.json` names the new ref and the payload digests.
#[test]
fn conflict_free_plan_applies_completely() {
    let tmp = tempfile::tempdir().unwrap();
    let baseline_payload = tmp.path().join("baseline-payload");
    let next_payload = tmp.path().join("next-payload");
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let baseline_manifest = tmp.path().join("baseline-files.txt");
    let next_manifest = tmp.path().join("next-files.txt");

    install_baseline(
        &workspace,
        &baseline_payload,
        &baseline_manifest,
        &[
            (CARRIER, CARRIER_BYTES),
            (MERGE, BASE),
            (DELETE, BASE),
            (PRESERVE, PRESERVED),
        ],
    );
    // The recorded baseline leaves the workspace equal to the payload. Edit one
    // path so the next payload merges rather than overwrites.
    write_bytes(&workspace, MERGE, LOCAL_NON_OVERLAP);

    let (descriptor, plan) = plan(
        &workspace,
        &next_payload,
        &next_manifest,
        &[
            (CARRIER, CARRIER_BYTES),
            (CREATE, CREATED),
            (MERGE, NEXT_NON_OVERLAP),
            (PRESERVE, PRESERVED),
        ],
    );

    assert!(
        plan.conflicts.is_empty(),
        "the fixture must be conflict-free"
    );
    assert_eq!(kind(&plan, CREATE), vec![FileChangeKind::Create]);
    assert_eq!(kind(&plan, MERGE), vec![FileChangeKind::Update]);
    assert_eq!(kind(&plan, DELETE), vec![FileChangeKind::Delete]);
    assert_eq!(kind(&plan, PRESERVE), vec![FileChangeKind::Preserve]);
    assert_eq!(
        mutation(&plan, MERGE),
        vec![WorkspaceMutation::Write {
            path: RelativePath::parse(MERGE).unwrap(),
            content: MERGED_NON_OVERLAP.to_vec(),
        }],
        "the merged result must be the planned write"
    );

    let receipt = FileSystemAddOnApplier
        .apply(
            &workspace,
            &AddOnApplyRequest {
                descriptor: &descriptor,
                payload_root: &next_payload,
                plan: &plan,
            },
        )
        .unwrap();
    assert!(receipt.backup_path.is_some());

    // Workspace: every mutation landed, including the merged result and the
    // deletion, and the preserved path is untouched.
    assert_eq!(fs::read(workspace.join(CREATE)).unwrap(), CREATED);
    assert_eq!(fs::read(workspace.join(MERGE)).unwrap(), MERGED_NON_OVERLAP);
    assert!(!workspace.join(DELETE).exists());
    assert_eq!(fs::read(workspace.join(PRESERVE)).unwrap(), PRESERVED);
    assert_eq!(fs::read(workspace.join(CARRIER)).unwrap(), CARRIER_BYTES);

    // Baseline: the payload bytes per path, and the dropped path is gone.
    assert_eq!(baseline_bytes(&workspace, CREATE).unwrap(), CREATED);
    assert_eq!(
        baseline_bytes(&workspace, MERGE).unwrap(),
        NEXT_NON_OVERLAP,
        "the baseline must record the payload bytes, not the merged workspace bytes"
    );
    assert_eq!(baseline_bytes(&workspace, PRESERVE).unwrap(), PRESERVED);
    assert_eq!(baseline_bytes(&workspace, CARRIER).unwrap(), CARRIER_BYTES);
    assert!(
        baseline_bytes(&workspace, DELETE).is_none(),
        "a path the payload dropped must leave the baseline"
    );

    // Provenance: the new ref, the new core version, and per-file payload
    // digests (never the workspace file digests).
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(workspace.join(".truss-core/addons.json")).unwrap())
            .unwrap();
    assert_eq!(record["schema_version"], 1);
    let addons = record["addons"].as_array().unwrap();
    assert_eq!(addons.len(), 1);
    let demo = &addons[0];
    assert_eq!(demo["name"], ADDON);
    assert_eq!(demo["source_ref"], NEW_REF);
    assert_eq!(demo["source_core_version"], NEW_CORE);
    let recorded = demo["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| {
            (
                file["path"].as_str().unwrap().to_owned(),
                file["upstream_sha256"].as_str().unwrap().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        recorded,
        vec![
            (CARRIER.to_owned(), digest(CARRIER_BYTES)),
            (CREATE.to_owned(), digest(CREATED)),
            (MERGE.to_owned(), digest(NEXT_NON_OVERLAP)),
            (PRESERVE.to_owned(), digest(PRESERVED)),
        ],
        "addons.json must name the payload digests per declared path"
    );
    assert_ne!(
        digest(NEXT_NON_OVERLAP),
        digest(MERGED_NON_OVERLAP),
        "the fixture must distinguish payload bytes from merged workspace bytes"
    );
    assert!(
        !record.to_string().contains(OLD_REF),
        "the previous source ref must not survive the update"
    );
    assert!(
        !record.to_string().contains(OLD_CORE),
        "the previous core version must not survive the update"
    );

    write_evidence(
        "s3c-row1-apply.txt",
        &format!(
            "create={}\nmerge_workspace={:x}\nmerge_baseline={:x}\ndelete_present={}\npreserve={:x}\nsource_ref={NEW_REF}\nsource_core_version={NEW_CORE}\nbackup={}\n",
            String::from_utf8_lossy(
                &fs::read(workspace.join(CREATE)).unwrap_or_default()
            )
            .trim(),
            Sha256::digest(fs::read(workspace.join(MERGE)).unwrap()),
            Sha256::digest(baseline_bytes(&workspace, MERGE).unwrap()),
            workspace.join(DELETE).exists(),
            Sha256::digest(baseline_bytes(&workspace, PRESERVE).unwrap()),
            receipt.backup_path.is_some(),
        ),
    );
}

/// Acceptance row 3: a plan carrying one conflict plus one clean change is
/// refused, and the complete workspace, the baseline tree, and `addons.json`
/// stay byte-identical.
#[test]
fn any_conflict_refuses_without_mutation() {
    let tmp = tempfile::tempdir().unwrap();
    let baseline_payload = tmp.path().join("baseline-payload");
    let next_payload = tmp.path().join("next-payload");
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let baseline_manifest = tmp.path().join("baseline-files.txt");
    let next_manifest = tmp.path().join("next-files.txt");

    install_baseline(
        &workspace,
        &baseline_payload,
        &baseline_manifest,
        &[
            (CARRIER, CARRIER_BYTES),
            (CONFLICT_PATH, BASE),
            (CLEAN_PATH, BASE),
        ],
    );
    // Overlapping local and payload edits on one path; the other path is clean.
    write_bytes(&workspace, CONFLICT_PATH, LOCAL_OVERLAP);

    let (descriptor, plan) = plan(
        &workspace,
        &next_payload,
        &next_manifest,
        &[
            (CARRIER, CARRIER_BYTES),
            (CONFLICT_PATH, NEXT_OVERLAP),
            (CLEAN_PATH, CLEAN_NEXT),
        ],
    );

    assert!(
        !plan.conflicts.is_empty(),
        "the fixture must carry one conflict"
    );
    assert_eq!(kind(&plan, CLEAN_PATH), vec![FileChangeKind::Update]);
    assert_eq!(mutation(&plan, CLEAN_PATH).len(), 1);

    let addons = workspace.join(".truss-core/addons.json");
    let baseline_root = workspace.join(".truss-core/base-addons");
    let before_workspace = workspace_snapshot(&workspace);
    let before_addons = fs::read(&addons).unwrap();
    let before_baseline = snapshot_digest(&workspace_snapshot(&baseline_root));

    let error = FileSystemAddOnApplier
        .apply(
            &workspace,
            &AddOnApplyRequest {
                descriptor: &descriptor,
                payload_root: &next_payload,
                plan: &plan,
            },
        )
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("conflict"),
        "expected a conflict refusal, got: {error}"
    );

    let after_workspace = workspace_snapshot(&workspace);
    let after_addons = fs::read(&addons).unwrap();
    let after_baseline = snapshot_digest(&workspace_snapshot(&baseline_root));

    assert_eq!(
        before_workspace, after_workspace,
        "a conflict refusal must not change the workspace"
    );
    assert_eq!(
        before_addons, after_addons,
        "a conflict refusal must not change addons.json"
    );
    assert_eq!(
        before_baseline, after_baseline,
        "a conflict refusal must not change the baseline tree"
    );
    assert_eq!(
        fs::read(workspace.join(CONFLICT_PATH)).unwrap(),
        LOCAL_OVERLAP
    );
    assert_eq!(fs::read(workspace.join(CLEAN_PATH)).unwrap(), BASE);

    write_evidence(
        "s3c-row3-refusal.txt",
        &format!(
            "refusal={error}\nconflicts={}\nbefore={}\nafter={}\nbaseline_equal={}\naddons_equal={}\n",
            plan.conflicts.len(),
            snapshot_digest(&before_workspace),
            snapshot_digest(&after_workspace),
            before_baseline == after_baseline,
            before_addons == after_addons,
        ),
    );
}

/// A fixture with a planned `Update` (`merge.md`) and a planned `Delete`
/// (`delete.md`), ready to be drifted between planning and applying.
struct DriftFixture {
    _tmp: tempfile::TempDir,
    workspace: PathBuf,
    next_payload: PathBuf,
    descriptor: AddOnDescriptor,
    plan: UpdatePlan,
}

fn drift_fixture() -> DriftFixture {
    let tmp = tempfile::tempdir().unwrap();
    let baseline_payload = tmp.path().join("baseline-payload");
    let next_payload = tmp.path().join("next-payload");
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let baseline_manifest = tmp.path().join("baseline-files.txt");
    let next_manifest = tmp.path().join("next-files.txt");

    install_baseline(
        &workspace,
        &baseline_payload,
        &baseline_manifest,
        &[(CARRIER, CARRIER_BYTES), (MERGE, BASE), (DELETE, BASE)],
    );
    write_bytes(&workspace, MERGE, LOCAL_NON_OVERLAP);

    let (descriptor, plan) = plan(
        &workspace,
        &next_payload,
        &next_manifest,
        &[(CARRIER, CARRIER_BYTES), (MERGE, NEXT_NON_OVERLAP)],
    );
    DriftFixture {
        _tmp: tmp,
        workspace,
        next_payload,
        descriptor,
        plan,
    }
}

/// Acceptance Row A: a competing writer changing a managed path between the
/// planning lock section and the apply lock section cannot be overwritten by
/// the stale plan. The barrier is deterministic: the test plans, mutates one
/// managed path, and only then applies.
#[test]
fn planned_observation_drift_is_refused_without_clobber() {
    let competing = b"competing writer\n";
    let mut evidence = String::new();

    for (label, target, observed) in [
        ("planned_write", MERGE, LOCAL_NON_OVERLAP),
        ("planned_delete", DELETE, BASE),
    ] {
        let fixture = drift_fixture();
        assert!(
            fixture.plan.conflicts.is_empty(),
            "{label}: the fixture must be conflict-free"
        );
        assert_eq!(
            mutation(&fixture.plan, target).len(),
            1,
            "{label}: the plan must stage {target}"
        );
        let frozen = fixture
            .plan
            .frozen_files
            .iter()
            .find(|frozen| frozen.path == RelativePath::parse(target).unwrap())
            .expect("the planned path must be frozen");
        assert_eq!(
            frozen.content.as_deref(),
            Some(observed),
            "{label}: the planner must have observed different bytes than the competing writer"
        );

        let addons = fixture.workspace.join(".truss-core/addons.json");
        let baseline_root = fixture.workspace.join(".truss-core/base-addons");
        let baseline_before = snapshot_digest(&workspace_snapshot(&baseline_root));

        // The barrier: change one managed path after planning and before apply.
        write_bytes(&fixture.workspace, target, competing);
        let drifted = workspace_snapshot(&fixture.workspace);
        let addons_drifted = fs::read(&addons).unwrap();

        let error = FileSystemAddOnApplier
            .apply(
                &fixture.workspace,
                &AddOnApplyRequest {
                    descriptor: &fixture.descriptor,
                    payload_root: &fixture.next_payload,
                    plan: &fixture.plan,
                },
            )
            .unwrap_err()
            .to_string();

        assert!(
            error.contains("workspace changed"),
            "{label}: expected a drift refusal, got: {error}"
        );
        assert_eq!(
            fs::read(fixture.workspace.join(target)).unwrap(),
            competing,
            "{label}: the stale plan clobbered the competing bytes"
        );
        assert_eq!(
            workspace_snapshot(&fixture.workspace),
            drifted,
            "{label}: the refused apply changed the workspace"
        );
        assert_eq!(
            fs::read(&addons).unwrap(),
            addons_drifted,
            "{label}: the refused apply changed addons.json"
        );
        assert_eq!(
            snapshot_digest(&workspace_snapshot(&baseline_root)),
            baseline_before,
            "{label}: the refused apply changed the baseline"
        );

        evidence.push_str(&format!(
            "case={label}\ntarget={target}\nobserved_bytes={:x}\ncompeting_bytes={:x}\nrefusal={error}\ncompetitor_survives={}\nworkspace_unchanged_by_refusal={}\n\n",
            Sha256::digest(observed),
            Sha256::digest(competing),
            fs::read(fixture.workspace.join(target)).unwrap() == competing,
            workspace_snapshot(&fixture.workspace) == drifted,
        ));
    }

    write_evidence("s3c-rowA-drift.txt", &evidence);
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
