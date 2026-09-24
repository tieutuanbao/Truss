//! S4 scoped conflict recovery acceptance rows, driven from outside the crate.
//!
//! Row 1 stages a conflicted add-on plan and proves the session is recorded
//! under `.truss-core/addon-update/<name>/` while the workspace, the baseline,
//! `addons.json`, and any core session are byte-identical.
//!
//! Row 2 changes an unrelated frozen managed path between staging and resume:
//! resume must refuse with the competing bytes intact and no provenance write.
//! A second fixture resolves the conflict and resumes successfully, proving the
//! session is the only thing cleared and that a seeded core session survives.
//!
//! Row 3 seeds a second add-on session and a core session, aborts the owned one
//! twice, and requires the owned session gone, the sibling and core sessions
//! byte-identical, and the workspace, baseline, and `addons.json` unchanged.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use truss::application::{
    AddOnInstallRequest, AddOnPayloadPort, AddOnPayloadSpec, AddOnPlanRequest, AddOnResumeRequest,
    AddOnStageRequest, AddOnStatePort,
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

const BASE: &[u8] = b"one\ntwo\nthree\n";
const LOCAL_OVERLAP: &[u8] = b"local overlap\n";
const NEXT_OVERLAP: &[u8] = b"next overlap\n";
const CLEAN_NEXT: &[u8] = b"clean next\n";
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

/// The conflicted fixture used by rows 1, 2, and 3: one overlap conflict plus
/// one clean `Update`, with a carrier and an unrelated preserve path.
struct ConflictFixture {
    _tmp: tempfile::TempDir,
    workspace: PathBuf,
    next_payload: PathBuf,
    descriptor: AddOnDescriptor,
    plan: UpdatePlan,
}

fn conflict_fixture() -> ConflictFixture {
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
            (CLEAN, BASE),
            (UNRELATED, UNRELATED_BYTES),
        ],
    );
    write_bytes(&workspace, CONFLICT, LOCAL_OVERLAP);

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
    assert!(
        !plan.mutations.is_empty(),
        "the fixture carries one clean change"
    );
    ConflictFixture {
        _tmp: tmp,
        workspace,
        next_payload,
        descriptor,
        plan,
    }
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

/// A workspace snapshot that can skip the add-on session namespace, so a stage
/// or abort can be proven to change nothing else.
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

/// Acceptance row 1: staging persists the session under the add-on namespace
/// with its resolution inputs and frozen observations, and changes nothing
/// else. `.truss-core/update/` stays untouched.
#[test]
fn staging_persists_the_session_and_mutates_nothing() {
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    // A core session sentinel proves the core namespace is never read or
    // written by an add-on stage.
    seed_core_session(workspace);

    let addons = workspace.join(".truss-core/addons.json");
    let baseline_root = workspace.join(".truss-core/base-addons");
    let core_session = workspace.join(".truss-core/update");

    let before_workspace = snapshot_without(workspace, &[ADDON_SESSION]);
    let before_addons = fs::read(&addons).unwrap();
    let before_baseline = snapshot_digest(&workspace_snapshot(&baseline_root));
    let before_core = workspace_snapshot(&core_session);

    FileSystemAddOnApplier
        .stage(
            workspace,
            &AddOnStageRequest {
                descriptor: &fixture.descriptor,
                plan: &fixture.plan,
            },
        )
        .unwrap();

    // The session is present, and its DTO names every conflict and frozen path.
    let session_path = session_root(workspace).join("session.json");
    assert!(session_path.is_file(), "the session must be persisted");
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(&session_path).unwrap()).unwrap();
    assert_eq!(record["from_version"], OLD_REF);
    assert_eq!(record["to_version"], NEW_REF);
    let conflict_paths = record["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["path"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(conflict_paths, vec![CONFLICT.to_owned()]);
    assert_eq!(
        record["frozen_files"].as_array().unwrap().len(),
        fixture.plan.frozen_files.len()
    );

    // Every resolution input round-trips byte-for-byte from the plan.
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
    // And every frozen observation round-trips, absent paths included.
    for frozen in &fixture.plan.frozen_files {
        let expected = frozen.content.clone().unwrap_or_default();
        if frozen.content.is_some() {
            assert_eq!(
                session_bytes(workspace, "frozen", frozen.path.as_str()),
                expected,
                "frozen bytes for {}",
                frozen.path
            );
        }
    }

    // Nothing else changed. The session namespace is excluded because staging
    // legitimately creates it; everything else must be byte-identical.
    let after_workspace = snapshot_without(workspace, &[ADDON_SESSION]);
    let after_addons = fs::read(&addons).unwrap();
    let after_baseline = snapshot_digest(&workspace_snapshot(&baseline_root));
    let after_core = workspace_snapshot(&core_session);

    assert_eq!(
        before_workspace, after_workspace,
        "staging must not change any managed file, baseline, or provenance"
    );
    assert_eq!(before_addons, after_addons, "staging changed addons.json");
    assert_eq!(
        before_baseline, after_baseline,
        "staging changed the baseline tree"
    );
    assert_eq!(before_core, after_core, ".truss-core/update/ was touched");
    // The clean change is staged, never applied.
    assert_eq!(fs::read(workspace.join(CLEAN)).unwrap(), BASE);
    assert_eq!(fs::read(workspace.join(CONFLICT)).unwrap(), LOCAL_OVERLAP);

    write_evidence(
        "s4-row1-stage.txt",
        &format!(
            "conflicts={}\nfrozen={}\nclean_change={:?}\nclean_mutation={}\nworkspace_excluding_session_equal={}\nbaseline_equal={}\naddons_equal={}\ncore_session_equal={}\nclean_path_unchanged={}\n",
            record["conflicts"].as_array().unwrap().len(),
            record["frozen_files"].as_array().unwrap().len(),
            kind(&fixture.plan, CLEAN),
            mutation(&fixture.plan, CLEAN).len(),
            before_workspace == after_workspace,
            before_baseline == after_baseline,
            before_addons == after_addons,
            before_core == after_core,
            fs::read(workspace.join(CLEAN)).unwrap() == BASE,
        ),
    );
}

/// Acceptance row 2, first fixture: a change to an unrelated frozen managed path
/// between staging and resume refuses with the competing bytes intact and no
/// provenance write.
#[test]
fn resume_refuses_unrelated_frozen_drift() {
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    seed_core_session(workspace);
    FileSystemAddOnApplier
        .stage(
            workspace,
            &AddOnStageRequest {
                descriptor: &fixture.descriptor,
                plan: &fixture.plan,
            },
        )
        .unwrap();

    let addons = workspace.join(".truss-core/addons.json");
    let baseline_root = workspace.join(".truss-core/base-addons");
    let before_addons = fs::read(&addons).unwrap();
    let before_baseline = snapshot_digest(&workspace_snapshot(&baseline_root));

    // The barrier: CARRIER is frozen and not the conflicted path. Change it.
    write_bytes(workspace, CARRIER, COMPETING);
    let drifted = snapshot_without(workspace, &[ADDON_SESSION]);

    let error = FileSystemAddOnApplier
        .resume(
            workspace,
            &AddOnResumeRequest {
                descriptor: &fixture.descriptor,
                payload_root: &fixture.next_payload,
            },
        )
        .unwrap_err()
        .to_string();

    assert!(
        error.contains("workspace changed"),
        "expected a frozen-drift refusal, got: {error}"
    );
    assert_eq!(
        fs::read(workspace.join(CARRIER)).unwrap(),
        COMPETING,
        "the competing bytes must survive the refusal"
    );
    assert_eq!(
        snapshot_without(workspace, &[ADDON_SESSION]),
        drifted,
        "the refused resume changed the workspace"
    );
    assert_eq!(
        fs::read(&addons).unwrap(),
        before_addons,
        "the refused resume wrote provenance"
    );
    assert_eq!(
        snapshot_digest(&workspace_snapshot(&baseline_root)),
        before_baseline,
        "the refused resume changed the baseline"
    );
    assert!(
        session_root(workspace).join("session.json").is_file(),
        "the refused resume must keep the session for a retry or abort"
    );

    write_evidence(
        "s4-row2-refusal.txt",
        &format!(
            "target={CARRIER}\nobserved_bytes={}\ncompeting_bytes={}\nrefusal={error}\ncompetitor_survives={}\nworkspace_unchanged_by_refusal={}\naddons_equal={}\nbaseline_equal={}\nsession_alive={}\n",
            digest(CARRIER_BYTES),
            digest(COMPETING),
            fs::read(workspace.join(CARRIER)).unwrap() == COMPETING,
            snapshot_without(workspace, &[ADDON_SESSION]) == drifted,
            fs::read(&addons).unwrap() == before_addons,
            snapshot_digest(&workspace_snapshot(&baseline_root)) == before_baseline,
            session_root(workspace).join("session.json").is_file(),
        ),
    );
}

/// Acceptance row 2, second fixture: resolving every conflict and resuming
/// applies all changes, writes provenance, and clears only the owned session.
#[test]
fn resume_applies_the_resolution_and_clears_only_the_owned_session() {
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    seed_core_session(workspace);
    FileSystemAddOnApplier
        .stage(
            workspace,
            &AddOnStageRequest {
                descriptor: &fixture.descriptor,
                plan: &fixture.plan,
            },
        )
        .unwrap();
    // The operator resolution replaces the staged conflict-marked bytes.
    write_bytes(
        workspace,
        &format!("{ADDON_SESSION}/{ADDON}/resolved/{CONFLICT}"),
        RESOLVED,
    );
    let core_session = workspace.join(".truss-core/update");
    let before_core = workspace_snapshot(&core_session);

    let receipt = FileSystemAddOnApplier
        .resume(
            workspace,
            &AddOnResumeRequest {
                descriptor: &fixture.descriptor,
                payload_root: &fixture.next_payload,
            },
        )
        .unwrap();
    assert!(receipt.backup_path.is_some());

    // Workspace: the conflict takes the resolution, the clean change lands, the
    // carrier and unrelated preserve paths are untouched.
    assert_eq!(fs::read(workspace.join(CONFLICT)).unwrap(), RESOLVED);
    assert_eq!(fs::read(workspace.join(CLEAN)).unwrap(), CLEAN_NEXT);
    assert_eq!(fs::read(workspace.join(CARRIER)).unwrap(), CARRIER_BYTES);
    assert_eq!(
        fs::read(workspace.join(UNRELATED)).unwrap(),
        UNRELATED_BYTES
    );

    // Baseline: the payload bytes per path, not the resolved workspace bytes.
    assert_eq!(
        baseline_bytes(workspace, CONFLICT).unwrap(),
        NEXT_OVERLAP,
        "the baseline must record the payload bytes"
    );
    assert_eq!(baseline_bytes(workspace, CLEAN).unwrap(), CLEAN_NEXT);
    assert_eq!(baseline_bytes(workspace, CARRIER).unwrap(), CARRIER_BYTES);
    assert_eq!(
        baseline_bytes(workspace, UNRELATED).unwrap(),
        UNRELATED_BYTES
    );

    // Provenance: the new ref, new core version, and payload digests.
    let record: serde_json::Value =
        serde_json::from_slice(&fs::read(workspace.join(".truss-core/addons.json")).unwrap())
            .unwrap();
    let demo = &record["addons"].as_array().unwrap()[0];
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
            (CONFLICT.to_owned(), digest(NEXT_OVERLAP)),
            (CLEAN.to_owned(), digest(CLEAN_NEXT)),
            (UNRELATED.to_owned(), digest(UNRELATED_BYTES)),
        ]
    );

    // Only the owned session is cleared; a seeded core session survives, and no
    // transaction journal or backup is left behind.
    assert!(
        !session_root(workspace).exists(),
        "the owned add-on session must be cleared"
    );
    assert_eq!(
        workspace_snapshot(&core_session),
        before_core,
        "resume must not touch the core session"
    );
    assert!(!workspace.join(".truss-core/transaction.json").exists());
    // A successful apply keeps its own backup snapshot and reports it.
    let backup = workspace.join(receipt.backup_path.as_deref().unwrap());
    assert!(
        backup.is_dir(),
        "the successful resume must keep its backup"
    );
    assert_eq!(
        fs::read_dir(workspace.join(".truss-backup"))
            .unwrap()
            .count(),
        1,
        "a successful resume must keep exactly its own backup"
    );

    write_evidence(
        "s4-row2-resume.txt",
        &format!(
            "conflict_workspace={}\nconflict_baseline={}\nclean_workspace={}\ncarrier={}\nsource_ref={NEW_REF}\nsource_core_version={NEW_CORE}\nsession_cleared={}\ncore_session_equal={}\nbackup={}\n",
            digest(&fs::read(workspace.join(CONFLICT)).unwrap()),
            digest(&baseline_bytes(workspace, CONFLICT).unwrap()),
            digest(&fs::read(workspace.join(CLEAN)).unwrap()),
            digest(&fs::read(workspace.join(CARRIER)).unwrap()),
            !session_root(workspace).exists(),
            workspace_snapshot(&core_session) == before_core,
            receipt.backup_path.is_some(),
        ),
    );
}

/// Acceptance row 3: abort removes only the owned add-on session, leaves the
/// workspace, baseline, `addons.json`, the sibling add-on session, and the core
/// session untouched, and is safe to repeat.
#[test]
fn abort_is_scoped_and_idempotent() {
    let fixture = conflict_fixture();
    let workspace = &fixture.workspace;
    seed_core_session(workspace);
    // A sibling add-on session, so removing the shared container cannot be
    // mistaken for removing only the owned session.
    write_bytes(workspace, SIBLING_SESSION, SIBLING_BYTES);
    FileSystemAddOnApplier
        .stage(
            workspace,
            &AddOnStageRequest {
                descriptor: &fixture.descriptor,
                plan: &fixture.plan,
            },
        )
        .unwrap();
    assert!(session_root(workspace).join("session.json").is_file());

    let addons = workspace.join(".truss-core/addons.json");
    let baseline_root = workspace.join(".truss-core/base-addons");
    let core_session = workspace.join(".truss-core/update");
    let before_workspace = snapshot_without(workspace, &[ADDON_SESSION]);
    let before_addons = fs::read(&addons).unwrap();
    let before_baseline = snapshot_digest(&workspace_snapshot(&baseline_root));
    let before_core = workspace_snapshot(&core_session);

    assert!(
        FileSystemAddOnApplier
            .abort(workspace, &fixture.descriptor.name)
            .unwrap(),
        "the first abort must remove the owned session"
    );
    let after_first = fs::read(workspace.join(SIBLING_SESSION)).unwrap();
    assert!(
        !session_root(workspace).exists(),
        "the owned session must be gone"
    );
    assert_eq!(
        after_first, SIBLING_BYTES,
        "the sibling session was touched"
    );
    assert_eq!(
        fs::read(workspace.join(CORE_SESSION)).unwrap(),
        CORE_SESSION_BYTES,
        "the core session was touched"
    );

    assert!(
        !FileSystemAddOnApplier
            .abort(workspace, &fixture.descriptor.name)
            .unwrap(),
        "the second abort must report that nothing was removed"
    );

    // The three surfaces are unchanged by either abort, and the second abort was
    // a no-op.
    assert_eq!(
        snapshot_without(workspace, &[ADDON_SESSION]),
        before_workspace,
        "abort changed the workspace, baseline, or provenance"
    );
    assert_eq!(fs::read(&addons).unwrap(), before_addons);
    assert_eq!(
        snapshot_digest(&workspace_snapshot(&baseline_root)),
        before_baseline
    );
    assert_eq!(workspace_snapshot(&core_session), before_core);
    assert_eq!(
        fs::read(workspace.join(SIBLING_SESSION)).unwrap(),
        SIBLING_BYTES
    );

    write_evidence(
        "s4-row3-abort.txt",
        &format!(
            "first_removed=true\nsecond_removed=false\nowned_session_present={}\nsibling_present={}\ncore_session_equal={}\nworkspace_equal={}\naddons_equal={}\nbaseline_equal={}\ncarrier={}\nclean={}\n",
            session_root(workspace).exists(),
            workspace.join(SIBLING_SESSION).exists(),
            workspace_snapshot(&core_session) == before_core,
            snapshot_without(workspace, &[ADDON_SESSION]) == before_workspace,
            fs::read(&addons).unwrap() == before_addons,
            snapshot_digest(&workspace_snapshot(&baseline_root)) == before_baseline,
            digest(&fs::read(workspace.join(CARRIER)).unwrap()),
            digest(&fs::read(workspace.join(CLEAN)).unwrap()),
        ),
    );
}

fn write_evidence(name: &str, body: &str) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/s4-evidence");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(name), body).unwrap();
}
