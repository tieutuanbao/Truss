//! S4b1 self-contained frozen conflict-session acceptance rows.
//!
//! The session is schema 2: it stores the complete candidate payload for every
//! descriptor path, the descriptor identity (ordered paths and digests), the
//! materialised plan with its digest, the per-conflict resolution inputs, and
//! every frozen workspace observation. Resume takes the add-on name alone and
//! never re-plans.
//!
//! Row 1 (`staged_session_resumes_without_the_external_payload`) stages a plan
//! with one clean write, one delete, one preserve, and one conflict, deletes
//! the external payload directory, edits `resolved/`, resumes, and verifies the
//! workspace, the complete baseline, the provenance record, and the removal of
//! the owned session. The companion
//! `resume_writes_provenance_last_and_stays_retryable` proves the provenance
//! record is written only after every workspace mutation by failing a later
//! mutation deterministically and re-running.
//!
//! Row 2 (`stored_session_material_is_validated_not_trusted`) tampers one
//! candidate file, tampers `plan.json`, and drops one candidate path; each must
//! refuse before any mutation with the three surfaces byte-identical.
//!
//! Row 3 (`schema_one_session_refuses_continue_and_permits_abort` and
//! `unsupported_schema_fails_closed`) refuses a schema-1 session with a message
//! naming abort and re-stage while still permitting abort, and refuses an
//! unsupported schema instead of reinterpreting it.
//!
//! Two further deterministic tests report the session size for the largest
//! shipped add-on payload and confirm that candidate material rejects symlinks
//! and path escapes on both stage and load.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use truss::application::{
    AddOnInstallRequest, AddOnPayloadPort, AddOnPayloadSpec, AddOnPlanRequest, AddOnStageRequest,
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
const CONFLICT: &str = ".agents/skills/demo/conflict.md";
const CLEAN: &str = ".agents/skills/demo/clean.md";
const UNRELATED: &str = ".agents/skills/demo/unrelated.md";
const REMOVED: &str = ".agents/skills/demo/removed.md";

const BASE: &[u8] = b"one\ntwo\nthree\n";
// `CLEAN` is a genuine clean three-way merge: the consumer edits the first
// region and upstream edits the last one, so the mutation content comes from
// `git merge-file` rather than from either side alone. That is exactly why the
// frozen `plan.json` must store the merged bytes.
const CLEAN_BASE: &[u8] = b"one\ntwo\nthree\n";
const CLEAN_LOCAL: &[u8] = b"ONE\ntwo\nthree\n";
const CLEAN_NEXT: &[u8] = b"one\ntwo\nTHREE\n";
const CLEAN_MERGED: &[u8] = b"ONE\ntwo\nTHREE\n";
const LOCAL_OVERLAP: &[u8] = b"local overlap\n";
const NEXT_OVERLAP: &[u8] = b"next overlap\n";
const UNRELATED_BYTES: &[u8] = b"unrelated\n";
const COMPETING: &[u8] = b"competing writer\n";
const RESOLVED: &[u8] = b"resolved by human\n";

const ADDON_SESSION: &str = ".truss-core/addon-update";
const SIBLING_SESSION: &str = ".truss-core/addon-update/other/session.json";
const SIBLING_BYTES: &[u8] = b"other add-on session\n";
const CORE_SESSION: &str = ".truss-core/update/session.json";
const CORE_SESSION_BYTES: &[u8] = b"core session\n";

fn manifest_text(files: &[&str]) -> String {
    let mut text = String::from("# add-on session fixture\n");
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

/// The conflicted fixture shared by the acceptance rows: one overlap conflict,
/// one clean write (`CLEAN`), one clean delete (`REMOVED`, absent from the next
/// payload), and two preserved paths (`CARRIER`, `UNRELATED`).
struct Fixture {
    _tmp: tempfile::TempDir,
    workspace: PathBuf,
    next_payload: PathBuf,
    descriptor: AddOnDescriptor,
    plan: UpdatePlan,
}

fn conflict_fixture() -> Fixture {
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
            (CONFLICT, BASE),
            (CLEAN, CLEAN_BASE),
            (UNRELATED, UNRELATED_BYTES),
            (REMOVED, BASE),
        ],
    );
    write_bytes(&workspace, CONFLICT, LOCAL_OVERLAP);
    write_bytes(&workspace, CLEAN, CLEAN_LOCAL);

    write_payload(
        &next_payload,
        &[
            (CARRIER, CARRIER_BYTES),
            (CONFLICT, NEXT_OVERLAP),
            (CLEAN, CLEAN_NEXT),
            (UNRELATED, UNRELATED_BYTES),
        ],
    );
    fs::write(
        &next_manifest,
        manifest_text(&[CARRIER, CONFLICT, CLEAN, UNRELATED]),
    )
    .unwrap();
    let descriptor = describe(&next_payload, &next_manifest, NEW_REF, NEW_CORE);
    let plan = FileSystemAddOnPlanner
        .plan(
            &workspace,
            &AddOnPlanRequest {
                descriptor: &descriptor,
                payload_root: &next_payload,
            },
        )
        .unwrap();
    assert_eq!(plan.conflicts.len(), 1, "the fixture carries one conflict");
    assert_eq!(plan.resolution_conflicts.len(), 1);
    assert_eq!(kind(&plan, CLEAN), vec![FileChangeKind::Update]);
    assert_eq!(kind(&plan, REMOVED), vec![FileChangeKind::Delete]);
    assert_eq!(kind(&plan, CARRIER), vec![FileChangeKind::Preserve]);
    assert_eq!(mutation(&plan, CLEAN).len(), 1);
    assert_eq!(mutation(&plan, REMOVED).len(), 1);
    assert_eq!(
        mutation(&plan, CLEAN),
        vec![WorkspaceMutation::Write {
            path: RelativePath::parse(CLEAN).unwrap(),
            content: CLEAN_MERGED.to_vec(),
        }],
        "the clean write must carry the merged bytes"
    );
    Fixture {
        _tmp: tmp,
        workspace,
        next_payload,
        descriptor,
        plan,
    }
}

fn stage(fixture: &Fixture) {
    FileSystemAddOnApplier
        .stage(
            &fixture.workspace,
            &AddOnStageRequest {
                descriptor: &fixture.descriptor,
                payload_root: &fixture.next_payload,
                plan: &fixture.plan,
            },
        )
        .unwrap();
}

fn seed_core_session(workspace: &Path) {
    write_bytes(workspace, CORE_SESSION, CORE_SESSION_BYTES);
    write_bytes(
        workspace,
        ".truss-core/update/resolved/carrier.md",
        b"core resolution\n",
    );
}

fn session_root(workspace: &Path) -> PathBuf {
    workspace.join(ADDON_SESSION).join(ADDON)
}

fn session_bytes(workspace: &Path, directory: &str, path: &str) -> Vec<u8> {
    fs::read(session_root(workspace).join(directory).join(path)).unwrap()
}

fn baseline_bytes(workspace: &Path, path: &str) -> Option<Vec<u8>> {
    fs::read(
        workspace
            .join(".truss-core/base-addons")
            .join(ADDON)
            .join(path),
    )
    .ok()
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn mutation(plan: &UpdatePlan, path: &str) -> Vec<WorkspaceMutation> {
    let path = RelativePath::parse(path).unwrap();
    plan.mutations
        .iter()
        .filter(|mutation| mutation.path() == &path)
        .cloned()
        .collect()
}

fn kind(plan: &UpdatePlan, path: &str) -> Vec<FileChangeKind> {
    let path = RelativePath::parse(path).unwrap();
    plan.changes
        .iter()
        .filter(|change| change.path == path)
        .map(|change| change.kind.clone())
        .collect()
}

/// A workspace snapshot that can skip the add-on session namespace, so a stage,
/// a refusal, or an abort can be proven to change nothing else.
fn snapshot_without(root: &Path, excluded: &[&str]) -> String {
    let mut lines = Vec::new();
    walk(root, root, excluded, &mut lines);
    lines.sort();
    lines.join("\n")
}

fn walk(root: &Path, directory: &Path, excluded: &[&str], lines: &mut Vec<String>) {
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
        if excluded.iter().any(|prefix| relative.starts_with(prefix)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&path).unwrap();
        if metadata.file_type().is_symlink() {
            let target = fs::read_link(&path).unwrap();
            lines.push(format!("symlink {relative} -> {}", target.display()));
        } else if metadata.is_dir() {
            lines.push(format!("dir {relative}"));
            walk(root, &path, excluded, lines);
        } else if metadata.is_file() {
            let bytes = fs::read(&path).unwrap();
            lines.push(format!("file {relative} {:x}", Sha256::digest(&bytes)));
        } else {
            lines.push(format!("other {relative}"));
        }
    }
}

/// Total byte size of every regular file below `root`.
fn tree_size(root: &Path) -> u64 {
    let mut total = 0;
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let metadata = fs::symlink_metadata(entry.path()).unwrap();
        if metadata.is_dir() {
            total += tree_size(&entry.path());
        } else if metadata.is_file() {
            total += metadata.len();
        }
    }
    total
}

fn tree_files(root: &Path) -> usize {
    let mut total = 0;
    for entry in fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let metadata = fs::symlink_metadata(entry.path()).unwrap();
        if metadata.is_dir() {
            total += tree_files(&entry.path());
        } else if metadata.is_file() {
            total += 1;
        }
    }
    total
}

/// The three surfaces a refusal must leave byte-identical: the workspace, the
/// add-on baseline tree, and `.truss-core/addons.json`.
struct Surfaces {
    workspace: String,
    baseline: String,
    addons: Vec<u8>,
}

fn surfaces(workspace: &Path) -> Surfaces {
    let baseline_root = workspace.join(".truss-core/base-addons");
    Surfaces {
        workspace: snapshot_without(workspace, &[ADDON_SESSION]),
        baseline: snapshot_digest(&workspace_snapshot(&baseline_root)),
        addons: fs::read(workspace.join(".truss-core/addons.json")).unwrap(),
    }
}

fn assert_surfaces_unchanged(before: &Surfaces, workspace: &Path, context: &str) {
    assert_eq!(
        snapshot_without(workspace, &[ADDON_SESSION]),
        before.workspace,
        "{context}: the refusal changed the workspace"
    );
    let baseline_root = workspace.join(".truss-core/base-addons");
    assert_eq!(
        snapshot_digest(&workspace_snapshot(&baseline_root)),
        before.baseline,
        "{context}: the refusal changed the baseline"
    );
    assert_eq!(
        fs::read(workspace.join(".truss-core/addons.json")).unwrap(),
        before.addons,
        "{context}: the refusal wrote provenance"
    );
}

/// Acceptance row 1: the session carries the complete candidate, so resume
/// completes after the external payload directory is deleted.
#[test]
fn staged_session_resumes_without_the_external_payload() {
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    seed_core_session(workspace);
    stage(&fixture);

    // The session records the descriptor identity, the ordered descriptor
    // paths and digests, the plan digest, and the path lists.
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(session_root(workspace).join("session.json")).unwrap())
            .unwrap();
    assert_eq!(record["schema_version"], 2);
    assert_eq!(record["from_ref"], OLD_REF);
    assert_eq!(record["source_ref"], NEW_REF);
    assert_eq!(record["source_core_version"], NEW_CORE);
    let recorded = record["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["path"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        recorded,
        vec![
            CARRIER.to_owned(),
            CONFLICT.to_owned(),
            CLEAN.to_owned(),
            UNRELATED.to_owned(),
        ],
        "the session must keep the descriptor path order"
    );
    assert_eq!(record["plan_sha256"].as_str().unwrap().len(), 64);
    assert!(session_root(workspace).join("plan.json").is_file());
    // The complete candidate: one payload file per descriptor path.
    for file in &fixture.descriptor.files {
        assert_eq!(
            session_bytes(workspace, "candidate", file.path.as_str()),
            fs::read(fixture.next_payload.join(file.path.as_str())).unwrap(),
            "candidate bytes for {}",
            file.path
        );
    }

    // The operator edits the resolution, and the external payload disappears.
    write_bytes(
        workspace,
        &format!("{ADDON_SESSION}/{ADDON}/resolved/{CONFLICT}"),
        RESOLVED,
    );
    fs::remove_dir_all(&fixture.next_payload).unwrap();
    assert!(!fixture.next_payload.exists());

    let receipt = FileSystemAddOnApplier
        .resume(workspace, &fixture.descriptor.name)
        .unwrap();
    assert!(receipt.backup_path.is_some());

    // Workspace bytes: the resolution, the merged clean write, the delete, and
    // the preserved paths.
    assert_eq!(fs::read(workspace.join(CONFLICT)).unwrap(), RESOLVED);
    assert_eq!(fs::read(workspace.join(CLEAN)).unwrap(), CLEAN_MERGED);
    assert!(!workspace.join(REMOVED).exists(), "the delete must land");
    assert_eq!(fs::read(workspace.join(CARRIER)).unwrap(), CARRIER_BYTES);
    assert_eq!(
        fs::read(workspace.join(UNRELATED)).unwrap(),
        UNRELATED_BYTES
    );

    // The complete baseline: the payload bytes for every descriptor path (the
    // upstream bytes, never the merged or the local bytes), and nothing for the
    // removed path.
    assert_eq!(baseline_bytes(workspace, CONFLICT).unwrap(), NEXT_OVERLAP);
    assert_eq!(baseline_bytes(workspace, CLEAN).unwrap(), CLEAN_NEXT);
    assert_eq!(baseline_bytes(workspace, CARRIER).unwrap(), CARRIER_BYTES);
    assert_eq!(
        baseline_bytes(workspace, UNRELATED).unwrap(),
        UNRELATED_BYTES
    );
    assert!(
        baseline_bytes(workspace, REMOVED).is_none(),
        "a deleted path must leave the baseline"
    );

    // Provenance: the new ref and the payload digests, written after the
    // workspace and the baseline.
    let addons: serde_json::Value =
        serde_json::from_slice(&fs::read(workspace.join(".truss-core/addons.json")).unwrap())
            .unwrap();
    let demo = &addons["addons"].as_array().unwrap()[0];
    assert_eq!(demo["source_ref"], NEW_REF);
    assert_eq!(demo["source_core_version"], NEW_CORE);
    let recorded_files = demo["files"]
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
        recorded_files,
        vec![
            (CARRIER.to_owned(), digest(CARRIER_BYTES)),
            (CONFLICT.to_owned(), digest(NEXT_OVERLAP)),
            (CLEAN.to_owned(), digest(CLEAN_NEXT)),
            (UNRELATED.to_owned(), digest(UNRELATED_BYTES)),
        ]
    );
    assert!(!workspace.join(".truss-core/transaction.json").exists());

    // Only the owned session is cleared; the seeded core session survives.
    assert!(
        !session_root(workspace).exists(),
        "the owned add-on session must be cleared"
    );
    assert_eq!(
        fs::read(workspace.join(CORE_SESSION)).unwrap(),
        CORE_SESSION_BYTES,
        "resume must not touch the core session"
    );

    write_evidence(
        "s4b1-row1-self-contained.txt",
        &format!(
            "candidate_paths={}\nresolved={}\nclean_workspace={}\nremoved_absent={}\nconflict_baseline={}\nsource_ref={NEW_REF}\nsession_cleared={}\ncore_session_survives={}\n",
            fixture.descriptor.files.len(),
            digest(&fs::read(workspace.join(CONFLICT)).unwrap()),
            digest(&fs::read(workspace.join(CLEAN)).unwrap()),
            !workspace.join(REMOVED).exists(),
            digest(&baseline_bytes(workspace, CONFLICT).unwrap()),
            !session_root(workspace).exists(),
            fs::read(workspace.join(CORE_SESSION)).unwrap() == CORE_SESSION_BYTES,
        ),
    );
}

/// Provenance is written last: a deterministic failure at a later workspace
/// mutation leaves `addons.json` and the baseline byte-identical even though an
/// earlier mutation was applied and rolled back, and the session stays
/// retryable.
#[test]
fn resume_writes_provenance_last_and_stays_retryable() {
    let tmp = tempfile::tempdir().unwrap();
    let baseline_payload = tmp.path().join("baseline-payload");
    let next_payload = tmp.path().join("next-payload");
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let baseline_manifest = tmp.path().join("baseline-files.txt");
    let next_manifest = tmp.path().join("next-files.txt");

    // A new path whose parent is, in the workspace, a regular file. It plans as
    // a clean create and sorts after the clean write, so the transaction
    // applies the clean write first and then fails on this create.
    const BLOCKER: &str = ".agents/skills/demo/zzz";
    const BLOCKED: &str = ".agents/skills/demo/zzz/child.md";

    install_baseline(
        &workspace,
        &baseline_payload,
        &baseline_manifest,
        &[(CONFLICT, BASE), (CLEAN, CLEAN_BASE)],
    );
    write_bytes(&workspace, CONFLICT, LOCAL_OVERLAP);
    write_bytes(&workspace, BLOCKER, b"not a directory\n");
    write_payload(
        &next_payload,
        &[
            (CONFLICT, NEXT_OVERLAP),
            (CLEAN, CLEAN_NEXT),
            (BLOCKED, b"blocked create\n"),
        ],
    );
    fs::write(&next_manifest, manifest_text(&[CONFLICT, CLEAN, BLOCKED])).unwrap();
    let descriptor = describe(&next_payload, &next_manifest, NEW_REF, NEW_CORE);
    let plan = FileSystemAddOnPlanner
        .plan(
            &workspace,
            &AddOnPlanRequest {
                descriptor: &descriptor,
                payload_root: &next_payload,
            },
        )
        .unwrap();
    assert_eq!(kind(&plan, BLOCKED), vec![FileChangeKind::Create]);
    FileSystemAddOnApplier
        .stage(
            &workspace,
            &AddOnStageRequest {
                descriptor: &descriptor,
                payload_root: &next_payload,
                plan: &plan,
            },
        )
        .unwrap();
    write_bytes(
        &workspace,
        &format!("{ADDON_SESSION}/{ADDON}/resolved/{CONFLICT}"),
        RESOLVED,
    );

    let before = surfaces(&workspace);
    let error = FileSystemAddOnApplier
        .resume(&workspace, &descriptor.name)
        .unwrap_err()
        .to_string();
    assert!(
        !error.contains("injected"),
        "the failure must be a real filesystem failure: {error}"
    );
    assert_surfaces_unchanged(&before, &workspace, "provenance-last");
    // The earlier clean write was applied and rolled back; the competing
    // blocker file is intact and the blocked path was never created.
    let clean_rolled_back = fs::read(workspace.join(CLEAN)).unwrap() == BASE;
    let addons_equal =
        fs::read(workspace.join(".truss-core/addons.json")).unwrap() == before.addons;
    assert!(clean_rolled_back, "the applied clean write must roll back");
    assert!(addons_equal, "addons.json must not be written");
    assert_eq!(
        fs::read(workspace.join(BLOCKER)).unwrap(),
        b"not a directory\n"
    );
    assert!(!workspace.join(BLOCKED).exists());
    assert!(
        session_root(&workspace).join("session.json").is_file(),
        "the session must survive a failed resume"
    );

    // Remove the blocker: the same staged session now resumes successfully.
    fs::remove_file(workspace.join(BLOCKER)).unwrap();
    FileSystemAddOnApplier
        .resume(&workspace, &descriptor.name)
        .unwrap();
    assert_eq!(fs::read(workspace.join(CLEAN)).unwrap(), CLEAN_NEXT);
    assert_eq!(fs::read(workspace.join(CONFLICT)).unwrap(), RESOLVED);
    assert_eq!(
        fs::read(workspace.join(BLOCKED)).unwrap(),
        b"blocked create\n"
    );
    assert!(!session_root(&workspace).exists());

    write_evidence(
        "s4b1-row1-provenance-last.txt",
        &format!(
            "failing_mutation={BLOCKED}\nrefusal={error}\nclean_rolled_back={clean_rolled_back}\naddons_equal={addons_equal}\nretry_clean={}\nretry_blocked_created={}\nsession_cleared={}\n",
            fs::read(workspace.join(CLEAN)).unwrap() == CLEAN_NEXT,
            workspace.join(BLOCKED).exists(),
            !session_root(&workspace).exists(),
        ),
    );
}

/// Acceptance row 2: the stored material is validated, not trusted. A tampered
/// candidate file, a tampered `plan.json`, and a missing candidate path each
/// refuse before any mutation with the three surfaces byte-identical.
#[test]
fn stored_session_material_is_validated_not_trusted() {
    type Tamper = fn(&Path) -> &'static str;
    let cases: [(&str, Tamper); 3] = [
        ("tampered candidate", |workspace| {
            let path = session_root(workspace).join("candidate").join(CLEAN);
            let mut bytes = fs::read(&path).unwrap();
            bytes.extend_from_slice(b"tampered\n");
            fs::write(&path, &bytes).unwrap();
            "digest mismatch"
        }),
        ("tampered plan", |workspace| {
            let path = session_root(workspace).join("plan.json");
            let mut bytes = fs::read(&path).unwrap();
            bytes.push(b'\n');
            fs::write(&path, &bytes).unwrap();
            "plan digest mismatch"
        }),
        ("dropped candidate", |workspace| {
            fs::remove_file(session_root(workspace).join("candidate").join(CLEAN)).unwrap();
            "candidate path set differs"
        }),
    ];

    let mut evidence = String::new();
    for (label, tamper) in cases {
        let fixture = conflict_fixture();
        let workspace = &fixture.workspace;
        stage(&fixture);
        let expected = tamper(workspace);

        let before = surfaces(workspace);
        let error = FileSystemAddOnApplier
            .resume(workspace, &fixture.descriptor.name)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(expected),
            "{label}: expected {expected:?}, got: {error}"
        );
        assert_surfaces_unchanged(&before, workspace, label);
        assert!(
            session_root(workspace).join("session.json").is_file(),
            "{label}: the refused resume must keep the session"
        );
        evidence.push_str(&format!(
            "case={label}\nrefusal={error}\nsession_alive={}\n\n",
            session_root(workspace).join("session.json").is_file(),
        ));
    }
    write_evidence("s4b1-row2-validated.txt", &evidence);
}

/// Acceptance row 3: a schema-1 session refuses `continue` with a message
/// naming abort and re-stage, and still permits `abort`.
#[test]
fn schema_one_session_refuses_continue_and_permits_abort() {
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    stage(&fixture);
    // Replace the schema-2 session with a faithful schema-1 one: identity and
    // path lists only, with no candidate payload and no materialised plan.
    fs::remove_dir_all(session_root(workspace).join("candidate")).unwrap();
    fs::remove_file(session_root(workspace).join("plan.json")).unwrap();
    let legacy = serde_json::json!({
        "schema_version": 1,
        "from_version": OLD_REF,
        "to_version": NEW_REF,
        "conflicts": [{ "path": CONFLICT }],
        "frozen_files": [{ "path": CONFLICT, "present": true }],
    });
    fs::write(
        session_root(workspace).join("session.json"),
        serde_json::to_vec_pretty(&legacy).unwrap(),
    )
    .unwrap();

    let before = surfaces(workspace);
    let error = FileSystemAddOnApplier
        .resume(workspace, &fixture.descriptor.name)
        .unwrap_err()
        .to_string();
    assert!(error.contains("schema 1"), "got: {error}");
    assert!(error.contains("abort"), "got: {error}");
    assert!(error.contains("re-stage"), "got: {error}");
    assert_surfaces_unchanged(&before, workspace, "schema 1 refusal");
    assert!(
        session_root(workspace).join("session.json").is_file(),
        "the schema-1 session must stay abortable"
    );

    assert!(
        FileSystemAddOnApplier
            .abort(workspace, &fixture.descriptor.name)
            .unwrap(),
        "abort must still remove the schema-1 session"
    );
    assert!(!session_root(workspace).exists());

    write_evidence(
        "s4b1-row3-schema-one.txt",
        &format!("refusal={error}\nabort_removed=true\nsession_present=false\n"),
    );
}

/// Acceptance row 3: an unsupported schema fails closed instead of being
/// reinterpreted, and the session stays abortable.
#[test]
fn unsupported_schema_fails_closed() {
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    stage(&fixture);
    let unsupported = serde_json::json!({ "schema_version": 3 });
    fs::write(
        session_root(workspace).join("session.json"),
        serde_json::to_vec_pretty(&unsupported).unwrap(),
    )
    .unwrap();

    let before = surfaces(workspace);
    let error = FileSystemAddOnApplier
        .resume(workspace, &fixture.descriptor.name)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unsupported add-on resolution schema 3"),
        "got: {error}"
    );
    assert_surfaces_unchanged(&before, workspace, "unsupported schema");
    assert!(FileSystemAddOnApplier
        .abort(workspace, &fixture.descriptor.name)
        .unwrap());

    write_evidence(
        "s4b1-row3-unsupported.txt",
        &format!("refusal={error}\nabort_removed=true\n"),
    );
}

/// The deleted candidate path and the complete candidate round-trip through a
/// staging that changes nothing else, and `.truss-core/update/` is untouched.
#[test]
fn staging_persists_a_self_contained_session_and_mutates_nothing() {
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    seed_core_session(workspace);

    let addons = workspace.join(".truss-core/addons.json");
    let baseline_root = workspace.join(".truss-core/base-addons");
    let core_session = workspace.join(".truss-core/update");
    let before_workspace = snapshot_without(workspace, &[ADDON_SESSION]);
    let before_addons = fs::read(&addons).unwrap();
    let before_baseline = snapshot_digest(&workspace_snapshot(&baseline_root));
    let before_core = workspace_snapshot(&core_session);

    stage(&fixture);

    // Every conflict input round-trips byte-for-byte from the plan.
    let resolution_conflict = &fixture.plan.resolution_conflicts[0];
    assert_eq!(
        session_bytes(workspace, "base", CONFLICT),
        resolution_conflict.base
    );
    assert_eq!(
        session_bytes(workspace, "local", CONFLICT),
        resolution_conflict.local
    );
    assert_eq!(
        session_bytes(workspace, "incoming", CONFLICT),
        resolution_conflict.incoming
    );
    assert_eq!(
        session_bytes(workspace, "resolved", CONFLICT),
        resolution_conflict.resolved
    );
    // Every frozen observation round-trips, absent paths included.
    for frozen in &fixture.plan.frozen_files {
        if let Some(content) = &frozen.content {
            assert_eq!(
                session_bytes(workspace, "frozen", frozen.path.as_str()),
                *content
            );
        } else {
            assert!(
                !session_root(workspace)
                    .join("frozen")
                    .join(frozen.path.as_str())
                    .exists(),
                "an absent frozen path must not be stored"
            );
        }
    }

    assert_eq!(
        snapshot_without(workspace, &[ADDON_SESSION]),
        before_workspace,
        "staging must not change any managed file, baseline, or provenance"
    );
    assert_eq!(fs::read(&addons).unwrap(), before_addons);
    assert_eq!(
        snapshot_digest(&workspace_snapshot(&baseline_root)),
        before_baseline
    );
    assert_eq!(workspace_snapshot(&core_session), before_core);
    assert_eq!(fs::read(workspace.join(CLEAN)).unwrap(), CLEAN_LOCAL);
    assert_eq!(fs::read(workspace.join(CONFLICT)).unwrap(), LOCAL_OVERLAP);
    write_evidence(
        "s4b1-stage.txt",
        &format!(
            "candidate_files={}\nplan_present={}\nworkspace_equal={}\naddons_equal={}\nbaseline_equal={}\ncore_session_equal={}\n",
            tree_files(&session_root(workspace).join("candidate")),
            session_root(workspace).join("plan.json").is_file(),
            snapshot_without(workspace, &[ADDON_SESSION]) == before_workspace,
            fs::read(&addons).unwrap() == before_addons,
            snapshot_digest(&workspace_snapshot(&baseline_root)) == before_baseline,
            workspace_snapshot(&core_session) == before_core,
        ),
    );
}

/// A competing change to any frozen managed path between staging and resume is
/// refused under the shared lock, with the competing bytes intact.
#[test]
fn resume_refuses_unrelated_frozen_drift() {
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    seed_core_session(workspace);
    stage(&fixture);
    write_bytes(
        workspace,
        &format!("{ADDON_SESSION}/{ADDON}/resolved/{CONFLICT}"),
        RESOLVED,
    );

    write_bytes(workspace, CARRIER, COMPETING);
    let before = surfaces(workspace);
    let drifted = before.workspace.clone();

    let error = FileSystemAddOnApplier
        .resume(workspace, &fixture.descriptor.name)
        .unwrap_err()
        .to_string();
    assert!(error.contains("workspace changed"), "got: {error}");
    assert_eq!(fs::read(workspace.join(CARRIER)).unwrap(), COMPETING);
    assert_surfaces_unchanged(&before, workspace, "frozen drift");
    assert_eq!(
        snapshot_without(workspace, &[ADDON_SESSION]),
        drifted,
        "the refused resume changed the workspace"
    );
    assert!(
        session_root(workspace).join("session.json").is_file(),
        "the refused resume must keep the session"
    );

    write_evidence(
        "s4b1-frozen-drift.txt",
        &format!(
            "target={CARRIER}\ncompeting_bytes={}\nrefusal={error}\ncompetitor_survives={}\nsession_alive={}\n",
            digest(COMPETING),
            fs::read(workspace.join(CARRIER)).unwrap() == COMPETING,
            session_root(workspace).join("session.json").is_file(),
        ),
    );
}

/// Abort removes only the owned add-on session, leaves the workspace, baseline,
/// `addons.json`, the sibling add-on session, and the core session untouched,
/// and is safe to repeat.
#[test]
fn abort_is_scoped_and_idempotent() {
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    seed_core_session(workspace);
    write_bytes(workspace, SIBLING_SESSION, SIBLING_BYTES);
    stage(&fixture);
    assert!(session_root(workspace).join("session.json").is_file());

    let before = surfaces(workspace);
    let core_session = workspace.join(".truss-core/update");
    let before_core = workspace_snapshot(&core_session);

    assert!(
        FileSystemAddOnApplier
            .abort(workspace, &fixture.descriptor.name)
            .unwrap(),
        "the first abort must remove the owned session"
    );
    assert!(
        !session_root(workspace).exists(),
        "the owned session must be gone"
    );
    assert_eq!(
        fs::read(workspace.join(SIBLING_SESSION)).unwrap(),
        SIBLING_BYTES,
        "the sibling session was touched"
    );
    assert!(
        !FileSystemAddOnApplier
            .abort(workspace, &fixture.descriptor.name)
            .unwrap(),
        "the second abort must report that nothing was removed"
    );
    assert_surfaces_unchanged(&before, workspace, "abort");
    assert_eq!(workspace_snapshot(&core_session), before_core);
    assert_eq!(
        fs::read(workspace.join(SIBLING_SESSION)).unwrap(),
        SIBLING_BYTES
    );

    write_evidence(
        "s4b1-abort.txt",
        &format!(
            "first_removed=true\nsecond_removed=false\nowned_session_present={}\nsibling_present={}\ncore_session_equal={}\n",
            session_root(workspace).exists(),
            workspace.join(SIBLING_SESSION).exists(),
            workspace_snapshot(&core_session) == before_core,
        ),
    );
}

/// Reporting row: stage a session for the largest shipped add-on payload and
/// report the resulting session size in bytes.
#[test]
fn largest_shipped_addon_payload_reports_the_session_size() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let manifest = repo.join("scripts/delivery-install-files.txt");
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    seed_core_state(&workspace);
    let foreign: [PathBuf; 0] = [];

    let describe = |root: &Path, source_ref: &str| {
        FileSystemAddOnPayload
            .describe(&AddOnPayloadSpec {
                root,
                manifest: &manifest,
                name: "delivery",
                source_ref,
                source_core_version: "0.1.13",
                foreign_manifests: &foreign,
            })
            .unwrap()
    };
    let old = describe(&repo, OLD_REF);
    FileSystemAddOnState
        .apply(
            &workspace,
            &AddOnInstallRequest {
                descriptor: &old,
                payload_root: &repo,
            },
        )
        .unwrap();

    // The next payload mirrors the shipped one with exactly one path changed,
    // and the consumer edits that same path, so the plan conflicts once.
    let next_payload = tmp.path().join("next-payload");
    for file in &old.files {
        write_bytes(
            &next_payload,
            file.path.as_str(),
            &fs::read(repo.join(file.path.as_str())).unwrap(),
        );
    }
    let target = old.files[0].path.clone();
    let mut changed = fs::read(next_payload.join(target.as_str())).unwrap();
    changed.extend_from_slice(b"\n# upstream change\n");
    fs::write(next_payload.join(target.as_str()), &changed).unwrap();
    write_bytes(&workspace, target.as_str(), b"consumer edit\n");

    let next = describe(&next_payload, NEW_REF);
    let plan = FileSystemAddOnPlanner
        .plan(
            &workspace,
            &AddOnPlanRequest {
                descriptor: &next,
                payload_root: &next_payload,
            },
        )
        .unwrap();
    assert_eq!(plan.conflicts.len(), 1);
    FileSystemAddOnApplier
        .stage(
            &workspace,
            &AddOnStageRequest {
                descriptor: &next,
                payload_root: &next_payload,
                plan: &plan,
            },
        )
        .unwrap();

    let session = workspace.join(ADDON_SESSION).join("delivery");
    let session_bytes = tree_size(&session);
    let session_files = tree_files(&session);
    let payload_bytes = old
        .files
        .iter()
        .map(|file| fs::metadata(repo.join(file.path.as_str())).unwrap().len())
        .sum::<u64>();

    write_evidence(
        "s4b1-largest-session.txt",
        &format!(
            "addon=delivery\nmanifest_paths={}\npayload_bytes={payload_bytes}\nsession_files={session_files}\nsession_bytes={session_bytes}\n",
            old.files.len(),
        ),
    );
    assert!(session.join("session.json").is_file());
    assert!(session_files >= old.files.len() * 2 + 2);
    assert!(session_bytes > payload_bytes);
}

/// Candidate material rejects symlinks and path escapes on both stage and load.
#[cfg(unix)]
#[test]
fn candidate_material_rejects_symlinks_and_escapes() {
    let tmp = tempfile::tempdir().unwrap();
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret.md"), b"outside\n").unwrap();

    // Stage: a declared payload path that is a symlink is refused, and the
    // refusal leaves no session behind.
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    fs::remove_file(fixture.next_payload.join(CLEAN)).unwrap();
    std::os::unix::fs::symlink(outside.join("secret.md"), fixture.next_payload.join(CLEAN))
        .unwrap();
    let error = FileSystemAddOnApplier
        .stage(
            workspace,
            &AddOnStageRequest {
                descriptor: &fixture.descriptor,
                payload_root: &fixture.next_payload,
                plan: &fixture.plan,
            },
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("symlink"), "stage: got {error}");
    assert!(
        !session_root(workspace).exists(),
        "a refused stage must leave no session"
    );

    // Load: a candidate file replaced by a symlink is refused.
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    stage(&fixture);
    let candidate = session_root(workspace).join("candidate").join(CLEAN);
    fs::remove_file(&candidate).unwrap();
    std::os::unix::fs::symlink(outside.join("secret.md"), &candidate).unwrap();
    let before = surfaces(workspace);
    let error = FileSystemAddOnApplier
        .resume(workspace, &fixture.descriptor.name)
        .unwrap_err()
        .to_string();
    assert!(error.contains("symlink"), "load file: got {error}");
    assert_surfaces_unchanged(&before, workspace, "candidate symlink");

    // Load: a symlinked directory component that escapes the session is
    // refused, so a declared path cannot be resolved outside the session.
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    stage(&fixture);
    let components = Path::new(CLEAN).parent().unwrap();
    let directory = session_root(workspace).join("candidate").join(components);
    fs::remove_dir_all(&directory).unwrap();
    std::os::unix::fs::symlink(&outside, &directory).unwrap();
    let before = surfaces(workspace);
    let error = FileSystemAddOnApplier
        .resume(workspace, &fixture.descriptor.name)
        .unwrap_err()
        .to_string();
    assert!(error.contains("symlink"), "load dir: got {error}");
    assert_surfaces_unchanged(&before, workspace, "candidate directory symlink");

    // Load: an extra candidate path is a path-set mismatch, and relative paths
    // with a `..` component are structurally rejected by `RelativePath`.
    assert!(RelativePath::parse("../escape.md").is_err());
    assert!(RelativePath::parse("a/../../b").is_err());
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    stage(&fixture);
    write_bytes(
        workspace,
        &format!("{ADDON_SESSION}/{ADDON}/candidate/extra.md"),
        b"extra\n",
    );
    let before = surfaces(workspace);
    let error = FileSystemAddOnApplier
        .resume(workspace, &fixture.descriptor.name)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("path set differs"),
        "load extra: got {error}"
    );
    assert_surfaces_unchanged(&before, workspace, "extra candidate path");

    write_evidence(
        "s4b1-candidate-safety.txt",
        &format!(
            "stage_symlink_refused=true\nload_file_symlink_refused=true\nload_dir_symlink_refused=true\nextra_path_refused=true\nrelative_path_escape_rejected={}\n",
            RelativePath::parse("../escape.md").is_err(),
        ),
    );
}

fn write_evidence(name: &str, body: &str) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/s4b1-evidence");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(name), body).unwrap();
}
