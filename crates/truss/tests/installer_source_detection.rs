#![cfg(unix)]

//! The installer must recognise a *published* checkout as its own source.
//!
//! `scripts/install-truss.sh` resolves where its payload comes from three ways:
//! a local source checkout, a git clone, or a remote base URL. Local mode is
//! chosen by the `distribution/` tree, which only a source repository carries —
//! an installed consumer has `.truss/core/` and no `distribution/`.
//!
//! The gate used to *also* require `AGENTS.md` at the source root. The 0.2.9
//! publish deliberately stopped publishing `AGENTS.md`, because the local Truss
//! installation and the agent entrypoints stay on the machine that owns them. The
//! extra condition therefore made the gate unreachable from any checkout of the
//! published tree, and a contributor running the documented local install saw
//!
//! ```text
//! Error: the Truss source at  declares no distribution/layout-version; this
//! bootstrap requires layout 3 and refuses to guess
//! ```
//!
//! with an empty source name — a message about the layout marker, produced by a
//! gate that never looked at the source at all.
//!
//! These tests pin local mode to the distribution tree alone. Two of them would
//! have caught the defect: the first because a published checkout has no
//! `AGENTS.md`, the third because widening local mode must not also stop reading
//! the layout marker's value.
//!
//! Binary staging is short-circuited with `TRUSS_CORE_BINARY` so these tests
//! measure source resolution and never invoke `cargo`; `--dry-run` means nothing
//! is written to the target.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("create directory");
    for entry in fs::read_dir(source).expect("read directory") {
        let entry = entry.expect("directory entry");
        let destination = target.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), &destination).expect("copy file");
        }
    }
}

/// A throwaway checkout carrying only what the installer reads while resolving
/// its source: the scripts and the distribution tree.
///
/// `layout` is written into `distribution/layout-version`, and `None` removes the
/// marker. The real repository cannot be used directly, because its own
/// `AGENTS.md` is present on disk as an untracked local-only file and would
/// satisfy the old gate. A published checkout does not have it.
fn source_tree(workspace: &Path, with_agents_md: bool, layout: Option<&str>) -> PathBuf {
    let root = workspace.join("source");
    copy_tree(&repo_root().join("scripts"), &root.join("scripts"));
    copy_tree(
        &repo_root().join("distribution"),
        &root.join("distribution"),
    );
    match layout {
        Some(value) => fs::write(
            root.join("distribution/layout-version"),
            format!("{value}\n"),
        )
        .expect("write layout marker"),
        None => {
            fs::remove_file(root.join("distribution/layout-version")).expect("remove layout marker")
        }
    }
    if with_agents_md {
        fs::write(root.join("AGENTS.md"), "# entrypoint\n").expect("write entrypoint");
    }
    root
}

fn run_installer(source: &Path, target: &Path) -> std::process::Output {
    fs::create_dir_all(target).expect("create target");
    Command::new("bash")
        .arg(source.join("scripts/install-truss.sh"))
        .arg("--directory")
        .arg(target)
        .arg("--dry-run")
        .arg("--yes")
        .env("TRUSS_CORE_BINARY", env!("CARGO_BIN_EXE_truss"))
        .env_remove("TRUSS_SOURCE_BASE_URL")
        .env_remove("TRUSS_CORE_SOURCE_BASE_URL")
        .env_remove("TRUSS_SOURCE_GIT")
        .output()
        .expect("run the installer")
}

fn combined_output(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The regression: a checkout of the published tree carries no `AGENTS.md`.
#[test]
fn a_published_checkout_is_recognised_as_its_own_source() {
    let workspace = tempfile::tempdir().expect("temp dir");
    let source = source_tree(workspace.path(), false, Some("3"));
    assert!(
        !source.join("AGENTS.md").exists(),
        "the fixture must model a published checkout, which has no AGENTS.md"
    );

    let output = run_installer(&source, &workspace.path().join("target"));
    let text = combined_output(&output);

    assert!(
        output.status.success(),
        "a published checkout must install from its own distribution tree; output:\n{text}"
    );
    assert!(
        text.contains(&format!("Truss source: {}", source.display())),
        "the installer must name the checkout as its source; output:\n{text}"
    );
    assert!(
        !text.contains("declares no distribution/layout-version"),
        "the layout marker is present, so the layout error must not appear; output:\n{text}"
    );
}

/// A local checkout that still has an entrypoint keeps working. This guards the
/// fix from over-correcting into "never require anything".
#[test]
fn a_source_root_that_still_has_an_entrypoint_is_recognised() {
    let workspace = tempfile::tempdir().expect("temp dir");
    let source = source_tree(workspace.path(), true, Some("3"));

    let output = run_installer(&source, &workspace.path().join("target"));
    let text = combined_output(&output);

    assert!(
        output.status.success(),
        "an entrypoint must not prevent local-source detection; output:\n{text}"
    );
    assert!(
        text.contains(&format!("Truss source: {}", source.display())),
        "the installer must name the checkout as its source; output:\n{text}"
    );
}

/// The counterexample the instrument must reject: a distribution tree declaring
/// a layout this bootstrap cannot install must still be refused.
///
/// A wrong-but-passing implementation would widen local mode and stop reading the
/// marker's value, installing a payload whose layout it never checked.
#[test]
fn a_source_declaring_another_layout_is_refused() {
    let workspace = tempfile::tempdir().expect("temp dir");
    let source = source_tree(workspace.path(), false, Some("2"));

    let output = run_installer(&source, &workspace.path().join("target"));
    let text = combined_output(&output);

    assert!(
        !output.status.success(),
        "a source declaring layout 2 must not install; output:\n{text}"
    );
    assert!(
        text.contains("declares layout 2"),
        "the refusal must name the declared layout; output:\n{text}"
    );
    assert!(
        text.contains("requires layout 3"),
        "the refusal must name the required layout; output:\n{text}"
    );
}
