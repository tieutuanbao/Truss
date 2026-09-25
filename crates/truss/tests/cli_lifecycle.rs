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
    assert!(!root.path().join(".truss/core").exists());

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
    assert!(root.path().join(".truss/core/manifest.json").is_file());
    assert!(root.path().join(".truss/core/base/AGENTS.md").is_file());
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
        .filter(|line| !line.contains(".truss/core/addon-update"))
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
    // leave `.truss/core/update/` byte-identical.
    let core_session = fixture.workspace.join(".truss/core/update/session.json");
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
    let addons_before = fs::read(fixture.workspace.join(".truss/core/addons.json")).unwrap();
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
        fs::read(fixture.workspace.join(".truss/core/addons.json")).unwrap(),
        addons_before
    );
    assert!(fixture.status(&mut transcript)["session_pending"]
        .as_bool()
        .unwrap());

    // 7. The operator edits the session's own resolved content, then continues
    //    by name alone (the command takes no payload).
    let resolved = fixture.workspace.join(format!(
        ".truss/core/addon-update/{ADDON}/resolved/{ADDON_SKILL}"
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
    let addons_before = fs::read(fixture.workspace.join(".truss/core/addons.json")).unwrap();
    let mut dry_args =
        fixture.update_args("update", &fixture.payload_b, &fixture.manifest_b, REF_B);
    dry_args.push("--dry-run".to_owned());
    let dry_args = dry_args.iter().map(String::as_str).collect::<Vec<_>>();
    let preview = json(&run(&dry_args, &mut transcript));
    assert!(preview["dry_run"].as_bool().unwrap());
    let after = common::workspace_snapshot(&fixture.workspace);
    assert_eq!(after, before, "a dry run must be byte-identical");
    assert_eq!(
        fs::read(fixture.workspace.join(".truss/core/addons.json")).unwrap(),
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
        fs::read(fixture.workspace.join(".truss/core/addons.json")).unwrap(),
        addons_before
    );
    assert!(fixture
        .workspace
        .join(format!(
            ".truss/core/addon-update/{ADDON}/resolved/{ADDON_SKILL}"
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
            fixture.workspace.join(format!(".truss/core/addon-update/{ADDON}/session.json")).is_file(),
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

// ---------------------------------------------------------------------------
// S4b4 — the real delivery add-on at its real location.
//
// The payload is the repository's own 18 delivery paths, staged from
// `scripts/delivery-install-files.txt`, and every workspace is built by the
// shipped `truss install`, so the scan-root rule is exercised against the real
// installed shape that made `.agents` an over-broad root.
// ---------------------------------------------------------------------------

const DELIVERY: &str = "delivery";
const DELIVERY_SUBJECT: &str = ".agents/skills/delivery/SKILL.md";
const DELIVERY_CLEAN: &str = ".agents/skills/delivery/references/trusses/pi.md";
const DELIVERY_STRAY: &str = ".agents/skills/delivery/stray.md";
const DELIVERY_REF_A: &str = "truss-v0.1.13";
const DELIVERY_REF_B: &str = "truss-v0.1.14";
const DELIVERY_REF_C: &str = "truss-v0.1.15";
const DELIVERY_REF_D: &str = "truss-v0.1.16";
const DELIVERY_CORE_VERSION: &str = "0.1.14";
const DELIVERY_RESOLVED: &[u8] = b"resolved delivery skill\n";

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn delivery_manifest() -> PathBuf {
    repository_root().join("scripts/delivery-install-files.txt")
}

fn delivery_payload_root() -> PathBuf {
    repository_root().join("distribution").join("payload")
}

fn delivery_paths() -> Vec<String> {
    fs::read_to_string(delivery_manifest())
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// Stage the real delivery payload bytes at their real relative paths.
fn stage_delivery_payload(destination: &Path) {
    for path in delivery_paths() {
        let bytes = fs::read(delivery_payload_root().join(&path)).unwrap();
        let target = destination.join(&path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
}

fn append_to_payload(payload: &Path, relative: &str, extra: &[u8]) {
    let target = payload.join(relative);
    let mut bytes = fs::read(&target).unwrap();
    bytes.extend_from_slice(extra);
    fs::write(&target, bytes).unwrap();
}

fn payload_digest(payload: &Path, relative: &str) -> String {
    sha256_hex(&fs::read(payload.join(relative)).unwrap())
}

fn evidence_s4b4(name: &str, body: &str) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("target/s4b4-evidence");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(name), body).unwrap();
}

struct RealDeliveryFixture {
    tmp: tempfile::TempDir,
    workspace: PathBuf,
    payload_a: PathBuf,
    payload_b: PathBuf,
    payload_c: PathBuf,
    payload_d: PathBuf,
    manifest: PathBuf,
}

impl RealDeliveryFixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path().join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let payload_a = tmp.path().join("payload-a");
        let payload_b = tmp.path().join("payload-b");
        let payload_c = tmp.path().join("payload-c");
        let payload_d = tmp.path().join("payload-d");
        stage_delivery_payload(&payload_a);
        // Ref B: exactly one changed file.
        stage_delivery_payload(&payload_b);
        append_to_payload(&payload_b, DELIVERY_SUBJECT, b"\n<!-- s4b4 ref b -->\n");
        // Ref C: the conflicted subject plus one clean path.
        stage_delivery_payload(&payload_c);
        append_to_payload(&payload_c, DELIVERY_SUBJECT, b"\n<!-- s4b4 ref c -->\n");
        append_to_payload(&payload_c, DELIVERY_CLEAN, b"\n<!-- s4b4 clean c -->\n");
        // Ref D: restage the same shape after the continue.
        stage_delivery_payload(&payload_d);
        append_to_payload(&payload_d, DELIVERY_SUBJECT, b"\n<!-- s4b4 ref d -->\n");
        append_to_payload(&payload_d, DELIVERY_CLEAN, b"\n<!-- s4b4 clean d -->\n");
        Self {
            tmp,
            workspace,
            payload_a,
            payload_b,
            payload_c,
            payload_d,
            manifest: delivery_manifest(),
        }
    }

    /// A real core install through the shipped command.
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
                DELIVERY,
                "--directory",
                path_str(&self.workspace),
                "--json",
            ],
            transcript,
        ))
    }

    fn install_args(&self, source: &Path, source_ref: &str) -> Vec<String> {
        vec![
            "addon".to_owned(),
            "install".to_owned(),
            "--name".to_owned(),
            DELIVERY.to_owned(),
            "--manifest".to_owned(),
            path_str(&self.manifest).to_owned(),
            "--source".to_owned(),
            path_str(source).to_owned(),
            "--source-ref".to_owned(),
            source_ref.to_owned(),
            "--directory".to_owned(),
            path_str(&self.workspace).to_owned(),
            "--json".to_owned(),
        ]
    }

    fn update_args(&self, source: &Path, source_ref: &str) -> Vec<String> {
        let mut args = self.install_args(source, source_ref);
        args[1] = "update".to_owned();
        let json_index = args.iter().position(|arg| arg == "--json").unwrap();
        args.splice(
            json_index..json_index,
            [
                "--source-core-version".to_owned(),
                DELIVERY_CORE_VERSION.to_owned(),
            ],
        );
        args
    }
}

/// Acceptance row 1: the whole cycle works for the real delivery add-on at its
/// real location `.agents/skills/delivery` and `.agents/skills/delivery-setup`.
#[test]
fn cli_addon_real_delivery_cycle_installs_updates_resolves_and_aborts() {
    let fixture = RealDeliveryFixture::new();
    let mut transcript = String::new();
    let paths = delivery_paths();
    assert_eq!(paths.len(), 18, "the real delivery manifest is 18 paths");
    fixture.core_state(&mut transcript);

    // A core conflict session plus `AGENTS.md` must survive every add-on step.
    let core_session = fixture.workspace.join(".truss/core/update/session.json");
    fs::create_dir_all(core_session.parent().unwrap()).unwrap();
    fs::write(&core_session, CORE_SESSION).unwrap();
    let agents_path = fixture.workspace.join("AGENTS.md");
    let agents_before = fs::read(&agents_path).unwrap();

    // 1. Absent status.
    let absent = run(
        &[
            "addon",
            "status",
            "--name",
            DELIVERY,
            "--directory",
            path_str(&fixture.workspace),
            "--json",
        ],
        &mut transcript,
    );
    assert_eq!(absent.status.code(), Some(1));
    assert_eq!(json(&absent)["record"], Value::Null);

    // 2. Install at ref A, at the real location.
    let args = fixture.install_args(&fixture.payload_a, DELIVERY_REF_A);
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let installed = run(&args, &mut transcript);
    assert!(installed.status.success(), "{}", stderr(&installed));
    let installed = json(&installed);
    assert_eq!(installed["operation"], "addon_install");
    assert!(installed["applied"].as_bool().unwrap());
    assert!(!installed["adopted"].as_bool().unwrap());
    assert_eq!(installed["source_ref"], DELIVERY_REF_A);
    assert!(fixture
        .workspace
        .join(".agents/skills/delivery-setup/SKILL.md")
        .is_file());

    // 3. Status reports the ref and all 18 payload digests.
    let recorded = fixture.status(&mut transcript);
    assert_eq!(recorded["record"]["source_ref"], DELIVERY_REF_A);
    assert_eq!(
        recorded["record"]["source_core_version"],
        env!("CARGO_PKG_VERSION")
    );
    let files = recorded["record"]["files"].as_array().unwrap();
    assert_eq!(files.len(), 18);
    for (index, path) in paths.iter().enumerate() {
        assert_eq!(files[index]["path"], path.as_str());
        assert_eq!(
            files[index]["sha256"],
            payload_digest(&fixture.payload_a, path)
        );
    }
    assert_eq!(fs::read(&agents_path).unwrap(), agents_before);
    assert_eq!(fs::read(&core_session).unwrap(), CORE_SESSION);

    // 4. A dry run to ref B reports exactly one Update and mutates nothing.
    let before = common::workspace_snapshot(&fixture.workspace);
    let addons_before = fs::read(fixture.workspace.join(".truss/core/addons.json")).unwrap();
    let mut dry = fixture.update_args(&fixture.payload_b, DELIVERY_REF_B);
    dry.push("--dry-run".to_owned());
    let dry = dry.iter().map(String::as_str).collect::<Vec<_>>();
    let preview = json(&run(&dry, &mut transcript));
    assert!(preview["dry_run"].as_bool().unwrap());
    assert_eq!(preview["conflicts"].as_array().unwrap().len(), 0);
    let updates = preview["changes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|change| change["kind"] == "update")
        .count();
    assert_eq!(updates, 1, "exactly one Update: {}", preview["changes"]);
    assert_eq!(
        common::workspace_snapshot(&fixture.workspace),
        before,
        "an add-on update dry run must mutate nothing"
    );
    assert_eq!(
        fs::read(fixture.workspace.join(".truss/core/addons.json")).unwrap(),
        addons_before
    );

    // 5. The real update applies ref B.
    let args = fixture.update_args(&fixture.payload_b, DELIVERY_REF_B);
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let updated = run(&args, &mut transcript);
    assert!(updated.status.success(), "{}", stderr(&updated));
    assert!(json(&updated)["applied"].as_bool().unwrap());
    assert_eq!(
        fs::read(fixture.workspace.join(DELIVERY_SUBJECT)).unwrap(),
        fs::read(fixture.payload_b.join(DELIVERY_SUBJECT)).unwrap()
    );

    // 6. A seeded conflict: consumer edit plus an upstream change on the same
    //    managed path, with a disjoint clean change staged and never applied.
    fs::write(fixture.workspace.join(DELIVERY_SUBJECT), b"consumer edit\n").unwrap();
    let addons_before = fs::read(fixture.workspace.join(".truss/core/addons.json")).unwrap();
    let conflict_before = managed_snapshot(&fixture.workspace);
    let args = fixture.update_args(&fixture.payload_c, DELIVERY_REF_C);
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let staged = run(&args, &mut transcript);
    assert_eq!(staged.status.code(), Some(2), "{}", stderr(&staged));
    let staged = json(&staged);
    assert!(staged["resolution_staged"].as_bool().unwrap());
    assert!(!staged["applied"].as_bool().unwrap());
    assert_eq!(staged["conflicts"].as_array().unwrap().len(), 1);
    assert_eq!(staged["conflicts"][0]["path"], DELIVERY_SUBJECT);
    assert_eq!(managed_snapshot(&fixture.workspace), conflict_before);
    assert_eq!(
        fs::read(fixture.workspace.join(DELIVERY_CLEAN)).unwrap(),
        fs::read(fixture.payload_a.join(DELIVERY_CLEAN)).unwrap(),
        "the clean subset of a conflicted plan must be staged, never applied"
    );
    assert_eq!(
        fs::read(fixture.workspace.join(".truss/core/addons.json")).unwrap(),
        addons_before
    );
    assert!(fixture.status(&mut transcript)["session_pending"]
        .as_bool()
        .unwrap());
    assert_eq!(fs::read(&core_session).unwrap(), CORE_SESSION);

    // 7. The operator edits the session's own `resolved/` content, then the
    //    payload-free `continue` completes.
    let resolved = fixture.workspace.join(format!(
        ".truss/core/addon-update/{DELIVERY}/resolved/{DELIVERY_SUBJECT}"
    ));
    fs::write(&resolved, DELIVERY_RESOLVED).unwrap();
    let continued = json(&run(
        &[
            "addon",
            "continue",
            "--name",
            DELIVERY,
            "--directory",
            path_str(&fixture.workspace),
            "--json",
        ],
        &mut transcript,
    ));
    assert_eq!(continued["operation"], "addon_continue");
    assert!(continued["applied"].as_bool().unwrap());
    assert_eq!(continued["source_ref"], Value::Null);
    assert_eq!(
        fs::read(fixture.workspace.join(DELIVERY_SUBJECT)).unwrap(),
        DELIVERY_RESOLVED
    );
    assert_eq!(
        fs::read(fixture.workspace.join(DELIVERY_CLEAN)).unwrap(),
        fs::read(fixture.payload_c.join(DELIVERY_CLEAN)).unwrap()
    );

    // 8. The record reports ref C and the payload digests, never the operator's
    //    resolved bytes.
    let resumed = fixture.status(&mut transcript);
    assert_eq!(resumed["record"]["source_ref"], DELIVERY_REF_C);
    assert_eq!(
        resumed["record"]["source_core_version"],
        DELIVERY_CORE_VERSION
    );
    let files = resumed["record"]["files"].as_array().unwrap();
    for (index, path) in paths.iter().enumerate() {
        assert_eq!(
            files[index]["sha256"],
            payload_digest(&fixture.payload_c, path)
        );
    }
    assert!(!resumed["session_pending"].as_bool().unwrap());
    assert_eq!(fs::read(&core_session).unwrap(), CORE_SESSION);
    assert_eq!(fs::read(&agents_path).unwrap(), agents_before);

    // 9. Idempotent abort after the continue: both calls are safe no-ops.
    for _ in 0..2 {
        let aborted = json(&run(
            &[
                "addon",
                "abort",
                "--name",
                DELIVERY,
                "--directory",
                path_str(&fixture.workspace),
                "--json",
            ],
            &mut transcript,
        ));
        assert!(!aborted["removed"].as_bool().unwrap());
    }
    assert_eq!(fs::read(&core_session).unwrap(), CORE_SESSION);

    // 10. Restage with ref D, then abort twice: a real removal, then a no-op.
    let args = fixture.update_args(&fixture.payload_d, DELIVERY_REF_D);
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let restaged = run(&args, &mut transcript);
    assert_eq!(restaged.status.code(), Some(2), "{}", stderr(&restaged));
    assert!(json(&restaged)["resolution_staged"].as_bool().unwrap());
    let managed_before_abort = managed_snapshot(&fixture.workspace);
    let first = json(&run(
        &[
            "addon",
            "abort",
            "--name",
            DELIVERY,
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
            DELIVERY,
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
        fs::read(fixture.workspace.join(DELIVERY_SUBJECT)).unwrap(),
        DELIVERY_RESOLVED
    );
    assert_eq!(fs::read(&agents_path).unwrap(), agents_before);

    println!("{transcript}");
    evidence_s4b4("s4b4-cli-transcript.txt", &transcript);
    evidence_s4b4(
        "s4b4-row1-cycle.txt",
        &format!(
            "install_source_ref={DELIVERY_REF_A}\nstatus_paths=18 status_ref={DELIVERY_REF_A}\ndry_run_updates=1 dry_run_mutated=false\nupdate_applied=true ref={DELIVERY_REF_B}\nconflict_exit=2 conflicts=1 staged=true applied=false\ncontinue_applied=true source_ref=null\nstatus_after_continue_ref={DELIVERY_REF_C} digests_are_payload=true\nabort_after_continue=removed:false,false\nrestaged_abort=removed:true,false\nagents_md_untouched=true\ncore_session_untouched=true\ncore_session_sha256={}\n",
            sha256_hex(CORE_SESSION),
        ),
    );
}

/// Acceptance row 2: the hazard check still fires inside the add-on's own
/// subtree. An undeclared `.agents/skills/delivery/stray.md` refuses the
/// install and is named, and the three surfaces stay unchanged.
#[test]
fn cli_addon_real_delivery_stray_path_is_refused() {
    let fixture = RealDeliveryFixture::new();
    let mut transcript = String::new();
    fixture.core_state(&mut transcript);
    fs::create_dir_all(fixture.workspace.join(".agents/skills/delivery")).unwrap();
    fs::write(fixture.workspace.join(DELIVERY_STRAY), b"stray\n").unwrap();

    let before = common::workspace_snapshot(&fixture.workspace);
    let args = fixture.install_args(&fixture.payload_a, DELIVERY_REF_A);
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let refused = run(&args, &mut transcript);
    let after = common::workspace_snapshot(&fixture.workspace);

    assert_eq!(refused.status.code(), Some(1), "{}", stderr(&refused));
    let error = stderr(&refused);
    assert!(
        error.contains("stray.md"),
        "the refusal must name the stray file, got: {error}"
    );
    assert_eq!(
        before, after,
        "the workspace surface must be unchanged by the refusal"
    );
    assert!(!fixture.workspace.join(".truss/core/addons.json").exists());
    assert!(!fixture.workspace.join(".truss/core/base-addons").exists());

    evidence_s4b4(
        "s4b4-row2-stray-refusal.txt",
        &format!(
            "stray={DELIVERY_STRAY}\nexit={}\nnamed=true\nworkspace_unchanged={}\naddons_json_absent={}\nbaseline_absent={}\nrefusal={}\n",
            refused.status.code().unwrap_or(-1),
            before == after,
            !fixture.workspace.join(".truss/core/addons.json").exists(),
            !fixture.workspace.join(".truss/core/base-addons").exists(),
            error.trim(),
        ),
    );
}

/// Acceptance row 3: a state whose `.truss/core` holds only `.gitignore` and
/// `lock` is refused as invalid before any observation, so the foreign set is
/// never silently empty.
#[test]
fn cli_addon_incomplete_core_state_is_refused_before_observation() {
    let fixture = RealDeliveryFixture::new();
    let mut transcript = String::new();
    let state_root = fixture.workspace.join(".truss/core");
    fs::create_dir_all(&state_root).unwrap();
    fs::write(state_root.join(".gitignore"), common::CORE_STATE_IGNORE).unwrap();
    fs::write(state_root.join("lock"), b"").unwrap();
    assert!(!state_root.join("manifest.json").exists());
    let _ = fixture.tmp.path();

    let before = common::workspace_snapshot(&fixture.workspace);
    let args = fixture.install_args(&fixture.payload_a, DELIVERY_REF_A);
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    let refused = run(&args, &mut transcript);
    let after = common::workspace_snapshot(&fixture.workspace);
    let error = stderr(&refused);

    assert_eq!(refused.status.code(), Some(1), "{}", error);
    assert!(
        error.contains("manifest.json"),
        "the refusal must name the missing manifest, got: {error}"
    );
    assert_eq!(
        before, after,
        "an invalid core state must be refused without mutation"
    );
    assert!(!fixture.workspace.join(".truss/core/addons.json").exists());
    assert!(!fixture.workspace.join(".truss/core/base-addons").exists());

    evidence_s4b4(
        "s4b4-row3-incomplete-core-state.txt",
        &format!(
            "core_state_files=.gitignore,lock\nexit={}\nrefused_before_observation=true\nworkspace_unchanged={}\nrefusal={}\n",
            refused.status.code().unwrap_or(-1),
            before == after,
            error.trim(),
        ),
    );
}

// S4b4 remediation — recorded ownership from the workspace's own state.
//
// The repository manifest and the shipped binary are untouched: the ownership
// set comes from `.truss/core/manifest.json` and `.truss/core/addons.json`, so
// the real command line needs no second manifest flag.
// ---------------------------------------------------------------------------

const REMEDIATION_ADDON: &str = "demo";
const REMEDIATION_SIBLING: &str = "demo-sibling";
const REMEDIATION_SUBJECT: &str = ".agents/skills/demo/SKILL.md";
const REMEDIATION_SHARED: &str = ".agents/skills/shared/notes.md";
const REMEDIATION_REF: &str = "truss-v0.1.13";
/// The sibling's bytes, staged identically into the first collision payload, so
/// a binary without the recorded-ownership guard adopts the colliding path
/// instead of failing on a byte comparison.
const REMEDIATION_SHARED_BYTES: &[u8] = b"shared notes\n";
/// Different bytes for the collision update, so a binary without the guard
/// visibly overwrites the sibling's managed file.
const REMEDIATION_SHARED_WRITE: &[u8] = b"demo wants this path\n";

/// Stage a payload directory holding exactly the given `(path, bytes)` pairs.
fn stage_remediation_payload(root: &Path, files: &[(&str, &[u8])]) {
    for (relative, bytes) in files {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
}

/// Write a membership manifest whose non-comment lines are `paths`.
fn remediation_manifest(root: &Path, name: &str, paths: &[&str]) -> PathBuf {
    let manifest = root.join(name);
    let mut text = String::from("# s4b4 remediation fixture manifest\n");
    for path in paths {
        text.push_str(path);
        text.push('\n');
    }
    fs::write(&manifest, text).unwrap();
    manifest
}

/// Build a real `addon install`/`addon update` argument vector for one
/// remediation payload.
fn remediation_args(
    command: &str,
    name: &str,
    manifest: &Path,
    source: &Path,
    workspace: &Path,
) -> Vec<String> {
    vec![
        "addon".to_owned(),
        command.to_owned(),
        "--name".to_owned(),
        name.to_owned(),
        "--manifest".to_owned(),
        path_str(manifest).to_owned(),
        "--source".to_owned(),
        path_str(source).to_owned(),
        "--source-ref".to_owned(),
        REMEDIATION_REF.to_owned(),
        "--source-core-version".to_owned(),
        DELIVERY_CORE_VERSION.to_owned(),
        "--directory".to_owned(),
        path_str(workspace).to_owned(),
        "--json".to_owned(),
    ]
}

/// Acceptance row A: with two add-ons recorded in `.truss/core/addons.json`
/// sharing a parent, the real `addon install` and `addon update` refuse a
/// descriptor that declares the other add-on's exact path, before any
/// mutation, and name the owner.
///
/// The ownership set is the workspace's own recorded state, so no extra CLI
/// flag is passed. Every refusal is checked against a byte-identical full
/// workspace, baseline tree, and `addons.json`.
#[test]
fn cli_addon_recorded_ownership_collision_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let workspace = tmp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let mut transcript = String::new();

    // Real core state through the shipped command, then a first add-on that
    // records `.agents/skills/shared/notes.md`.
    let core = run(
        &["install", "--directory", path_str(&workspace), "--json"],
        &mut transcript,
    );
    assert!(core.status.success(), "{}", stderr(&core));
    let sibling_payload = tmp.path().join("sibling-payload");
    stage_remediation_payload(
        &sibling_payload,
        &[(REMEDIATION_SHARED, REMEDIATION_SHARED_BYTES)],
    );
    let sibling_manifest =
        remediation_manifest(tmp.path(), "sibling-files.txt", &[REMEDIATION_SHARED]);
    let sibling_args = remediation_args(
        "install",
        REMEDIATION_SIBLING,
        &sibling_manifest,
        &sibling_payload,
        &workspace,
    );
    let sibling_args = sibling_args.iter().map(String::as_str).collect::<Vec<_>>();
    let sibling_install = run(&sibling_args, &mut transcript);
    assert!(
        sibling_install.status.success(),
        "the sibling install must succeed: {}",
        stderr(&sibling_install)
    );
    assert!(json(&sibling_install)["applied"].as_bool().unwrap());

    let addons_path = workspace.join(".truss/core/addons.json");
    let baselines_root = workspace.join(".truss/core/base-addons");
    let sibling_baseline = common::workspace_snapshot(&baselines_root);

    // 1. `install` with a descriptor declaring the sibling's exact path and
    //    byte-identical payload content: refused before anything is written.
    let collision_payload = tmp.path().join("collision-payload");
    stage_remediation_payload(
        &collision_payload,
        &[(REMEDIATION_SHARED, REMEDIATION_SHARED_BYTES)],
    );
    let collision_manifest =
        remediation_manifest(tmp.path(), "collision-files.txt", &[REMEDIATION_SHARED]);
    let before_install = common::workspace_snapshot(&workspace);
    let addons_before_install = fs::read(&addons_path).unwrap();
    let install_args = remediation_args(
        "install",
        REMEDIATION_ADDON,
        &collision_manifest,
        &collision_payload,
        &workspace,
    );
    let install_args = install_args.iter().map(String::as_str).collect::<Vec<_>>();
    let refused_install = run(&install_args, &mut transcript);
    let install_error = stderr(&refused_install);
    assert_eq!(
        refused_install.status.code(),
        Some(1),
        "the colliding install must be refused: {install_error}"
    );
    assert!(
        install_error.contains(REMEDIATION_SIBLING) && install_error.contains(REMEDIATION_SHARED),
        "the refusal must name the owner and the path, got: {install_error}"
    );
    assert_eq!(
        common::workspace_snapshot(&workspace),
        before_install,
        "the refused install must leave the workspace byte-identical"
    );
    assert_eq!(
        fs::read(&addons_path).unwrap(),
        addons_before_install,
        "the refused install must leave addons.json byte-identical"
    );
    assert_eq!(
        common::workspace_snapshot(&baselines_root),
        sibling_baseline,
        "the refused install must leave the baseline tree byte-identical"
    );
    let install_unchanged = common::workspace_snapshot(&workspace) == before_install;
    let install_addons_unchanged = fs::read(&addons_path).unwrap() == addons_before_install;
    let install_baselines_unchanged =
        common::workspace_snapshot(&baselines_root) == sibling_baseline;

    // 2. A real install of the add-on under test at its own path, so the
    //    update case starts from a genuine record.
    let subject_payload = tmp.path().join("subject-payload");
    stage_remediation_payload(&subject_payload, &[(REMEDIATION_SUBJECT, b"demo skill\n")]);
    let subject_manifest =
        remediation_manifest(tmp.path(), "subject-files.txt", &[REMEDIATION_SUBJECT]);
    let subject_args = remediation_args(
        "install",
        REMEDIATION_ADDON,
        &subject_manifest,
        &subject_payload,
        &workspace,
    );
    let subject_args = subject_args.iter().map(String::as_str).collect::<Vec<_>>();
    let subject_install = run(&subject_args, &mut transcript);
    assert!(
        subject_install.status.success(),
        "the add-on's own install must succeed: {}",
        stderr(&subject_install)
    );

    // 3. `update` whose descriptor declares the sibling's exact path, with
    //    different bytes: refused before planning, so the sibling's managed
    //    file is never overwritten and no provenance moves.
    let write_payload = tmp.path().join("write-payload");
    stage_remediation_payload(
        &write_payload,
        &[(REMEDIATION_SHARED, REMEDIATION_SHARED_WRITE)],
    );
    let write_manifest = remediation_manifest(tmp.path(), "write-files.txt", &[REMEDIATION_SHARED]);
    let before_update = common::workspace_snapshot(&workspace);
    let baselines_before_update = common::workspace_snapshot(&baselines_root);
    let addons_before_update = fs::read(&addons_path).unwrap();
    let update_args = remediation_args(
        "update",
        REMEDIATION_ADDON,
        &write_manifest,
        &write_payload,
        &workspace,
    );
    let update_args = update_args.iter().map(String::as_str).collect::<Vec<_>>();
    let refused_update = run(&update_args, &mut transcript);
    let update_error = stderr(&refused_update);
    assert_eq!(
        refused_update.status.code(),
        Some(1),
        "the colliding update must be refused: {update_error}"
    );
    assert!(
        update_error.contains(REMEDIATION_SIBLING) && update_error.contains(REMEDIATION_SHARED),
        "the refusal must name the owner and the path, got: {update_error}"
    );
    assert_eq!(
        common::workspace_snapshot(&workspace),
        before_update,
        "the refused update must leave the workspace byte-identical"
    );
    assert_eq!(
        common::workspace_snapshot(&baselines_root),
        baselines_before_update,
        "the refused update must leave the baseline tree byte-identical"
    );
    assert_eq!(
        fs::read(&addons_path).unwrap(),
        addons_before_update,
        "the refused update must leave addons.json byte-identical"
    );
    let update_unchanged = common::workspace_snapshot(&workspace) == before_update;
    let update_addons_unchanged = fs::read(&addons_path).unwrap() == addons_before_update;
    let update_baselines_unchanged =
        common::workspace_snapshot(&baselines_root) == baselines_before_update;
    assert_eq!(
        fs::read(workspace.join(REMEDIATION_SHARED)).unwrap(),
        REMEDIATION_SHARED_BYTES,
        "the sibling's managed file must keep its recorded bytes"
    );
    // No plan was built, so no add-on session was staged either.
    assert!(!workspace
        .join(format!(".truss/core/addon-update/{REMEDIATION_ADDON}"))
        .exists());

    // The add-on under test still records only its own path; the sibling still
    // records only its own.
    let status = json(&run(
        &[
            "addon",
            "status",
            "--name",
            REMEDIATION_ADDON,
            "--directory",
            path_str(&workspace),
            "--json",
        ],
        &mut transcript,
    ));
    assert_eq!(
        status["record"]["files"][0]["path"].as_str().unwrap(),
        REMEDIATION_SUBJECT
    );
    let sibling_status = json(&run(
        &[
            "addon",
            "status",
            "--name",
            REMEDIATION_SIBLING,
            "--directory",
            path_str(&workspace),
            "--json",
        ],
        &mut transcript,
    ));
    assert_eq!(
        sibling_status["record"]["files"][0]["path"]
            .as_str()
            .unwrap(),
        REMEDIATION_SHARED
    );

    println!("{transcript}");
    evidence_s4b4("s4b4-remediation-cli-transcript.txt", &transcript);
    evidence_s4b4(
        "s4b4-remediation-row-a.txt",
        &format!(
            "sibling_install_applied=true sibling_path={REMEDIATION_SHARED}\ninstall_collision_exit={}\ninstall_workspace_unchanged={}\ninstall_addons_json_unchanged={}\ninstall_baselines_unchanged={}\nupdate_collision_exit={}\nupdate_workspace_unchanged={}\nupdate_addons_json_unchanged={}\nupdate_baselines_unchanged={}\nsibling_bytes_unchanged=true\naddon_session_staged=false\ninstall_refusal={}\nupdate_refusal={}\nworkspace_snapshot_sha256={}\naddons_json_sha256={}\nbaselines_sha256={}\n",
            refused_install.status.code().unwrap_or(-1),
            install_unchanged,
            install_addons_unchanged,
            install_baselines_unchanged,
            refused_update.status.code().unwrap_or(-1),
            update_unchanged,
            update_addons_unchanged,
            update_baselines_unchanged,
            install_error.trim(),
            update_error.trim(),
            sha256_hex(common::workspace_snapshot(&workspace).as_bytes()),
            sha256_hex(&fs::read(&addons_path).unwrap()),
            sha256_hex(common::workspace_snapshot(&baselines_root).as_bytes()),
        ),
    );
}
