mod common;

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use truss::application::{AddOnInstallRequest, AddOnPayloadPort, AddOnPayloadSpec, AddOnStatePort};
use truss::domain::{AddOnDescriptor, AddOnName, AddOnState};
use truss::infrastructure::{
    addon_observation_witness, FileSystemAddOnPayload, FileSystemAddOnState,
};

use common::{seed_core_state, snapshot_digest, workspace_snapshot, write_file};

/// Fixture payload: three files across three directories.
const FIXTURE_FILES: &[(&str, &str)] = &[
    (".agents/skills/demo/SKILL.md", "demo skill\n"),
    (".agents/skills/demo/agents/openai.yaml", "name: demo\n"),
    (".agents/skills/demo/references/notes.md", "notes\n"),
];

/// Fixture manifest: the membership owner for the fixture payload.
const FIXTURE_MANIFEST: &str = "\
# fixture add-on\n\
.agents/skills/demo/SKILL.md\n\
.agents/skills/demo/agents/openai.yaml\n\
.agents/skills/demo/references/notes.md\n";

const SOURCE_REF: &str = "truss-v0.1.13";

fn write_fixture_payload(root: &Path) {
    for (relative, content) in FIXTURE_FILES {
        write_file(root, relative, content);
    }
}

fn describe(payload: &Path, manifest: &Path, name: &str) -> AddOnDescriptor {
    let foreign: [PathBuf; 0] = [];
    let spec = AddOnPayloadSpec {
        root: payload,
        manifest,
        name,
        source_ref: SOURCE_REF,
        source_core_version: "0.1.13",
        foreign_manifests: &foreign,
    };
    FileSystemAddOnPayload.describe(&spec).unwrap()
}

fn request<'a>(descriptor: &'a AddOnDescriptor, payload_root: &'a Path) -> AddOnInstallRequest<'a> {
    AddOnInstallRequest {
        descriptor,
        payload_root,
    }
}

/// Reload checks the record against the staged payload bytes, never against
/// the consumer workspace.
fn assert_record_matches_payload(
    record: &AddOnState,
    workspace: &Path,
    payload: &Path,
    name: &str,
) -> Result<(), String> {
    let name = AddOnName::parse(name).map_err(|error| error.to_string())?;
    let installation = record
        .installation(&name)
        .ok_or_else(|| format!("record has no add-on named {name}"))?;
    for file in &installation.files {
        let expected = fs::read(payload.join(file.path.as_str()))
            .map_err(|error| format!("payload {} unreadable: {error}", file.path))?;
        let digest = format!("{:x}", Sha256::digest(&expected));
        if digest != file.hash.as_str() {
            return Err(format!(
                "recorded baseline digest {} for {} is not the payload digest {}",
                file.hash.as_str(),
                file.path,
                digest
            ));
        }
        if file.content != expected {
            return Err(format!(
                "recorded baseline bytes for {} are not the payload bytes",
                file.path
            ));
        }
        let copy = workspace
            .join(".truss-core/base-addons")
            .join(name.as_str())
            .join(file.path.as_str());
        let copied = fs::read(&copy)
            .map_err(|error| format!("baseline copy {} unreadable: {error}", copy.display()))?;
        if copied != expected {
            return Err(format!(
                "baseline copy for {} is not the payload bytes",
                file.path
            ));
        }
    }
    Ok(())
}

/// Write a durable evidence file under the git-ignored `target/s2-evidence/`
/// directory so the recorded output outlives the test run.
fn evidence(name: &str, body: &str) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/s2-evidence");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(name), body).unwrap();
}

/// A fixture with the valid pre-existing core state decision 0003 clause 8
/// requires. The state root, `.gitignore`, and lock are written by
/// `common::seed_core_state`.
fn fixture(tmp: &Path, name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let fixture = bare_fixture(tmp, name);
    seed_core_state(&fixture.1);
    fixture
}

/// A workspace with no core state yet; add-on apply must refuse it untouched.
fn bare_fixture(tmp: &Path, name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let payload = tmp.join(format!("{name}-payload"));
    let workspace = tmp.join(format!("{name}-workspace"));
    let manifest = tmp.join(format!("{name}-install-files.txt"));
    write_fixture_payload(&payload);
    fs::create_dir_all(&workspace).unwrap();
    fs::write(&manifest, FIXTURE_MANIFEST).unwrap();
    (payload, workspace, manifest)
}

/// Acceptance row 1: a successful install writes the record after the files
/// exist, and the record matches the payload.
#[test]
fn install_writes_the_record_after_the_files_and_it_matches_the_payload() {
    let tmp = tempfile::tempdir().unwrap();
    let (payload, workspace, manifest) = fixture(tmp.path(), "install");
    let descriptor = describe(&payload, &manifest, "demo");

    let receipt = FileSystemAddOnState
        .apply(&workspace, &request(&descriptor, &payload))
        .unwrap();
    assert!(!receipt.adopted, "a fresh install must not report adoption");

    let record = FileSystemAddOnState
        .load(&workspace)
        .unwrap()
        .expect("the record must exist after a successful install");
    assert_eq!(record.schema_version, AddOnState::SCHEMA_VERSION);
    assert_record_matches_payload(&record, &workspace, &payload, "demo").unwrap();

    // The record is a separate file with its own schema; the core manifest is
    // not created and its schema does not move.
    assert!(workspace.join(".truss-core/addons.json").is_file());
    assert!(
        !workspace.join(".truss-core/manifest.json").exists(),
        "the add-on record must not create or rewrite the core manifest"
    );

    // Every managed path exists with the recorded digest, and every baseline
    // copy equals the payload bytes.
    let name = AddOnName::parse("demo").unwrap();
    let installation = record.installation(&name).unwrap();
    for file in &installation.files {
        let installed = fs::read(workspace.join(file.path.as_str())).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&installed)),
            file.hash.as_str(),
            "installed {} does not match the recorded digest",
            file.path
        );
        let baseline = fs::read(
            workspace
                .join(".truss-core/base-addons/demo")
                .join(file.path.as_str()),
        )
        .unwrap();
        assert_eq!(
            baseline,
            fs::read(payload.join(file.path.as_str())).unwrap(),
            "baseline copy for {} is not the payload byte",
            file.path
        );
    }

    // A consumer edit after the install must not move the recorded baseline.
    write_file(&workspace, FIXTURE_FILES[0].0, "consumer edit\n");
    let reloaded = FileSystemAddOnState.load(&workspace).unwrap().unwrap();
    assert_record_matches_payload(&reloaded, &workspace, &payload, "demo").unwrap();
    let first = reloaded
        .installation(&name)
        .unwrap()
        .files
        .iter()
        .find(|file| file.path.as_str() == FIXTURE_FILES[0].0)
        .unwrap();
    let local = fs::read(workspace.join(FIXTURE_FILES[0].0)).unwrap();
    assert_ne!(
        format!("{:x}", Sha256::digest(&local)),
        first.hash.as_str(),
        "the local edit must differ from the recorded payload digest"
    );

    evidence(
        "s2-install-record.txt",
        &format!(
            "workspace={}\nrecord={}\n",
            workspace.display(),
            fs::read_to_string(workspace.join(".truss-core/addons.json")).unwrap()
        ),
    );
}

/// Acceptance row 2: a legacy install whose files exactly equal the supplied
/// ref is adopted.
#[test]
fn legacy_install_is_adopted_when_every_managed_path_matches() {
    let tmp = tempfile::tempdir().unwrap();
    let (payload, workspace, manifest) = fixture(tmp.path(), "adopt");
    write_fixture_payload(&workspace);
    let descriptor = describe(&payload, &manifest, "demo");

    assert!(FileSystemAddOnState.load(&workspace).unwrap().is_none());
    let receipt = FileSystemAddOnState
        .apply(&workspace, &request(&descriptor, &payload))
        .unwrap();
    assert!(receipt.adopted, "an exact legacy install must be adopted");

    let record = FileSystemAddOnState.load(&workspace).unwrap().unwrap();
    assert_record_matches_payload(&record, &workspace, &payload, "demo").unwrap();
}

/// Acceptance row 2: a legacy install with a single edited byte stops.
#[test]
fn legacy_install_with_one_edited_byte_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let (payload, workspace, manifest) = fixture(tmp.path(), "edited");
    write_fixture_payload(&workspace);
    write_file(&workspace, FIXTURE_FILES[0].0, "consumer edit\n");
    let descriptor = describe(&payload, &manifest, "demo");

    let record_before = FileSystemAddOnState.load(&workspace).unwrap();
    assert!(record_before.is_none());
    assert!(!workspace.join(".truss-core/addons.json").exists());

    let error = FileSystemAddOnState
        .apply(&workspace, &request(&descriptor, &payload))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("add-on adoption mismatch"),
        "expected an adoption refusal, got: {error}"
    );

    let record_after = FileSystemAddOnState.load(&workspace).unwrap();
    assert!(record_after.is_none());
    assert!(!workspace.join(".truss-core/addons.json").exists());

    evidence(
        "s2-adoption-refusal.txt",
        &format!(
            "refusal={error}\nrecord_before_present={}\nrecord_after_present={}\n",
            record_before.is_some(),
            record_after.is_some()
        ),
    );
}

/// Acceptance row 3 (direct refusal): a refusal reached during preflight
/// performs no mutation, including no change to the pre-existing core state.
#[test]
fn adoption_stop_writes_no_record_and_leaves_the_tree_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let (payload, workspace, manifest) = fixture(tmp.path(), "stop");
    write_fixture_payload(&workspace);
    write_file(&workspace, FIXTURE_FILES[1].0, "consumer edit\n");
    let descriptor = describe(&payload, &manifest, "demo");

    assert!(
        workspace.join(".truss-core/lock").is_file(),
        "the fixture must start from a valid core state"
    );
    let before = workspace_snapshot(&workspace);
    let error = FileSystemAddOnState
        .apply(&workspace, &request(&descriptor, &payload))
        .unwrap_err()
        .to_string();
    let after = workspace_snapshot(&workspace);

    assert_eq!(
        before, after,
        "a refused adoption must leave the fixture tree unchanged"
    );
    assert!(
        workspace.join(".truss-core").exists(),
        "the pre-existing core state must be untouched, not removed"
    );
    assert!(!workspace.join(".truss-core/addons.json").exists());
    assert!(!workspace.join(".truss-core/base-addons").exists());

    evidence(
        "s2-stop-tree-hashes.txt",
        &format!(
            "before={}\nafter={}\nrefusal={error}\naddons_json_present={}\n",
            snapshot_digest(&before),
            snapshot_digest(&after),
            workspace.join(".truss-core/addons.json").exists()
        ),
    );
}

/// Acceptance row 3, first amended fixture: with no core state at all, add-on
/// apply refuses and creates nothing.
///
/// This is the fixture that rejects the bootstrap-on-refusal implementation:
/// `2968c03` called `fs::create_dir_all(&state_root)` and
/// `ensure_state_ignore(&state_root)` before its locked refusal, so it left
/// `.truss-core/`, `.truss-core/.gitignore`, and normally
/// `.truss-core/lock` behind and changes this snapshot.
#[test]
fn addon_apply_without_core_state_refuses_without_mutation() {
    let tmp = tempfile::tempdir().unwrap();
    let (payload, workspace, manifest) = bare_fixture(tmp.path(), "no-core");
    write_fixture_payload(&workspace);
    let descriptor = describe(&payload, &manifest, "demo");

    let before = workspace_snapshot(&workspace);
    let error = FileSystemAddOnState
        .apply(&workspace, &request(&descriptor, &payload))
        .unwrap_err()
        .to_string();
    let after = workspace_snapshot(&workspace);

    assert_eq!(
        before, after,
        "a missing core state must be refused without mutation"
    );
    assert!(
        !workspace.join(".truss-core").exists(),
        "add-on apply must never create the core state root"
    );
    assert!(!workspace.join(".truss-core/addons.json").exists());

    evidence(
        "s2b-row3-no-core-state.txt",
        &format!(
            "before={}\nafter={}\nrefusal={error}\ncore_state_present_after={}\n",
            snapshot_digest(&before),
            snapshot_digest(&after),
            workspace.join(".truss-core").exists()
        ),
    );
}

/// Acceptance row 3, second amended fixture: valid core state, preflight
/// passes, then a competing writer changes one managed path before the
/// existing lock is acquired. The authoritative locked observation must refuse
/// and the workspace must end exactly as the competing writer left it, with no
/// `addons.json`, no baseline, and no managed-file write.
///
/// This rejects a preflight-authoritative implementation: one that commits
/// from the preflight verdict writes provenance for the competing edit (or
/// overwrites the edit) and so changes the post-change snapshot.
#[test]
fn locked_observation_refuses_after_preflight_passes_and_preserves_the_competing_change() {
    let tmp = tempfile::tempdir().unwrap();
    let (payload, workspace, manifest) = fixture(tmp.path(), "locked-refuse");
    write_fixture_payload(&workspace);
    let descriptor = describe(&payload, &manifest, "demo");
    let barrier_workspace = workspace.clone();
    let (tx, rx) = std::sync::mpsc::channel();

    addon_observation_witness::arm_barrier(&workspace, move || {
        write_file(&barrier_workspace, FIXTURE_FILES[0].0, "competing edit\n");
        tx.send(workspace_snapshot(&barrier_workspace))
            .expect("the snapshot receiver must still be open");
    });
    let error = FileSystemAddOnState
        .apply(&workspace, &request(&descriptor, &payload))
        .unwrap_err()
        .to_string();
    addon_observation_witness::disarm_barrier(&workspace);

    let post_change = rx
        .try_recv()
        .expect("the barrier must run after preflight and before lock acquisition");
    let after = workspace_snapshot(&workspace);
    assert_eq!(
        after, post_change,
        "the locked refusal must preserve the competing change exactly"
    );
    assert!(
        error.contains("add-on adoption mismatch"),
        "the locked observation must refuse the competing edit, got: {error}"
    );
    assert!(!workspace.join(".truss-core/addons.json").exists());
    assert!(!workspace.join(".truss-core/base-addons").exists());

    evidence(
        "s2b-row3-locked-refusal.txt",
        &format!(
            "post_change={}\nafter={}\nrefusal={error}\naddons_json_present={}\nbase_addons_present={}\n",
            snapshot_digest(&post_change),
            snapshot_digest(&after),
            workspace.join(".truss-core/addons.json").exists(),
            workspace.join(".truss-core/base-addons").exists()
        ),
    );
}

/// Decision 0003 clause 8: a missing, unsafe, or incomplete core state is
/// refused without mutation. Every case starts from an exact legacy install so
/// that a valid core state would adopt; only the prerequisite is broken.
#[test]
fn invalid_core_state_is_refused_without_mutation() {
    /// One broken core-state prerequisite: a label and the mutation that
    /// breaks it.
    type Case = (&'static str, fn(&Path));

    let mut cases: Vec<Case> = Vec::new();
    cases.push(("missing-ignore-file", |workspace: &Path| {
        fs::remove_file(workspace.join(".truss-core/.gitignore")).unwrap();
    }));
    cases.push(("missing-ignore-rule", |workspace: &Path| {
        fs::write(workspace.join(".truss-core/.gitignore"), "/lock\n").unwrap();
    }));
    cases.push(("missing-lock", |workspace: &Path| {
        fs::remove_file(workspace.join(".truss-core/lock")).unwrap();
    }));
    cases.push(("non-regular-lock", |workspace: &Path| {
        let lock = workspace.join(".truss-core/lock");
        fs::remove_file(&lock).unwrap();
        fs::create_dir(&lock).unwrap();
    }));
    cases.push(("state-root-is-a-file", |workspace: &Path| {
        fs::remove_dir_all(workspace.join(".truss-core")).unwrap();
        fs::write(workspace.join(".truss-core"), b"not a state root").unwrap();
    }));
    #[cfg(unix)]
    {
        cases.push(("symlinked-lock", |workspace: &Path| {
            let lock = workspace.join(".truss-core/lock");
            fs::remove_file(&lock).unwrap();
            std::os::unix::fs::symlink("real-lock", &lock).unwrap();
        }));
        cases.push(("symlinked-state-root", |workspace: &Path| {
            let state_root = workspace.join(".truss-core");
            let real = workspace.join(".truss-core-real");
            fs::rename(&state_root, &real).unwrap();
            std::os::unix::fs::symlink(&real, &state_root).unwrap();
        }));
    }

    let mut observed = String::new();
    for (label, break_it) in cases {
        let tmp = tempfile::tempdir().unwrap();
        let (payload, workspace, manifest) = fixture(tmp.path(), label);
        write_fixture_payload(&workspace);
        let descriptor = describe(&payload, &manifest, "demo");
        break_it(&workspace);

        let before = workspace_snapshot(&workspace);
        let error = FileSystemAddOnState
            .apply(&workspace, &request(&descriptor, &payload))
            .unwrap_err()
            .to_string();
        let after = workspace_snapshot(&workspace);

        assert_eq!(
            before, after,
            "{label}: an invalid core state must be refused without mutation"
        );
        assert!(
            !workspace.join(".truss-core/addons.json").exists(),
            "{label}"
        );
        assert!(
            !workspace.join(".truss-core/base-addons").exists(),
            "{label}"
        );
        observed.push_str(&format!("{label}: {error}\n"));
    }
    evidence("s2b-invalid-core-state.txt", &observed);
}

/// Acceptance row 2: an extra managed local path the payload does not declare
/// also stops adoption.
#[test]
fn legacy_install_with_an_extra_managed_path_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let (payload, workspace, manifest) = fixture(tmp.path(), "extra");
    write_fixture_payload(&workspace);
    write_file(
        &workspace,
        ".agents/skills/demo/NOTES.md",
        "consumer notes\n",
    );
    let descriptor = describe(&payload, &manifest, "demo");

    let error = FileSystemAddOnState
        .apply(&workspace, &request(&descriptor, &payload))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("is not declared by the payload"),
        "expected an extra-path refusal, got: {error}"
    );
    assert!(!workspace.join(".truss-core/addons.json").exists());
}

/// Synchronization proof for the Blocking finding: the observation that
/// authorizes `commit` runs while the shared `.truss-core/lock` is held, so a
/// competing writer cannot slip a consumer edit between observation and write.
///
/// The witness spawns the competing writer from inside `observe` and joins it
/// before the observation reads the workspace, so the test does not depend on
/// thread timing: the writer either finds the lock held and is refused, or it
/// takes the lock and mutates the managed path, which the observation must
/// then refuse. The present-but-wrong implementation it rejects is the one
/// that observes before acquiring the lock, where the competing writer takes
/// the free lock and the exact fixture is refused instead of adopted.
#[test]
fn adoption_observation_holds_the_shared_lock_against_a_competing_writer() {
    let tmp = tempfile::tempdir().unwrap();
    let (payload, workspace, manifest) = fixture(tmp.path(), "locked-adopt");
    write_fixture_payload(&workspace);
    let descriptor = describe(&payload, &manifest, "demo");
    let witness = tmp.path().join("s2-lock-witness.txt");
    let target = workspace.join(FIXTURE_FILES[0].0);
    let original = fs::read(&target).unwrap();

    addon_observation_witness::arm(&workspace, &witness, &target);
    let receipt = FileSystemAddOnState
        .apply(&workspace, &request(&descriptor, &payload))
        .unwrap();
    addon_observation_witness::disarm(&workspace);

    assert!(receipt.adopted, "an exact legacy install must be adopted");
    let outcome = fs::read_to_string(&witness).unwrap();
    assert_eq!(
        outcome.trim(),
        addon_observation_witness::SHARED_LOCK_HELD,
        "a competing writer reached the workspace during observation"
    );
    assert_eq!(
        fs::read(&target).unwrap(),
        original,
        "the competing writer must not alter a managed consumer file"
    );
    let record = FileSystemAddOnState.load(&workspace).unwrap().unwrap();
    assert_record_matches_payload(&record, &workspace, &payload, "demo").unwrap();

    evidence(
        "s2-lock-witness.txt",
        &format!(
            "competing_writer={}\nconsumer_file_unchanged=true\nadopted={}\n",
            outcome.trim(),
            receipt.adopted
        ),
    );
}

/// The same proof for the fresh-install half of the finding: a file that
/// appears during observation could be overwritten by the commit, so the
/// observation that decides `fresh` must run under the lock too.
#[test]
fn fresh_install_observation_holds_the_shared_lock_against_a_competing_writer() {
    let tmp = tempfile::tempdir().unwrap();
    let (payload, workspace, manifest) = fixture(tmp.path(), "locked-fresh");
    let descriptor = describe(&payload, &manifest, "demo");
    let witness = tmp.path().join("s2-lock-witness-fresh.txt");
    let target = workspace.join(FIXTURE_FILES[0].0);

    addon_observation_witness::arm(&workspace, &witness, &target);
    let receipt = FileSystemAddOnState
        .apply(&workspace, &request(&descriptor, &payload))
        .unwrap();
    addon_observation_witness::disarm(&workspace);

    assert!(!receipt.adopted, "an empty workspace is a fresh install");
    let outcome = fs::read_to_string(&witness).unwrap();
    assert_eq!(
        outcome.trim(),
        addon_observation_witness::SHARED_LOCK_HELD,
        "a competing writer reached the workspace during observation"
    );
    assert_eq!(
        fs::read(&target).unwrap(),
        fs::read(payload.join(FIXTURE_FILES[0].0)).unwrap(),
        "the installed file must be the payload bytes, not competing bytes"
    );
    let record = FileSystemAddOnState.load(&workspace).unwrap().unwrap();
    assert_record_matches_payload(&record, &workspace, &payload, "demo").unwrap();

    evidence(
        "s2-lock-witness-fresh.txt",
        &format!(
            "competing_writer={}\ninstalled_bytes_are_payload=true\nadopted={}\n",
            outcome.trim(),
            receipt.adopted
        ),
    );
}

/// Acceptance row 1 counterexample, first form: the wrong writer records the
/// consumer workspace bytes as upstream. The payload comparison is what
/// catches it, so this test runs the assertion and is expected to panic.
#[test]
#[should_panic(expected = "is not the payload digest")]
fn wrong_writer_that_records_target_bytes_fails_the_payload_comparison() {
    let tmp = tempfile::tempdir().unwrap();
    let (record, payload) = install_with_wrong_writer(tmp.path());
    assert_record_matches_payload(
        &record,
        &tmp.path().join("wrong-workspace"),
        &payload,
        "demo",
    )
    .unwrap();
}

/// The same counterexample as a non-panicking assertion, so the red is durable
/// and its exact message is recorded as evidence.
#[test]
fn consumer_edit_is_caught_when_it_is_recorded_as_upstream() {
    let tmp = tempfile::tempdir().unwrap();
    let (record, payload) = install_with_wrong_writer(tmp.path());
    let error = assert_record_matches_payload(
        &record,
        &tmp.path().join("wrong-workspace"),
        &payload,
        "demo",
    )
    .unwrap_err();
    assert!(
        error.contains("is not the payload digest"),
        "expected the payload comparison to reject the wrong writer, got: {error}"
    );
    evidence("s2-wrong-writer.txt", &format!("observed_red={error}\n"));
}

/// A deliberately wrong writer: it stages the consumer workspace bytes as if
/// they were the payload, so the record would bless a consumer edit as
/// upstream. The real writer never does this because the payload root is the
/// operator-supplied immutable ref, not the workspace.
fn install_with_wrong_writer(tmp: &Path) -> (AddOnState, PathBuf) {
    let real_payload = tmp.join("real-payload");
    let workspace = tmp.join("wrong-workspace");
    let manifest = tmp.join("wrong-install-files.txt");
    write_fixture_payload(&real_payload);
    write_fixture_payload(&workspace);
    seed_core_state(&workspace);
    fs::write(&manifest, FIXTURE_MANIFEST).unwrap();
    // The consumer edits one managed file.
    write_file(&workspace, FIXTURE_FILES[0].0, "consumer edit\n");
    // The wrong writer treats the workspace bytes as the immutable ref.
    let wrong = describe(&workspace, &manifest, "demo");
    FileSystemAddOnState
        .apply(&workspace, &request(&wrong, &workspace))
        .unwrap();
    let record = FileSystemAddOnState.load(&workspace).unwrap().unwrap();
    (record, real_payload)
}
