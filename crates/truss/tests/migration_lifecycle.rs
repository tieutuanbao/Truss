//! REQ-001–REQ-008, REQ-017–REQ-026, REQ-032, REQ-034, REQ-036, REQ-037.
//!
//! Every case here drives the shipped `truss` binary against a real workspace,
//! so the contract is observed where an operator observes it: the process exit
//! status, the text and JSON output, and the repository bytes. The preview
//! cases compare a complete recursive snapshot, so a preview that prints a
//! correct plan while opening the lock or allocating a backup directory is
//! rejected. The retirement case invokes a *copied* binary from inside the
//! legacy tree, which is the only instrument that proves the Linux
//! running-executable apply (D-04).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::workspace_snapshot;

const BINARY: &str = env!("CARGO_BIN_EXE_truss");

fn install(root: &Path) -> Output {
    Command::new(BINARY)
        .args(["install", "--directory"])
        .arg(root)
        .arg("--json")
        .output()
        .expect("the shipped binary runs")
}

fn status(root: &Path) -> Output {
    Command::new(BINARY)
        .args(["status", "--directory"])
        .arg(root)
        .arg("--json")
        .output()
        .expect("the shipped binary runs")
}

fn migrate(root: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["migrate", "--directory"];
    let path = root.to_str().unwrap();
    args.push(path);
    args.extend_from_slice(extra);
    Command::new(BINARY)
        .args(&args)
        .output()
        .expect("the shipped binary runs")
}

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "expected one JSON object on stdout: {error}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// How many times an `ETXTBSY` exec is retried before the failure is
/// reported. The holder is a concurrent child of this test process that
/// inherited the destination's write descriptor at `fork` and drops it at its
/// own `exec`; that window is sub-millisecond, so a short bounded wait is
/// sufficient and the failure is still reported if no holder ever leaves.
const BUSY_EXEC_ATTEMPTS: usize = 50;

/// Runs `command`, retrying while the kernel reports `ETXTBSY`
/// (`ErrorKind::ExecutableFileBusy`).
///
/// A destination written moments earlier (by the `fs::copy` below, or by the
/// migration's atomic publish) can be briefly "busy" to `exec`: while another
/// test thread forks during that write, the child inherits the write
/// descriptor and holds it until its own `exec` closes it. This is a property
/// of running these tests in parallel on Linux, not of the binary under test,
/// so the retry waits out the transient holder without weakening any assertion.
fn output_retrying_busy(command: &mut Command) -> std::io::Result<Output> {
    let mut attempt = 0;
    loop {
        match command.output() {
            Err(error)
                if error.kind() == std::io::ErrorKind::ExecutableFileBusy
                    && attempt + 1 < BUSY_EXEC_ATTEMPTS =>
            {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            result => return result,
        }
    }
}

/// A real installation moved to the pre-0008 layout: the state root becomes
/// `.truss-core`, every managed destination is rewritten to `.truss-core/…`,
/// the `base/` mirror follows, and the recorded core version is the released
/// `0.1.16` a genuine legacy consumer carries (ASM-09).
fn legacy_tree(state: &Path, core_version: Option<&str>) {
    let manifest_path = state.join("manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    if let Some(version) = core_version {
        manifest["core_version"] = serde_json::json!(version);
    }
    for file in manifest["files"].as_array_mut().unwrap() {
        let path = file["path"]
            .as_str()
            .unwrap()
            .replace(".truss/core/", ".truss-core/");
        file["path"] = serde_json::json!(path);
    }
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let base = state.join("base/.truss/core");
    if base.is_dir() {
        fs::rename(&base, state.join("base/.truss-core")).unwrap();
    }
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap();
        }
    }
}

fn legacy_repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    assert!(install(root.path()).status.success());
    let state = root.path().join(".truss/core");
    fs::rename(&state, root.path().join(".truss-core")).unwrap();
    legacy_tree(&root.path().join(".truss-core"), Some("0.1.16"));
    if fs::read_dir(root.path().join(".truss"))
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false)
    {
        fs::remove_dir(root.path().join(".truss")).unwrap();
    }
    assert!(root.path().join(".truss-core/manifest.json").is_file());
    assert!(!root.path().join(".truss").exists());
    root
}

/// A repository holding both roots: the installed new root and a legacy copy
/// of it. `install` resolves the legacy root when it is the only tree, so the
/// legacy side must be built from a copy rather than by re-running install.
fn dual_repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    assert!(install(root.path()).status.success());
    copy_tree(
        &root.path().join(".truss/core"),
        &root.path().join(".truss-core"),
    );
    legacy_tree(&root.path().join(".truss-core"), None);
    root
}

/// REQ-001: `--force` and every other unknown flag are refused by argument
/// mapping before any repository read.
#[test]
fn unknown_flags_are_refused_before_any_repository_read() {
    let fixture = legacy_repository();
    let root = fixture.path();
    let before = workspace_snapshot(root);
    for extra in ["--force", "--overwrite", "--yes"] {
        let output = migrate(root, &[extra]);
        assert!(
            !output.status.success(),
            "{extra} must be refused, so no hidden force path exists"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unexpected argument")
                || String::from_utf8_lossy(&output.stderr).contains("error"),
            "clap names the refusal: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(before, workspace_snapshot(root));
}

/// REQ-002 / REQ-036: preview is observational in both modes; its envelope has
/// every stable field and a template path with no concrete timestamp.
#[test]
fn preview_performs_no_mutation_and_emits_the_stable_envelope() {
    let fixture = legacy_repository();
    let root = fixture.path();
    let before = workspace_snapshot(root);

    let text = migrate(root, &[]);
    assert!(text.status.success(), "a ready preview exits 0");
    let body = String::from_utf8_lossy(&text.stdout);
    assert!(body.contains("ready"), "text names the state: {body}");
    assert!(
        body.contains(".truss-migration-backup/<UTC-timestamp>/"),
        "text prints the deterministic template: {body}"
    );

    let output = migrate(root, &["--json"]);
    assert!(output.status.success());
    let envelope = json(&output);
    for field in [
        "operation",
        "state",
        "reason",
        "applied",
        "repository",
        "backup_path",
        "backup_path_template",
        "transaction_id",
        "run_key",
        "legacy_roots",
        "operations",
        "conflicts",
        "evidence",
    ] {
        assert!(
            envelope.get(field).is_some(),
            "the envelope always carries {field}"
        );
    }
    assert_eq!(envelope["operation"], "migrate");
    assert_eq!(envelope["state"], "ready");
    assert_eq!(envelope["applied"], false);
    assert!(envelope["backup_path"].is_null());
    assert!(envelope["backup_path_template"]
        .as_str()
        .unwrap()
        .contains("<UTC-timestamp>"));
    assert!(!envelope["operations"].as_array().unwrap().is_empty());

    assert_eq!(
        before,
        workspace_snapshot(root),
        "a preview mutates absolutely nothing"
    );
    assert!(!root.join(".truss-migration-backup").exists());
    assert!(!root.join(".truss/core").exists());
}

/// REQ-004 / REQ-036: exit 0 covers exactly `ready`, `already_migrated`, and
/// `migrated`; `blocked`, `rolled_back`, and `recovery_required` are non-zero.
#[test]
fn exit_classes_follow_the_state_vocabulary() {
    // ready
    let fixture = legacy_repository();
    assert_eq!(migrate(fixture.path(), &[]).status.code(), Some(0));
    // migrated
    let fixture = legacy_repository();
    let applied = migrate(fixture.path(), &["--apply", "--json"]);
    assert_eq!(applied.status.code(), Some(0));
    assert_eq!(json(&applied)["state"], "migrated");
    // already_migrated
    let again = migrate(fixture.path(), &["--apply", "--json"]);
    assert_eq!(again.status.code(), Some(0));
    assert_eq!(json(&again)["state"], "already_migrated");
    // blocked / not_installed
    let empty = tempfile::tempdir().unwrap();
    let blocked = migrate(empty.path(), &["--json"]);
    assert_eq!(blocked.status.code(), Some(1));
    let envelope = json(&blocked);
    assert_eq!(envelope["state"], "blocked");
    assert_eq!(envelope["reason"], "not_installed");
    // blocked text mode writes the refusal to stderr, never a success report
    let text = migrate(empty.path(), &[]);
    assert_eq!(text.status.code(), Some(1));
    assert!(text.stdout.is_empty());
    assert!(String::from_utf8_lossy(&text.stderr).contains("blocked"));

    // rolled_back: a destination parent that cannot be created forces a real
    // failure after the first mutation.
    let fixture = legacy_repository();
    let root = fixture.path();
    fs::create_dir_all(root.join(".truss")).unwrap();
    fs::write(root.join(".truss/core"), b"not a directory\n").unwrap();
    let rolled = migrate(root, &["--apply", "--json"]);
    assert_eq!(rolled.status.code(), Some(1));
    assert_eq!(
        json(&rolled)["state"],
        "rolled_back",
        "a failure after the first mutation reports honestly"
    );
    assert!(
        root.join(".truss-core/manifest.json").is_file(),
        "a rollback restores the legacy installation"
    );
}

/// REQ-005 / REQ-006 / REQ-007: all three legacy roots are recognized, a
/// new-only repository is `already_migrated`, and a fresh one is refused.
#[test]
fn all_three_legacy_roots_are_recognized() {
    let fixture = legacy_repository();
    let root = fixture.path();
    common::write_file(root, ".delivery/run-a/plan.md", "plan\n");
    common::write_file(root, ".delivery-dispatch/run-a/prompt.md", "prompt\n");
    let preview = json(&migrate(root, &["--json"]));
    let roots: Vec<&str> = preview["legacy_roots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(
        roots,
        vec![".truss-core", ".delivery", ".delivery-dispatch"]
    );
    assert_eq!(preview["run_key"], "run-a");

    let applied = migrate(root, &["--apply", "--json"]);
    assert_eq!(applied.status.code(), Some(0));
    assert!(root.join(".truss/core/manifest.json").is_file());
    assert!(root
        .join(".truss/delivery/runs/run-a/evidence/plan.md")
        .is_file());
    assert!(root
        .join(".truss/delivery/runs/run-a/dispatch/prompt.md")
        .is_file());
    assert!(!root.join(".delivery").exists());
    assert!(!root.join(".delivery-dispatch").exists());
    assert!(!root.join(".truss-core").exists());
}

/// REQ-008: migration is the one command that succeeds on a dual root, and it
/// does not route through the ordinary resolver that every read refuses.
#[test]
fn dual_root_migrates_while_the_read_commands_still_refuse() {
    let fixture = dual_repository();
    let root = fixture.path();
    assert!(root.join(".truss/core/manifest.json").is_file());
    assert!(root.join(".truss-core/manifest.json").is_file());

    for command in [vec!["status"], vec!["doctor"]] {
        let mut args = command.clone();
        args.extend(["--directory", root.to_str().unwrap()]);
        let output = Command::new(BINARY).args(&args).output().unwrap();
        assert!(
            !output.status.success(),
            "{command:?} must still refuse a dual root"
        );
    }

    let preview = json(&migrate(root, &["--json"]));
    assert_eq!(preview["state"], "ready", "migrate proceeds on a dual root");
    assert!(
        !preview["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|line| line.as_str().unwrap().contains("decide which tree")),
        "migrate must not produce the ordinary resolver refusal"
    );
    let applied = migrate(root, &["--apply", "--json"]);
    assert_eq!(applied.status.code(), Some(0));
    assert_eq!(json(&applied)["state"], "migrated");
    assert!(!root.join(".truss-core").exists());
    assert!(root.join(".truss/core/manifest.json").is_file());
}

/// REQ-017 / REQ-018: delivery evidence and dispatch land under one run key.
/// REQ-019: an ambiguous shape blocks and names its candidates.
#[test]
fn delivery_evidence_and_dispatch_share_one_run_key() {
    let fixture = legacy_repository();
    let root = fixture.path();
    common::write_file(root, ".delivery/run-a/plan.md", "plan\n");
    common::write_file(root, ".delivery-dispatch/run-a/prompt.md", "prompt\n");
    assert_eq!(migrate(root, &["--apply", "--json"]).status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(root.join(".truss/delivery/runs/run-a/evidence/plan.md")).unwrap(),
        "plan\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".truss/delivery/runs/run-a/dispatch/prompt.md")).unwrap(),
        "prompt\n"
    );

    let fixture = legacy_repository();
    let root = fixture.path();
    common::write_file(root, ".delivery/aaa/plan.md", "a\n");
    common::write_file(root, ".delivery/bbb/plan.md", "b\n");
    let before = workspace_snapshot(root);
    let blocked = migrate(root, &["--json"]);
    assert_eq!(blocked.status.code(), Some(1));
    let envelope = json(&blocked);
    assert_eq!(envelope["state"], "blocked");
    assert_eq!(envelope["reason"], "ambiguous_run_key");
    assert!(envelope["conflicts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|line| line.as_str().unwrap().contains(".delivery/aaa")));
    assert_eq!(before, workspace_snapshot(root));
}

/// REQ-020: a byte-different destination blocks before mutation; a byte-equal
/// one is idempotent.
#[test]
fn collisions_are_idempotent_or_refused() {
    let fixture = legacy_repository();
    let root = fixture.path();
    common::write_file(root, ".truss/core/docs/WORKFLOW.md", "operator bytes\n");
    let before = workspace_snapshot(root);
    let blocked = migrate(root, &["--apply", "--json"]);
    assert_eq!(blocked.status.code(), Some(1));
    assert_eq!(json(&blocked)["reason"], "collision");
    assert_eq!(
        fs::read_to_string(root.join(".truss/core/docs/WORKFLOW.md")).unwrap(),
        "operator bytes\n",
        "a different-byte destination is never overwritten"
    );
    assert_eq!(before, workspace_snapshot(root));
}

/// REQ-021 / REQ-023: only the managed block changes and only migrated
/// authority documents are rewritten.
#[test]
fn entrypoint_block_and_prose_are_scoped() {
    let fixture = legacy_repository();
    let root = fixture.path();
    let original = fs::read_to_string(root.join("AGENTS.md")).unwrap();
    let before = format!("consumer prelude\n\n{original}\nconsumer trailer\n");
    fs::write(root.join("AGENTS.md"), &before).unwrap();
    // A migrated authority document gets the structural token rewrite.
    common::write_file(
        root,
        ".truss-core/docs/decisions/0002-y.md",
        "read `.truss-core/docs/WORKFLOW.md` for the flow\n",
    );
    // An unrelated file that merely mentions the old root is untouched.
    common::write_file(root, "notes.md", "the `.truss-core/docs` tree is old\n");

    assert_eq!(migrate(root, &["--apply", "--json"]).status.code(), Some(0));

    let after = fs::read_to_string(root.join("AGENTS.md")).unwrap();
    assert!(
        after.starts_with("consumer prelude\n\n"),
        "prefix preserved"
    );
    assert!(after.ends_with("consumer trailer\n"), "suffix preserved");
    assert_eq!(
        after, before,
        "the block was already canonical, so no byte changed"
    );
    assert_eq!(
        fs::read_to_string(root.join(".truss/authority/decisions/0002-y.md")).unwrap(),
        "read `.truss/core/docs/WORKFLOW.md` for the flow\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("notes.md")).unwrap(),
        "the `.truss-core/docs` tree is old\n",
        "no repository-wide search and replace"
    );
}

/// REQ-022: a corrupt managed block is a refusal that changes nothing.
#[test]
fn corrupt_entrypoint_markers_block_before_mutation() {
    let fixture = legacy_repository();
    let root = fixture.path();
    fs::write(root.join("AGENTS.md"), "no block here\n").unwrap();
    let before = workspace_snapshot(root);
    let blocked = migrate(root, &["--apply", "--json"]);
    assert_eq!(blocked.status.code(), Some(1));
    let envelope = json(&blocked);
    assert_eq!(envelope["state"], "blocked");
    assert_eq!(envelope["reason"], "invalid_markers");
    assert_eq!(
        fs::read_to_string(root.join("AGENTS.md")).unwrap(),
        "no block here\n"
    );
    assert_eq!(before, workspace_snapshot(root));
}

/// REQ-024 / D-01: the five relative rules land, legacy spellings go, and
/// unrelated rules stay byte-identical.
#[test]
fn ignore_repair_preserves_unrelated_rules_and_adds_the_relative_set() {
    let fixture = legacy_repository();
    let root = fixture.path();
    let unrelated = "# mine\n/notes.md\n.truss/core/bin/truss-v0-migrate\nold/\n";
    fs::write(
        root.join(".gitignore"),
        format!("{unrelated}/.truss-core/\n/.truss/\n"),
    )
    .unwrap();
    assert_eq!(migrate(root, &["--apply", "--json"]).status.code(), Some(0));
    let repaired = fs::read_to_string(root.join(".gitignore")).unwrap();
    for rule in [
        ".truss/authority/",
        ".truss/delivery/",
        ".truss/core/bin/truss",
        ".truss/core/bin/truss.exe",
        ".truss-migration-backup/",
    ] {
        assert!(
            repaired.lines().any(|line| line.trim() == rule),
            "missing {rule} in {repaired}"
        );
    }
    assert!(repaired.contains("# mine\n"));
    assert!(repaired.contains("/notes.md\n"));
    assert!(
        repaired.contains(".truss/core/bin/truss-v0-migrate\n"),
        "an unrelated rule that merely mentions .truss is preserved: {repaired}"
    );
    assert!(!repaired.contains("/.truss-core/"));
    assert!(
        !repaired
            .lines()
            .any(|line| line.trim_start().starts_with("/.truss")),
        "no root-anchored spelling: {repaired}"
    );
}

/// REQ-025 / REQ-034: apply prints the exact created path, a second apply adds
/// no backup, and `already_migrated` names the existing one.
#[test]
fn apply_prints_the_exact_path_and_is_idempotent() {
    let fixture = legacy_repository();
    let root = fixture.path();
    let applied = json(&migrate(root, &["--apply", "--json"]));
    let path = PathBuf::from(applied["backup_path"].as_str().unwrap());
    assert!(path.is_dir(), "the printed path is the created directory");
    assert!(!applied["backup_path"].as_str().unwrap().contains("<UTC"));
    assert!(path.join("migration.json").is_file());
    assert!(path.join("backup").is_dir());

    let before: Vec<String> = backup_dirs(root);
    assert_eq!(before.len(), 1);
    let again = json(&migrate(root, &["--apply", "--json"]));
    assert_eq!(again["state"], "already_migrated");
    assert_eq!(
        again["backup_path"].as_str().unwrap(),
        applied["backup_path"]
    );
    assert_eq!(backup_dirs(root), before, "no second backup directory");
}

fn backup_dirs(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    if let Ok(entries) = fs::read_dir(root.join(".truss-migration-backup")) {
        for entry in entries.flatten() {
            if entry.file_type().unwrap().is_dir() {
                found.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    found.sort();
    found
}

/// REQ-032: an incomplete but valid journal makes preview `recovery_required`,
/// and `--apply` rolls it back before a fresh preflight migrates.
#[test]
fn an_incomplete_journal_is_recovered_then_replanned() {
    let fixture = legacy_repository();
    let root = fixture.path();
    let applied = json(&migrate(root, &["--apply", "--json"]));
    let path = PathBuf::from(applied["backup_path"].as_str().unwrap());
    let journal_path = path.join("migration.json");
    let mut journal: serde_json::Value =
        serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
    journal["state"] = serde_json::json!("in_progress");
    journal["phase"] = serde_json::json!("namespaces_published");
    fs::write(&journal_path, serde_json::to_vec_pretty(&journal).unwrap()).unwrap();

    let preview = json(&migrate(root, &["--json"]));
    assert_eq!(preview["state"], "recovery_required");
    assert_eq!(preview["state"].as_str().unwrap(), "recovery_required");

    let recovered = migrate(root, &["--apply", "--json"]);
    assert_eq!(recovered.status.code(), Some(0));
    let envelope = json(&recovered);
    assert_eq!(
        envelope["state"], "migrated",
        "recovery rolled back then migrated fresh: {envelope}"
    );
    assert!(root.join(".truss/core/manifest.json").is_file());
    assert!(!root.join(".truss-core").exists());
    let entries: Vec<String> = backup_dirs(root);
    assert!(entries.len() >= 2, "the recovered transaction is retained");
}

/// REQ-011 / REQ-012 / REQ-015: managed state moves byte-for-byte, the record
/// rewrite reports clean, and managed docs never become authority.
#[test]
fn migrated_state_is_clean_and_managed_documents_stay_managed() {
    let fixture = legacy_repository();
    let root = fixture.path();
    let legacy_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".truss-core/manifest.json")).unwrap()).unwrap();
    let legacy_hashes: Vec<String> = legacy_manifest["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["upstream_sha256"].as_str().unwrap().to_owned())
        .collect();

    assert_eq!(migrate(root, &["--apply", "--json"]).status.code(), Some(0));

    let status = json(&status(root));
    let modified = status["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|file| file["modified"] == true)
        .count();
    let missing = status["files"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|file| file["missing"] == true)
        .count();
    assert_eq!(
        (modified, missing),
        (0, 0),
        "status reports clean: {status}"
    );

    let rewritten: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".truss/core/manifest.json")).unwrap()).unwrap();
    assert_eq!(rewritten["core_version"], "0.1.16");
    let moved: Vec<String> = rewritten["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["path"].as_str().unwrap().to_owned())
        .collect();
    let doc_destinations = moved
        .iter()
        .filter(|path| path.starts_with(".truss/core/docs/"))
        .count();
    assert_eq!(doc_destinations, 13, "the 13 core destinations moved");
    let rewritten_hashes: Vec<String> = rewritten["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["upstream_sha256"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(rewritten_hashes, legacy_hashes, "hashes are preserved");
    assert!(root.join(".truss/core/docs/decisions/README.md").is_file());
    assert!(!root.join(".truss/authority/decisions/README.md").exists());

    // The post-migration read surfaces must not be left refusing.
    let doctor = Command::new(BINARY)
        .args(["doctor", "--directory"])
        .arg(root)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        doctor.status.success(),
        "doctor is healthy after migration: {}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    let addon = Command::new(BINARY)
        .args(["addon", "status", "--name", "demo", "--directory"])
        .arg(root)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&addon.stderr).contains("truss migrate --directory"),
        "addon status keeps working after migration: {}",
        String::from_utf8_lossy(&addon.stderr)
    );
}

/// D-04 / REQ-037: a copied binary performs the Linux running-binary apply
/// from inside the legacy tree it retires.
#[test]
fn a_copied_running_binary_applies_and_retires_its_own_tree() {
    let fixture = legacy_repository();
    let root = fixture.path();
    let legacy_binary = root.join(".truss-core/bin/truss");
    fs::create_dir_all(legacy_binary.parent().unwrap()).unwrap();
    fs::copy(BINARY, &legacy_binary).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let output = output_retrying_busy(
        Command::new(&legacy_binary)
            .arg("migrate")
            .arg("--directory")
            .arg(root)
            .arg("--apply")
            .arg("--json")
            .current_dir(elsewhere.path()),
    )
    .expect("the copied binary runs");
    assert!(
        output.status.success(),
        "the running binary retires its own tree: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(json(&output)["state"], "migrated");
    assert!(
        !root.join(".truss-core").exists(),
        "the legacy tree that held the running executable is retired"
    );
    assert!(
        root.join(".truss/core/bin/truss").is_file(),
        "the executable moved to the new root"
    );
}

#[cfg(unix)]
fn mode_of(path: &Path) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).unwrap().permissions().mode() & 0o7777
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// Remediation: a migrated executable stays runnable and the retained backup
/// keeps its mode. Before the fix `copy_bytes_atomic` created every
/// destination with the process default mode, so `.truss/core/bin/truss` was
/// published as `-rw-rw-r--` and the installation was functionally broken.
#[cfg(unix)]
#[test]
fn migrated_executables_keep_their_permission_bits() {
    let fixture = legacy_repository();
    let root = fixture.path();
    let binary = root.join(".truss-core/bin/truss");
    fs::create_dir_all(binary.parent().unwrap()).unwrap();
    fs::copy(BINARY, &binary).unwrap();
    let source_mode = mode_of(&binary);
    assert_eq!(
        source_mode & 0o111,
        0o111,
        "the fixture binary is executable before migration"
    );
    // The two canonical executable helper scripts the remediation names, made
    // executable inside the legacy baseline so this fixture matches a real
    // legacy consumer that carried them with an execute bit.
    let scripts = [
        ".truss-core/base/.agents/skills/onboard-repository/scripts/emit_evidence_bundle.py",
        ".truss-core/base/.agents/skills/onboard-repository/scripts/render_patch.py",
    ];
    for script in scripts {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.join(script), fs::Permissions::from_mode(0o755)).unwrap();
    }

    let applied = json(&migrate(root, &["--apply", "--json"]));
    assert_eq!(applied["state"], "migrated");
    let published = root.join(".truss/core/bin/truss");
    assert!(published.is_file());
    assert_eq!(
        mode_of(&published),
        source_mode,
        "the published binary keeps its permission bits"
    );
    assert_eq!(
        mode_of(&published) & 0o111,
        0o111,
        "the binary stays runnable"
    );
    assert!(
        output_retrying_busy(Command::new(&published).arg("--version")).is_ok(),
        "the migrated binary is still executable"
    );

    let backup = PathBuf::from(applied["backup_path"].as_str().unwrap())
        .join("backup/.truss-core/bin/truss");
    assert_eq!(
        mode_of(&backup),
        source_mode,
        "the retained backup keeps the mode for a later rollback"
    );

    for script in scripts {
        let published = root.join(script.replace(".truss-core/", ".truss/core/"));
        assert!(published.is_file(), "{} moved", published.display());
        assert_eq!(
            mode_of(&published) & 0o111,
            0o111,
            "{} stays executable",
            published.display()
        );
    }
}

/// Owner decision A (ADR 0008 amendment "Bundled executable refresh"):
/// `migrate --apply` writes the *running* executable to the bundled entrypoint
/// so a migrated pre-0008 installation is readable by its own entrypoint.
///
/// This is the discriminating instrument. Before the fix the migration moved
/// the legacy stub byte-for-byte and reported `migrated`, leaving a dead
/// entrypoint: the behaviour existed, ran, and returned a pass. The stub below
/// is deliberately not the running binary, so a `migrated` run that publishes
/// the legacy bytes is rejected.
#[cfg(unix)]
#[test]
fn apply_publishes_the_running_executable_instead_of_the_legacy_stub() {
    let fixture = legacy_repository();
    let root = fixture.path();
    let legacy_binary = root.join(".truss-core/bin/truss");
    fs::create_dir_all(legacy_binary.parent().unwrap()).unwrap();
    let stub = b"#!/bin/sh\necho not_installed\n";
    fs::write(&legacy_binary, stub).unwrap();
    // A legacy entrypoint that is not even executable: the published running
    // executable must still be published executable (755 at minimum).
    set_mode(&legacy_binary, 0o600);

    let applied = json(&migrate(root, &["--apply", "--json"]));
    assert_eq!(applied["state"], "migrated");

    let published = root.join(".truss/core/bin/truss");
    assert!(published.is_file(), "the bundled entrypoint exists");
    assert_eq!(
        fs::read(&published).unwrap(),
        fs::read(BINARY).unwrap(),
        "the published entrypoint is the running executable, not the legacy stub"
    );
    assert_eq!(
        mode_of(&published),
        0o755,
        "the published entrypoint is executable (755 at minimum)"
    );
    assert!(
        output_retrying_busy(Command::new(&published).arg("--version")).is_ok(),
        "the published entrypoint runs"
    );

    // The legacy stub is still backed up byte-for-byte, so rollback and the
    // retained recovery material keep the original installation.
    let backup = PathBuf::from(applied["backup_path"].as_str().unwrap())
        .join("backup/.truss-core/bin/truss");
    assert_eq!(fs::read(&backup).unwrap(), stub);
}

/// Owner decision A: even when the legacy tree shipped no executable, the
/// migration creates the bundled entrypoint from the running executable, and a
/// rollback removes the file it created.
#[cfg(unix)]
#[test]
fn apply_creates_the_bundled_entrypoint_when_the_legacy_tree_had_none() {
    let fixture = legacy_repository();
    let root = fixture.path();
    assert!(!root.join(".truss-core/bin/truss").exists());
    let applied = json(&migrate(root, &["--apply", "--json"]));
    assert_eq!(applied["state"], "migrated");
    let published = root.join(".truss/core/bin/truss");
    assert_eq!(fs::read(&published).unwrap(), fs::read(BINARY).unwrap());
    assert_eq!(mode_of(&published) & 0o111, 0o111);

    // Force a real failure after publication, then assert the rollback removed
    // the entrypoint the migration created (the legacy tree never had one).
    let fixture = legacy_repository();
    let root = fixture.path();
    fs::create_dir_all(root.join(".truss/core")).unwrap();
    fs::write(
        root.join(".truss/core/docs"),
        b"a file where a directory was needed\n",
    )
    .unwrap();
    let rolled = json(&migrate(root, &["--apply", "--json"]));
    assert_eq!(rolled["state"], "rolled_back", "{rolled}");
    assert!(
        !root.join(".truss/core/bin/truss").exists(),
        "rollback removes the created entrypoint"
    );
    assert!(
        root.join(".truss-core").is_dir(),
        "rollback restores the legacy tree"
    );
    assert!(root.join(".truss-core/manifest.json").is_file());
}
