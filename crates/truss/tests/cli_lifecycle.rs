use std::fs;
use std::process::Command;

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
