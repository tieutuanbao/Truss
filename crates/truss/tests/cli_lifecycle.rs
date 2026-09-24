//! Core and add-on CLI lifecycle tests.
//!
//! The first two tests are the pre-existing core lifecycle suite: they were not
//! modified by S4b3. Everything below `S4b3` drives the add-on subcommand
//! surface through the shipped binary, against a workspace whose core state was
//! produced by the shipped `install` command.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use sha2::{Digest, Sha256};

mod common;

#[test]
fn cli_installs_reports_and_diagnoses_a_fresh_core() {
    let root = tempfile::tempdir().unwrap();
    let binary = env!("CARGO_BIN_EXE_truss");

    let dry = Command::new(binary)
        .args(["install", "--directory"])
        .arg(root.path())
        .args(["--dry-run", "--json"])
        .output()
        .unwrap();
    assert!(
        dry.status.success(),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
    assert!(!root.path().join(".truss-core").exists());

    let install = Command::new(binary)
        .args(["install", "--directory"])
        .arg(root.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        install.status.success(),
        "{}",
        String::from_utf8_lossy(&install.stderr)
    );
    let output: serde_json::Value = serde_json::from_slice(&install.stdout).unwrap();
    assert_eq!(output["operation"], "install");
    assert_eq!(output["applied"], true);
    assert!(root.path().join("AGENTS.md").is_file());
    assert!(root.path().join(".truss-core/manifest.json").is_file());
    assert!(root.path().join(".truss-core/base/AGENTS.md").is_file());
    assert!(!root.path().join("truss.db").exists());

    let status = Command::new(binary)
        .args(["status", "--directory"])
        .arg(root.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(status.status.success());
    let status: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status["condition"], "current");

    let doctor = Command::new(binary)
        .args(["doctor", "--directory"])
        .arg(root.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(doctor.status.success());
    let doctor: serde_json::Value = serde_json::from_slice(&doctor.stdout).unwrap();
    assert_eq!(doctor["healthy"], true);
}

#[test]
fn install_migrates_an_existing_core_without_overwriting_consumer_content() {
    let root = tempfile::tempdir().unwrap();
    let agents = root.path().join("AGENTS.md");
    fs::write(&agents, "consumer-owned instructions\n").unwrap();
    let before = fs::read(&agents).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_truss"))
        .args(["install", "--directory"])
        .arg(root.path())
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(fs::read(&agents).unwrap(), before);
    let output: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        output["changes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|change| change["path"] == "AGENTS.md")
            .unwrap()["kind"],
        "adopt"
    );
}

// ---------------------------------------------------------------------------
// S4b3 — the add-on subcommand surface.
// ---------------------------------------------------------------------------

const ADDON: &str = "demo";
const ADDON_SKILL: &str = "docs/demo/SKILL.md";
const ADDON_REFERENCE: &str = "docs/demo/reference.md";

const REF_A: &str = "truss-v0.1.13";
const REF_B: &str = "truss-v0.1.14";
const REF_C: &str = "truss-v0.1.15";
const REF_D: &str = "truss-v0.1.16";
const UPDATE_CORE_VERSION: &str = "0.1.14";

const SKILL_A: &[u8] = b"one\ntwo\nthree\n";
const SKILL_B: &[u8] = b"one\ntwo\nthree\nfour\n";
const SKILL_C: &[u8] = b"incoming\n";
const SKILL_D: &[u8] = b"incoming d\n";
const REFERENCE_A: &[u8] = b"reference a\n";
const REFERENCE_B: &[u8] = b"reference b\n";
const REFERENCE_C: &[u8] = b"reference c\n";
const REFERENCE_D: &[u8] = b"reference d\n";
const RESOLVED_SKILL: &[u8] = b"resolved skill\n";
const CORE_SESSION: &[u8] = b"{\"schema_version\":1}\n";

/// The managed surface a conflicted update must not touch: the complete
/// workspace snapshot minus the owned add-on session namespace, which staging
/// is allowed to write.
fn managed_snapshot(workspace: &Path) -> String {
    common::workspace_snapshot(workspace)
        .lines()
        .filter(|line| !line.contains(".truss-core/addon-update"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn path_str(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// Run the shipped binary and record the exact command, exit status, and both
/// streams, so the handoff can quote a real CLI transcript.
fn run(args: &[&str], transcript: &mut String) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_truss"))
        .args(args)
        .output()
        .unwrap();
    transcript.push_str(&format!(
        "$ truss {}\nexit={}\nstdout:\n{}stderr:\n{}\n",
        args.join(" "),
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ));
    output
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not JSON ({error}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn evidence(name: &str, body: &str) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/s4b3-evidence");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(name), body).unwrap();
}

fn write_payload(root: &Path, skill: &[u8], reference: &[u8]) {
    for (relative, bytes) in [(ADDON_SKILL, skill), (ADDON_REFERENCE, reference)] {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
}

struct AddonFixture {
    tmp: tempfile::TempDir,
    workspace: PathBuf,
    payload_a: PathBuf,
    payload_b: PathBuf,
    payload_c: PathBuf,
    payload_d: PathBuf,
    manifest_a: PathBuf,
    manifest_b: PathBuf,
    manifest_c: PathBuf,
    manifest_d: PathBuf,
}

impl AddonFixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let payload_a = tmp.path().join("payload-a");
        let payload_b = tmp.path().join("payload-b");
        let payload_c = tmp.path().join("payload-c");
        let payload_d = tmp.path().join("payload-d");
        write_payload(&payload_a, SKILL_A, REFERENCE_A);
        write_payload(&payload_b, SKILL_B, REFERENCE_B);
        write_payload(&payload_c, SKILL_C, REFERENCE_C);
        write_payload(&payload_d, SKILL_D, REFERENCE_D);
        let manifest_a = tmp.path().join("files-a.txt");
        let manifest_b = tmp.path().join("files-b.txt");
        let manifest_c = tmp.path().join("files-c.txt");
        let manifest_d = tmp.path().join("files-d.txt");
        for manifest in [&manifest_a, &manifest_b, &manifest_c, &manifest_d] {
            fs::write(
                manifest,
                format!("# add-on fixture manifest\n{ADDON_SKILL}\n{ADDON_REFERENCE}\n"),
            )
            .unwrap();
        }
        Self {
            tmp,
            workspace,
            payload_a,
            payload_b,
            payload_c,
            payload_d,
            manifest_a,
            manifest_b,
            manifest_c,
            manifest_d,
        }
    }

    /// A real core install through the shipped command, so the workspace holds
    /// genuine core state and the core-managed files the add-on must coexist
    /// with.
    fn core_state(&self, transcript: &mut String) {
        let output = run(
            &[
                "install",
                "--directory",
                path_str(&self.workspace),
                "--json",
            ],
            transcript,
        );
        assert!(output.status.success(), "{}", stderr(&output));
    }

    fn status(&self, transcript: &mut String) -> Value {
        json(&run(
            &[
                "addon",
                "status",
                "--name",
                ADDON,
                "--directory",
                path_str(&self.workspace),
                "--json",
            ],
            transcript,
        ))
    }

    fn update_args(
        &self,
        command: &'static str,
        source: &Path,
        manifest: &Path,
        source_ref: &str,
    ) -> Vec<String> {
        vec![
            "addon".to_owned(),
            command.to_owned(),
            "--name".to_owned(),
            ADDON.to_owned(),
            "--manifest".to_owned(),
            path_str(manifest).to_owned(),
            "--source".to_owned(),
            path_str(source).to_owned(),
            "--source-ref".to_owned(),
            source_ref.to_owned(),
            "--source-core-version".to_owned(),
            UPDATE_CORE_VERSION.to_owned(),
            "--directory".to_owned(),
            path_str(&self.workspace).to_owned(),
            "--json".to_owned(),
        ]
    }
}

/// Acceptance row 1: the full cycle through the real binary.
///
/// Absent `status`, `install` at ref A, `status`, `update --dry-run` to ref B,
/// `update`, a seeded conflict, the operator edit of the session's `resolved/`
/// content, a payload-free `continue`, and `abort` twice. The reported
/// `source_ref` and per-file digests are checked against the manifest and the
/// payload bytes, and the seeded core session proves abort is add-on scoped.
#[test]
fn cli_addon_cycle_installs_reports_updates_resolves_and_aborts() {
    let fixture = AddonFixture::new();
    let mut transcript = String::new();
    fixture.core_state(&mut transcript);

    // A core conflict session is seeded so the add-on abort can be shown to
    // leave `.truss-core/update/` byte-identical.
    let core_session = fixture.workspace.join(".truss-core/update/session.json");
    fs::create_dir_all(core_session.parent().unwrap()).unwrap();
    fs::write(&core_session, CORE_SESSION).unwrap();

    // 1. Status with no record reports absence without mutating. The exit code
    //    mirrors the core `status`, whose uninstalled answer is `1`.
    let absent_output = run(
        &[
            "addon",
            "status",
            "--name",
            ADDON,
            "--directory",
            path_str(&fixture.workspace),
            "--json",
        ],
        &mut transcript,
    );
    assert_eq!(absent_output.status.code(), Some(1));
    let absent = json(&absent_output);
    assert_eq!(absent["operation"], "addon_status");
    assert_eq!(absent["name"], ADDON);
    assert_eq!(absent["record"], Value::Null);
    assert!(!absent["session_pending"].as_bool().unwrap());

    // 2. Install at ref A; the core-version default comes from the executable.
    let installed = run(
        &[
            "addon",
            "install",
            "--name",
            ADDON,
            "--manifest",
            path_str(&fixture.manifest_a),
            "--source",
            path_str(&fixture.payload_a),
            "--source-ref",
            REF_A,
            "--directory",
            path_str(&fixture.workspace),
            "--json",
        ],
        &mut transcript,
    );
    assert!(installed.status.success(), "{}", stderr(&installed));
    let installed = json(&installed);
    assert_eq!(installed["operation"], "addon_install");
    assert!(installed["applied"].as_bool().unwrap());
    assert!(!installed["adopted"].as_bool().unwrap());
    assert_eq!(installed["source_ref"], REF_A);
    assert_eq!(
        fs::read(fixture.workspace.join(ADDON_SKILL)).unwrap(),
        SKILL_A
    );
    assert_eq!(
        fs::read(fixture.workspace.join(ADDON_REFERENCE)).unwrap(),
        REFERENCE_A
    );

    // 3. Status reports the record, the core version, and per-file digests.
    let recorded = fixture.status(&mut transcript);
    assert_eq!(recorded["record"]["source_ref"], REF_A);
    assert_eq!(
        recorded["record"]["source_core_version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(recorded["record"]["files"][0]["path"], ADDON_SKILL);
    assert_eq!(
        recorded["record"]["files"][0]["sha256"],
        sha256_hex(SKILL_A)
    );
    assert_eq!(recorded["record"]["files"][1]["path"], ADDON_REFERENCE);
    assert_eq!(
        recorded["record"]["files"][1]["sha256"],
        sha256_hex(REFERENCE_A)
    );
    assert!(!recorded["session_pending"].as_bool().unwrap());

    // 4. A dry run to ref B reports the plan and mutates nothing.
    let preview_before = managed_snapshot(&fixture.workspace);
    let mut dry_run_args =
        fixture.update_args("update", &fixture.payload_b, &fixture.manifest_b, REF_B);
    dry_run_args.push("--dry-run".to_owned());
    let dry_run_args = dry_run_args.iter().map(String::as_str).collect::<Vec<_>>();
    let dry_run_output = run(&dry_run_args, &mut transcript);
    assert!(
        dry_run_output.status.success(),
        "{}",
        stderr(&dry_run_output)
    );
    let preview = json(&dry_run_output);
    assert!(preview["dry_run"].as_bool().unwrap());
    assert!(!preview["applied"].as_bool().unwrap());
    assert_eq!(preview["conflicts"].as_array().unwrap().len(), 0);
    assert_eq!(preview["changes"].as_array().unwrap().len(), 2);
    assert_eq!(
        managed_snapshot(&fixture.workspace),
        preview_before,
        "an add-on update dry run must mutate nothing"
    );

    // 5. A real update to ref B applies the clean plan.
    let update_args = fixture.update_args("update", &fixture.payload_b, &fixture.manifest_b, REF_B);
    let update_args = update_args.iter().map(String::as_str).collect::<Vec<_>>();
    let update_output = run(&update_args, &mut transcript);
    assert!(update_output.status.success(), "{}", stderr(&update_output));
    let updated = json(&update_output);
    assert!(updated["applied"].as_bool().unwrap());
    assert_eq!(
        fs::read(fixture.workspace.join(ADDON_SKILL)).unwrap(),
        SKILL_B
    );
    assert_eq!(
        fs::read(fixture.workspace.join(ADDON_REFERENCE)).unwrap(),
        REFERENCE_B
    );

    // 6. Seed a conflict: a consumer edit plus a disjoint upstream change.
    fs::write(fixture.workspace.join(ADDON_SKILL), b"local edit\n").unwrap();
    let addons_before = fs::read(fixture.workspace.join(".truss-core/addons.json")).unwrap();
    let conflict_before = managed_snapshot(&fixture.workspace);
    let conflict_args =
        fixture.update_args("update", &fixture.payload_c, &fixture.manifest_c, REF_C);
    let conflict_args = conflict_args.iter().map(String::as_str).collect::<Vec<_>>();
    let staged = run(&conflict_args, &mut transcript);
    assert_eq!(staged.status.code(), Some(2), "{}", stderr(&staged));
    let staged = json(&staged);
    assert!(staged["resolution_staged"].as_bool().unwrap());
    assert!(!staged["applied"].as_bool().unwrap());
    assert_eq!(staged["conflicts"].as_array().unwrap().len(), 1);
    assert_eq!(staged["conflicts"][0]["path"], ADDON_SKILL);
    assert_eq!(staged["conflicts"][0]["reason"], "overlapping_changes");
    // The clean subset of the conflicted plan is staged, never applied.
    assert_eq!(managed_snapshot(&fixture.workspace), conflict_before);
    assert_eq!(
        fs::read(fixture.workspace.join(ADDON_REFERENCE)).unwrap(),
        REFERENCE_B
    );
    assert_eq!(
        fs::read(fixture.workspace.join(".truss-core/addons.json")).unwrap(),
        addons_before
    );
    assert!(fixture.status(&mut transcript)["session_pending"]
        .as_bool()
        .unwrap());

    // 7. The operator edits the session's own resolved content, then continues
    //    by name alone (the command takes no payload).
    let resolved = fixture.workspace.join(format!(
        ".truss-core/addon-update/{ADDON}/resolved/{ADDON_SKILL}"
    ));
    fs::write(&resolved, RESOLVED_SKILL).unwrap();
    let continue_output = run(
        &[
            "addon",
            "continue",
            "--name",
            ADDON,
            "--directory",
            path_str(&fixture.workspace),
            "--json",
        ],
        &mut transcript,
    );
    assert!(
        continue_output.status.success(),
        "{}",
        stderr(&continue_output)
    );
    let continued = json(&continue_output);
    assert_eq!(continued["operation"], "addon_continue");
    assert!(continued["applied"].as_bool().unwrap());
    assert_eq!(continued["source_ref"], Value::Null);
    assert_eq!(
        fs::read(fixture.workspace.join(ADDON_SKILL)).unwrap(),
        RESOLVED_SKILL
    );
    assert_eq!(
        fs::read(fixture.workspace.join(ADDON_REFERENCE)).unwrap(),
        REFERENCE_C
    );

    // 8. The new record reports ref C, and its digests are the payload bytes,
    //    never the operator's resolved bytes.
    let resumed = fixture.status(&mut transcript);
    assert_eq!(resumed["record"]["source_ref"], REF_C);
    assert_eq!(
        resumed["record"]["source_core_version"],
        UPDATE_CORE_VERSION
    );
    assert_eq!(resumed["record"]["files"][0]["sha256"], sha256_hex(SKILL_C));
    assert_eq!(
        resumed["record"]["files"][1]["sha256"],
        sha256_hex(REFERENCE_C)
    );
    assert!(!resumed["session_pending"].as_bool().unwrap());
    assert_eq!(fs::read(&core_session).unwrap(), CORE_SESSION);

    // 9. Abort twice after the continue: the session is already consumed, so
    //    both calls are safe no-ops and the core session survives.
    for _ in 0..2 {
        let aborted = run(
            &[
                "addon",
                "abort",
                "--name",
                ADDON,
                "--directory",
                path_str(&fixture.workspace),
                "--json",
            ],
            &mut transcript,
        );
        assert!(aborted.status.success(), "{}", stderr(&aborted));
        assert!(!json(&aborted)["removed"].as_bool().unwrap());
    }
    assert_eq!(fs::read(&core_session).unwrap(), CORE_SESSION);

    // 10. Stage one more conflict, then abort twice: the first removes only the
    //     owned add-on session, the second is a safe no-op.
    let restaged_args =
        fixture.update_args("update", &fixture.payload_d, &fixture.manifest_d, REF_D);
    let restaged_args = restaged_args.iter().map(String::as_str).collect::<Vec<_>>();
    let restaged = run(&restaged_args, &mut transcript);
    assert_eq!(restaged.status.code(), Some(2), "{}", stderr(&restaged));
    assert!(json(&restaged)["resolution_staged"].as_bool().unwrap());
    let managed_before_abort = managed_snapshot(&fixture.workspace);
    let first = json(&run(
        &[
            "addon",
            "abort",
            "--name",
            ADDON,
            "--directory",
            path_str(&fixture.workspace),
            "--json",
        ],
        &mut transcript,
    ));
    assert!(first["removed"].as_bool().unwrap());
    let second = json(&run(
        &[
            "addon",
            "abort",
            "--name",
            ADDON,
            "--directory",
            path_str(&fixture.workspace),
            "--json",
        ],
        &mut transcript,
    ));
    assert!(!second["removed"].as_bool().unwrap());
    assert_eq!(managed_snapshot(&fixture.workspace), managed_before_abort);
    assert_eq!(fs::read(&core_session).unwrap(), CORE_SESSION);
    assert_eq!(
        fs::read(fixture.workspace.join(ADDON_SKILL)).unwrap(),
        RESOLVED_SKILL
    );

    println!("{transcript}");
    evidence("s4b3-cli-transcript.txt", &transcript);
    evidence(
        "s4b3-row1-cycle.txt",
        &format!(
            "install_source_ref={REF_A}\nstatus_after_install=({REF_A}, {}, {} {})\nstatus_after_update=({REF_B})\nconflicted_update_exit=2 conflicts=1 resolution_staged=true applied=false\ncontinue_applied=true source_ref=null\nstatus_after_continue=({REF_C}, {}, {} {})\nabort_after_continue=removed:false,false\nrestaged_abort=removed:true,false\ncore_session_untouched=true\ncore_session_sha256={}\n",
            env!("CARGO_PKG_VERSION"),
            ADDON_SKILL,
            sha256_hex(SKILL_A),
            UPDATE_CORE_VERSION,
            ADDON_SKILL,
            sha256_hex(SKILL_C),
            sha256_hex(CORE_SESSION),
        ),
    );
}

/// Acceptance row 2: a dry run is mutation free, and a conflicted plan is
/// staged instead of being partially applied.
#[test]
fn cli_addon_dry_run_and_conflicted_update_never_mutate() {
    let fixture = AddonFixture::new();
    let mut transcript = String::new();
    fixture.core_state(&mut transcript);
    let installed = run(
        &[
            "addon",
            "install",
            "--name",
            ADDON,
            "--manifest",
            path_str(&fixture.manifest_a),
            "--source",
            path_str(&fixture.payload_a),
            "--source-ref",
            REF_A,
            "--directory",
            path_str(&fixture.workspace),
            "--json",
        ],
        &mut transcript,
    );
    assert!(installed.status.success(), "{}", stderr(&installed));

    // The dry run leaves the workspace, the baseline, and addons.json identical.
    let before = common::workspace_snapshot(&fixture.workspace);
    let addons_before = fs::read(fixture.workspace.join(".truss-core/addons.json")).unwrap();
    let mut dry_args =
        fixture.update_args("update", &fixture.payload_b, &fixture.manifest_b, REF_B);
    dry_args.push("--dry-run".to_owned());
    let dry_args = dry_args.iter().map(String::as_str).collect::<Vec<_>>();
    let preview = json(&run(&dry_args, &mut transcript));
    assert!(preview["dry_run"].as_bool().unwrap());
    let after = common::workspace_snapshot(&fixture.workspace);
    assert_eq!(after, before, "a dry run must be byte-identical");
    assert_eq!(
        fs::read(fixture.workspace.join(".truss-core/addons.json")).unwrap(),
        addons_before
    );

    // A conflicted plan is staged; neither the conflicted path nor the clean
    // subset is applied.
    fs::write(fixture.workspace.join(ADDON_SKILL), b"local edit\n").unwrap();
    let conflict_before = managed_snapshot(&fixture.workspace);
    let conflict_args =
        fixture.update_args("update", &fixture.payload_c, &fixture.manifest_c, REF_C);
    let conflict_args = conflict_args.iter().map(String::as_str).collect::<Vec<_>>();
    let staged = run(&conflict_args, &mut transcript);
    assert_eq!(staged.status.code(), Some(2), "{}", stderr(&staged));
    let staged = json(&staged);
    assert!(staged["resolution_staged"].as_bool().unwrap());
    assert!(!staged["applied"].as_bool().unwrap());
    let conflict_after = managed_snapshot(&fixture.workspace);
    assert_eq!(conflict_after, conflict_before);
    assert_eq!(
        fs::read(fixture.workspace.join(ADDON_SKILL)).unwrap(),
        b"local edit\n"
    );
    assert_eq!(
        fs::read(fixture.workspace.join(ADDON_REFERENCE)).unwrap(),
        REFERENCE_A
    );
    assert_eq!(
        fs::read(fixture.workspace.join(".truss-core/addons.json")).unwrap(),
        addons_before
    );
    assert!(fixture
        .workspace
        .join(format!(
            ".truss-core/addon-update/{ADDON}/resolved/{ADDON_SKILL}"
        ))
        .is_file());

    evidence(
        "s4b3-row2-dry-run-and-conflict.txt",
        &format!(
            "dry_run_snapshot_before={}\ndry_run_snapshot_after={}\ndry_run_equal={}\ndry_run_addons_json_equal=true\nconflict_snapshot_before={}\nconflict_snapshot_after={}\nconflict_managed_surface_equal={}\nconflict_exit=2 staged=true applied=false\nlocal_skill={:?}\nreference_untouched={:?}\naddons_json_equal=true\nsession_marker={}\n",
            common::snapshot_digest(&before),
            common::snapshot_digest(&after),
            after == before,
            common::snapshot_digest(&conflict_before),
            common::snapshot_digest(&conflict_after),
            conflict_after == conflict_before,
            String::from_utf8_lossy(&fs::read(fixture.workspace.join(ADDON_SKILL)).unwrap()),
            String::from_utf8_lossy(&fs::read(fixture.workspace.join(ADDON_REFERENCE)).unwrap()),
            fixture.workspace.join(format!(".truss-core/addon-update/{ADDON}/session.json")).is_file(),
        ),
    );
}

/// Acceptance row 1, counterexample side: `continue` is payload free, and a
/// branch name is refused before any mutation.
#[test]
fn cli_addon_continue_is_payload_free_and_mutable_refs_are_rejected() {
    let fixture = AddonFixture::new();
    let mut transcript = String::new();

    let help = run(&["addon", "continue", "--help"], &mut transcript);
    assert!(help.status.success());
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("--name"), "{help}");
    assert!(!help.contains("--manifest"), "{help}");
    assert!(!help.contains("--source"), "{help}");

    // A payload argument on `continue` fails at the parser, not at the facade.
    let refused = run(
        &[
            "addon",
            "continue",
            "--name",
            ADDON,
            "--manifest",
            "files.txt",
        ],
        &mut transcript,
    );
    assert_eq!(refused.status.code(), Some(2), "{}", stderr(&refused));
    assert!(stderr(&refused).contains("unexpected argument"));

    // A branch name is refused before the workspace is even opened.
    let absent = fixture.tmp.path().join("absent-workspace");
    let branch = run(
        &[
            "addon",
            "install",
            "--name",
            ADDON,
            "--manifest",
            path_str(&fixture.manifest_a),
            "--source",
            path_str(&fixture.payload_a),
            "--source-ref",
            "main",
            "--directory",
            path_str(&absent),
            "--json",
        ],
        &mut transcript,
    );
    assert_eq!(branch.status.code(), Some(1));
    assert!(
        stderr(&branch).contains("immutable release tag or exact commit SHA"),
        "{}",
        stderr(&branch)
    );
    assert!(!absent.exists(), "a refused ref must touch no workspace");

    // A 40-hex commit SHA passes argument mapping and fails later, for a
    // different reason: the ref shape is accepted.
    let commit = run(
        &[
            "addon",
            "install",
            "--name",
            ADDON,
            "--manifest",
            path_str(&fixture.manifest_a),
            "--source",
            path_str(&fixture.payload_a),
            "--source-ref",
            "b11d79ad2310890713ab281e6d0a8a57e449bef6",
            "--directory",
            path_str(&absent),
            "--json",
        ],
        &mut transcript,
    );
    assert!(
        !stderr(&commit).contains("immutable release tag"),
        "{}",
        stderr(&commit)
    );

    evidence("s4b3-row1-parser.txt", &transcript);
}
