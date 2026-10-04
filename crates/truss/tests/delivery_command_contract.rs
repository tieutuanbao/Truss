//! Delivery command contract: dispatch commands are copy-exact, never guessed.
//!
//! Two real incidents motivate this suite. A BA dispatch for the tuple
//! `Pi / tao-router/thinking / high` first guessed `--agent opencode`, then
//! guessed `worker-start --agent pi --model ... --effort ...`. Both guesses
//! failed at launch because the reader had to invent argv from prose. This
//! suite holds the delivery payload to a closed command contract: every truss
//! reference states exactly one launch classification with executable
//! recipes, and every run command lives in one copy-exact cookbook.

use std::fs;
use std::path::{Path, PathBuf};

const DELIVERY_DIR: &str = "distribution/payload/.agents/skills/delivery";
const TRUSS_DIR: &str = "distribution/payload/.agents/skills/delivery/references/trusses";

/// The repository root, found from the crate directory so the suite is
/// independent of the harness working directory.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|ancestor| ancestor.join("distribution/payload").is_dir())
        .expect("repository root containing distribution/payload")
        .to_path_buf()
}

/// Every truss launch reference, sorted for deterministic output.
fn truss_files() -> Vec<(String, String)> {
    let truss_dir = repo_root().join(TRUSS_DIR);
    let mut rows: Vec<(String, String)> = fs::read_dir(&truss_dir)
        .expect("trusses directory exists")
        .map(|entry| {
            let path = entry.expect("truss dir entry").path();
            let name = path
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .to_string();
            (
                name,
                fs::read_to_string(&path).expect("truss reference is readable UTF-8"),
            )
        })
        .filter(|(name, _)| name.ends_with(".md") && name != "trusses.md")
        .collect();
    rows.sort();
    rows
}

fn classification_of(content: &str, name: &str) -> String {
    let line = content
        .lines()
        .find(|line| line.starts_with("Launch classification:"))
        .unwrap_or_else(|| {
            panic!(
                "{name}: missing 'Launch classification:' line; a reader cannot \
                 pick the dispatch path without guessing"
            )
        });
    let value = line
        .strip_prefix("Launch classification:")
        .expect("prefix present")
        .trim()
        .to_string();
    for allowed in ["native", "composed", "unresolved"] {
        if value.starts_with(allowed) {
            return allowed.to_string();
        }
    }
    panic!("{name}: classification '{value}' is not native|composed|unresolved");
}

fn assert_contains(content: &str, needle: &str, name: &str) {
    assert!(
        content.contains(needle),
        "{name}: missing required contract text: {needle:?}"
    );
}

fn worker_start_lines(content: &str) -> Vec<&str> {
    content
        .lines()
        .filter(|line| {
            line.contains("worker-start")
                && !line.contains('\\')
                && (line.contains("--agent") || line.contains("--terminal"))
        })
        .map(|line| line.trim())
        .collect()
}

#[test]
fn every_truss_reference_states_one_launch_classification() {
    let files = truss_files();
    assert!(
        files.len() >= 10,
        "expected the full truss reference set, found {}: {:?}",
        files.len(),
        files.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>()
    );
    for (name, content) in &files {
        assert_contains(content, "Orca agent ID:", name);
        assert_contains(content, "Forbidden", name);
        classification_of(content, name);
    }
}

#[test]
fn native_trusses_carry_an_exact_worker_start_command() {
    for (name, content) in truss_files() {
        if classification_of(&content, &name) != "native" {
            continue;
        }
        let has_agent_start = content
            .lines()
            .any(|line| line.contains("worker-start") && line.contains("--agent"));
        assert!(
            has_agent_start,
            "{name}: classified native but no worker-start --agent command line"
        );
    }
}

#[test]
fn composed_trusses_carry_the_full_terminal_chain() {
    for (name, content) in truss_files() {
        if classification_of(&content, &name) != "composed" {
            continue;
        }
        assert_contains(&content, "terminal create", &name);
        assert_contains(&content, "--for tui-idle", &name);
        let start_lines = worker_start_lines(&content);
        assert!(
            start_lines.iter().any(|line| line.contains("--terminal")),
            "{name}: classified composed but no worker-start --terminal command line"
        );
        for line in start_lines {
            if line.contains("--terminal") {
                for banned in ["--agent ", "--model ", "--effort "] {
                    assert!(
                        !line.contains(banned),
                        "{name}: composed worker-start line repeats {banned:?}; \
                         the terminal argv already carries those pins: {line}"
                    );
                }
            }
        }
    }
}

#[test]
fn pi_and_opencode_never_receive_worker_start_model_pins() {
    for (name, content) in truss_files() {
        if name != "pi.md" && name != "opencode.md" {
            continue;
        }
        let offender = content
            .lines()
            .find(|line| line.contains("worker-start") && line.contains("--model"));
        assert!(
            offender.is_none(),
            "{name}: regression — a worker-start --model recipe exists for an agent \
             Orca refuses to pin at launch: {offender:?}"
        );
        assert_eq!(
            classification_of(&content, &name),
            "composed",
            "{name}: these agents are composed-path; the incidents that motivated \
             this suite were both composed tuples dispatched as native"
        );
    }
}

#[test]
fn unresolved_trusses_stop_instead_of_guessing() {
    for (name, content) in truss_files() {
        if classification_of(&content, &name) != "unresolved" {
            continue;
        }
        assert_contains(&content, "UNSUPPORTED_LAUNCH", &name);
    }
}

#[test]
fn command_cookbook_exists_and_covers_the_lifecycle() {
    let recipes = read("references/command-recipes.md");
    for needle in [
        "$DELIVERY_ORCA_CLI",
        "run_",
        "task_",
        "ctx_",
        "msg_",
        "delivery_",
        "term_",
        "run-create",
        "run-use",
        "terminal create",
        "--for tui-idle",
        "worker_done,escalation,question",
        "--ack",
        "request-show",
        "--retry-request",
        "--retry-of",
        "worker-release",
        "worker-list",
        "--terminal-state reclaimable",
        "orchestrationRequestId",
    ] {
        assert_contains(&recipes, needle, "references/command-recipes.md");
    }
    // The ack consumes the delivery id; a msg_ id there redelivers the batch.
    let ack_lines: Vec<&str> = recipes
        .lines()
        .filter(|line| line.contains("--ack"))
        .collect();
    assert!(
        !ack_lines.is_empty(),
        "command-recipes.md: no check --ack recipe"
    );
    assert!(
        ack_lines.iter().all(|line| !line.contains("msg_")),
        "command-recipes.md: an --ack recipe names a msg_ id; ack consumes the \
         delivery id: {ack_lines:?}"
    );
    // The prompt travels as an in-worktree file, never as interpolated shell.
    assert!(
        !recipes.contains("\"$(cat"),
        "command-recipes.md: a recipe interpolates a file into a shell argument, \
         the exact transport this contract forbids"
    );
    // Business-analyst skill resolves from the consumer root, not `orca skills get`.
    assert_contains(
        &recipes,
        "$DELIVERY_ROOT/.agents/skills/business-analyst/SKILL.md",
        "references/command-recipes.md",
    );
}

#[test]
fn policy_and_index_point_readers_at_the_cookbook() {
    let skill = read("SKILL.md");
    assert_contains(&skill, "references/command-recipes.md", "SKILL.md");
    let index = read("references/trusses.md");
    assert_contains(
        &index,
        "references/command-recipes.md",
        "references/trusses.md",
    );
    // Normalization table: managed-block truss names map to Orca agent IDs.
    for truss in ["Claude Code", "Codex", "Pi", "OpenCode", "Zcode"] {
        assert_contains(&index, truss, "references/trusses.md");
    }
    assert_contains(&index, "agent ID", "references/trusses.md");
}

fn read(rel: &str) -> String {
    let path = repo_root().join(DELIVERY_DIR).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("missing delivery contract file {}: {error}", path.display())
    })
}
