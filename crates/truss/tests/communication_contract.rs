//! Contract for the single communication standard shipped with Truss.

use std::fs;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate directory has a parent")
        .parent()
        .expect("the workspace directory has a parent")
        .to_path_buf()
}

fn repository_file(path: &str) -> String {
    let full = repository_root().join(path);
    fs::read_to_string(&full)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", full.display()))
}

#[test]
fn workflow_defines_one_plain_language_communication_standard() {
    let workflow = repository_file("distribution/payload/.truss/core/docs/WORKFLOW.md");

    for required in [
        "Apply one standard to every user-facing reply",
        "the language of the person's own words",
        "required facts, then safety warnings",
        "### Plain language",
        "Keep sentences under about 20 words",
        "### Terms and abbreviations",
        "`AI`, `URL`, `OK`, `PDF`, `email`, `Wi-Fi`",
        "collect them in one terms line at the end",
        "### Conclusion first",
        "If the whole reply is a question to the person",
        "### Prefer tables and diagrams",
        "about four columns",
        "### Mermaid rules",
        "A[\"Gửi yêu cầu (HTTP)\"]",
        "### Everyday examples",
        "when a concept is abstract or the person seems unsure",
        "### Length and ending",
        "Order the end of a reply as: terms line",
        "Skip that question for a one-line answer",
        "### Required facts",
        "Bạn muốn mình giải thích sâu hơn phần nào?",
    ] {
        assert!(
            workflow.contains(required),
            "WORKFLOW.md is missing the single communication rule {required:?}"
        );
    }

    for removed in ["`expert`", "`intermediate`", "`layperson`", "### Levels"] {
        assert!(
            !workflow.contains(removed),
            "WORKFLOW.md still exposes removed reply level {removed:?}"
        );
    }
}

#[test]
fn distribution_no_longer_ships_or_prompts_for_a_communication_choice() {
    let root = repository_root();
    assert!(
        !root
            .join("distribution/payload/.truss/core/docs/templates/communication.md")
            .exists(),
        "the obsolete communication choice template is still shipped"
    );

    for path in [
        "distribution/payload/.truss/core/docs/WORKFLOW.md",
        "distribution/payload/.truss/core/docs/README.md",
        "distribution/payload/.agents/skills/delivery/SKILL.md",
        "distribution/payload/.agents/skills/delivery-setup/SKILL.md",
        "crates/truss/src/infrastructure/embedded_distribution.rs",
        "scripts/truss-install-files.txt",
        "distribution/entrypoints/agent-truss-block.md",
        "scripts/agent-truss-block.md",
        "scripts/install-truss.sh",
        "scripts/install-truss.ps1",
        "docs/installation.md",
        "docs/maintenance.md",
        "docs/delivery.md",
        "README.md",
    ] {
        let source = repository_file(path);
        assert!(
            !source.contains("communication.md"),
            "{path} still refers to the obsolete communication choice"
        );
        assert!(
            !source.contains("reply-style choice") && !source.contains("pick a level"),
            "{path} still asks the owner to choose a reply level"
        );
    }
}
