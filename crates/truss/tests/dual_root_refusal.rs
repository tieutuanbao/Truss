//! REQ-009: the read commands refuse a repository that holds both roots.
//!
//! Every case drives the shipped `truss` binary against a real workspace, so
//! the refusal is observed where an operator observes it: the process exit
//! status and the message on stderr. A repository holding both `.truss/core`
//! and `.truss-core` has two locks, two baselines, and a stale one of each, so
//! `status`, `doctor`, and `addon status` must refuse instead of reporting
//! confidently from whichever tree the ordinary resolver happens to prefer.
//! A new-only or legacy-only repository keeps its current behaviour.
//!
//! The refusal is read-only: the dual-root snapshot is compared before and
//! after, so a check that "refuses" by first mutating a tree is rejected too.

mod common;

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use common::workspace_snapshot;

const BINARY: &str = env!("CARGO_BIN_EXE_truss");

/// Install a real new-root installation through the shipped binary.
fn install_new_root(root: &Path) {
    let output = Command::new(BINARY)
        .args(["install", "--directory"])
        .arg(root)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "install must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(root.join(".truss/core/manifest.json").is_file());
}

/// Recursively copy a directory tree, so the second root is a real installed
/// tree rather than a marker the fixture invented.
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

/// A repository holding a new-root install and a copy of it at the legacy
/// root: the exact shape REQ-009 refuses.
fn dual_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    install_new_root(root.path());
    copy_tree(
        &root.path().join(".truss/core"),
        &root.path().join(".truss-core"),
    );
    root
}

/// A repository whose only install was moved to the legacy root.
fn legacy_only() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    install_new_root(root.path());
    fs::rename(
        root.path().join(".truss/core"),
        root.path().join(".truss-core"),
    )
    .unwrap();
    root
}

fn new_only() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    install_new_root(root.path());
    root
}

fn run_status(root: &Path) -> Output {
    Command::new(BINARY)
        .args(["status", "--directory"])
        .arg(root)
        .output()
        .unwrap()
}

fn run_doctor(root: &Path) -> Output {
    Command::new(BINARY)
        .args(["doctor", "--directory"])
        .arg(root)
        .output()
        .unwrap()
}

fn run_addon_status(root: &Path) -> Output {
    Command::new(BINARY)
        .args(["addon", "status", "--name", "demo", "--directory"])
        .arg(root)
        .output()
        .unwrap()
}

/// One read command on a dual root: non-zero exit, no report on stdout, and a
/// message that names the migration preview form `truss migrate --directory`.
fn assert_refusal(command: &str, output: &Output) {
    assert!(
        !output.status.success(),
        "{command} must refuse a dual-root repository and exit non-zero"
    );
    assert!(
        output.stdout.is_empty(),
        "{command} must not report from one tree on a dual root: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("truss migrate --directory"),
        "{command} must name the migration preview form: {stderr}"
    );
}

fn assert_success(command: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{command} must keep working: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `addon status` for an unrecorded add-on already reports the record state and
/// exits 1 on stdout without touching stderr. "Keeps working" means that
/// report, not the dual-root refusal.
fn assert_addon_status_not_refused(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "addon status must not refuse a single-root repository: {stderr}"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("not recorded"),
        "addon status keeps its normal report: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn all_three_read_commands_refuse_a_dual_root_repository() {
    let root = dual_root();
    assert_refusal("status", &run_status(root.path()));
    assert_refusal("doctor", &run_doctor(root.path()));
    assert_refusal("addon status", &run_addon_status(root.path()));
}

#[test]
fn dual_root_refusal_is_read_only() {
    let root = dual_root();
    let before = workspace_snapshot(root.path());
    let _ = run_status(root.path());
    let _ = run_doctor(root.path());
    let _ = run_addon_status(root.path());
    let after = workspace_snapshot(root.path());
    assert_eq!(before, after, "a refusal must not touch the repository");
}

#[test]
fn new_only_repository_keeps_working() {
    let root = new_only();
    let status = run_status(root.path());
    assert_success("status", &status);
    assert!(
        String::from_utf8_lossy(&status.stdout).contains("current"),
        "status reports the current installation: {}",
        String::from_utf8_lossy(&status.stdout)
    );
    assert_success("doctor", &run_doctor(root.path()));
    assert_addon_status_not_refused(&run_addon_status(root.path()));
}

#[test]
fn legacy_only_repository_keeps_working() {
    let root = legacy_only();
    assert_success("status", &run_status(root.path()));
    assert_success("doctor", &run_doctor(root.path()));
    assert_addon_status_not_refused(&run_addon_status(root.path()));
}
