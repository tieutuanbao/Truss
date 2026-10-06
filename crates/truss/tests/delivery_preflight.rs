//! Exercise the shipped read-only pre-dispatch helper, not a test-local parser.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::tempdir;

const TABLE: &str = "| Role | Truss | Model | Effort |\n\
| --- | --- | --- | --- |\n\
| `project-manager` | current | current | current |\n\
| `ba` | Pi | tao-router/thinking | high |\n\
| `architect` | Codex | thinking-high | default |\n\
| `detailed-designer` | Codex | thinking | low |\n\
| `planner` | Pi | tao-router/thinking | default |\n\
| `implement` | Codex | code-writer | high |\n\
| `visual-engineering` | Pi | tao-router/visual-engineering | high |\n\
| `tester` | Codex | tester | high |\n\
| `debugger` | Codex | debugger | high |\n";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|path| path.join("distribution/payload").is_dir())
        .unwrap()
        .to_path_buf()
}

fn managed(table: &str) -> String {
    format!(
        "Human-owned prose\n<!-- delivery:begin -->\n## Delivery\n{table}<!-- delivery:end -->\n"
    )
}

fn run(agents: &Path, approved: &Path, role: &str) -> Output {
    Command::new("python3")
        .arg(
            root().join("distribution/payload/.agents/skills/delivery/scripts/check-role-tuple.py"),
        )
        .args(["--agents"])
        .arg(agents)
        .arg("--approved")
        .arg(approved)
        .args(["--role", role])
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .expect("python3 runs the installed helper")
}

#[test]
fn rereads_disk_and_rejects_changed_tuple_without_writing_inputs() {
    let dir = tempdir().unwrap();
    let agents = dir.path().join("AGENTS.md");
    let approved = dir.path().join("approved-envelope.md");
    let initial = managed(TABLE);
    fs::write(&agents, &initial).unwrap();
    fs::write(&approved, TABLE).unwrap();

    let result = run(&agents, &approved, "architect");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(receipt["tuple"]["truss"], "codex");
    assert_eq!(receipt["tuple"]["model"], "thinking-high");
    assert_eq!(receipt["tuple"]["effort"], "default");
    assert_eq!(fs::read_to_string(&agents).unwrap(), initial);
    assert_eq!(fs::read_to_string(&approved).unwrap(), TABLE);

    for (before, after) in [
        (
            "| Codex | thinking-high | default |",
            "| Pi | thinking-high | default |",
        ),
        (
            "| Codex | thinking-high | default |",
            "| Codex | thinking | default |",
        ),
        (
            "| Codex | thinking-high | default |",
            "| Codex | thinking-high | high |",
        ),
    ] {
        let changed = initial.replace(before, after);
        fs::write(&agents, &changed).unwrap();
        let result = run(&agents, &approved, "architect");
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains("DELIVERY_ROLE_TUPLE_CHANGED"), "{error}");
        assert!(
            error.contains("architect") && error.contains("approved"),
            "{error}"
        );
        assert_eq!(fs::read_to_string(&agents).unwrap(), changed);
        assert_eq!(fs::read_to_string(&approved).unwrap(), TABLE);
    }
}

#[test]
fn exact_documented_aliases_match_but_guessed_names_do_not() {
    let dir = tempdir().unwrap();
    let agents = dir.path().join("AGENTS.md");
    let approved = dir.path().join("approved-envelope.md");
    fs::write(&approved, TABLE).unwrap();
    fs::write(
        &agents,
        managed(&TABLE.replace("| Codex |", "| Codex CLI |")),
    )
    .unwrap();
    assert!(run(&agents, &approved, "architect").status.success());

    // Every documented truss must normalize through the same shipped owner,
    // including unresolved launch adapters; launch capability is a later gate.
    for (name, want) in [
        ("Claude Code", "claude"),
        ("Cursor Agent CLI", "cursor"),
        ("Antigravity CLI", "antigravity"),
        ("Pi", "pi"),
        ("OpenCode", "opencode"),
        ("Zcode", "zcode"),
        ("Grok Build", "grok"),
        ("Kiro CLI", "kiro"),
        ("GitHub Copilot CLI", "copilot"),
    ] {
        let table = TABLE.replace("| Codex |", &format!("| {name} |"));
        fs::write(&agents, managed(&table)).unwrap();
        fs::write(&approved, &table).unwrap();
        let result = run(&agents, &approved, "architect");
        assert!(
            result.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let receipt: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(receipt["tuple"]["truss"], want);
    }
    fs::write(&approved, TABLE).unwrap();

    fs::write(
        &agents,
        managed(&TABLE.replace("| Codex |", "| Codexish |")),
    )
    .unwrap();
    let result = run(&agents, &approved, "architect");
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("unrecognized Truss"));
}

#[test]
fn malformed_missing_duplicate_and_retired_configuration_fails_closed() {
    let dir = tempdir().unwrap();
    let agents = dir.path().join("AGENTS.md");
    let approved = dir.path().join("approved-envelope.md");
    fs::write(&approved, TABLE).unwrap();
    let initial = managed(TABLE);
    for (input, diagnostic) in [
        (TABLE.to_owned(), "managed block"),
        (
            initial.replace("<!-- delivery:end -->", ""),
            "managed block",
        ),
        (format!("{initial}{initial}"), "managed block"),
        (format!("<!-- delivery:end -->\n{initial}"), "managed block"),
        (
            initial.replace(
                "| Role | Truss | Model | Effort |",
                "| Role | Truss | Effort |",
            ),
            "table",
        ),
        (
            initial.replace("| `architect` | Codex | thinking-high | default |\n", ""),
            "missing role",
        ),
        (
            initial.replace(
                "<!-- delivery:end -->",
                "| `architect` | Codex | thinking-high | default |\n<!-- delivery:end -->",
            ),
            "duplicate role",
        ),
        (
            initial.replace(
                "<!-- delivery:end -->",
                "\n| `architect` | Codex | thinking-high | default |\n<!-- delivery:end -->",
            ),
            "duplicate role",
        ),
        (
            initial.replace(
                "<!-- delivery:end -->",
                "\n| `tester-debugger` | Codex | tester | high |\n<!-- delivery:end -->",
            ),
            "DELIVERY_ROLE_CONFIG_MIGRATION_REQUIRED",
        ),
        (
            initial.replace(
                "<!-- delivery:end -->",
                "\n| `consult` | Codex | thinking | high |\n<!-- delivery:end -->",
            ),
            "outside",
        ),
        (initial.replace("`architect`", "`consult`"), "unknown role"),
        (initial.replace("thinking-high", "…"), "unresolvable"),
        (
            initial.replace("`tester`", "`tester-debugger`"),
            "DELIVERY_ROLE_CONFIG_MIGRATION_REQUIRED",
        ),
    ] {
        fs::write(&agents, &input).unwrap();
        let result = run(&agents, &approved, "architect");
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(
            error.contains(diagnostic),
            "wanted {diagnostic:?}, got {error}"
        );
        assert!(
            error.contains("setup") || error.contains("envelope"),
            "{error}"
        );
        assert_eq!(fs::read_to_string(&agents).unwrap(), input);
    }
}

#[test]
fn invalid_envelope_unreadable_input_and_unknown_target_do_not_allow_dispatch() {
    let dir = tempdir().unwrap();
    let agents = dir.path().join("AGENTS.md");
    let approved = dir.path().join("approved-envelope.md");
    fs::write(&agents, managed(TABLE)).unwrap();
    fs::write(&approved, "No frozen role tuple table").unwrap();
    let result = run(&agents, &approved, "architect");
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("envelope"));
    fs::write(&approved, format!("{TABLE}\n{TABLE}")).unwrap();
    assert_eq!(run(&agents, &approved, "architect").status.code(), Some(2));
    fs::write(&approved, TABLE).unwrap();
    assert_eq!(run(&agents, &approved, "consult").status.code(), Some(2));
    let result = run(&dir.path().join("missing.md"), &approved, "architect");
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("missing.md"));
}
