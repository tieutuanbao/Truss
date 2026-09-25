//! Fixture helpers shared by the add-on integration suites.
//!
//! `seed_core_state` establishes the valid pre-existing core state that
//! decision 0003 clause 8 requires before any add-on operation, and
//! `workspace_snapshot` captures the complete path/type/content view that a
//! refusal or a dry run must leave byte-identical.

#![allow(dead_code)]

use std::fs;
use std::path::Path;

use serde_json::json;
use sha2::{Digest, Sha256};
use truss::application::CoreDistributionPort;
use truss::infrastructure::EmbeddedCoreDistribution;

/// The core-owned shared artifacts decision 0003 clause 8 requires before an
/// add-on operation. A real core install writes exactly these rules through
/// `state_io::ensure_state_ignore`; add-on operations validate them read-only
/// and never create or repair them.
pub const CORE_STATE_IGNORE: &str =
    "/lock\n/transaction.json\n/base.next-*\n/update/\n/update-candidate/\n/addon-update/\n";

pub fn write_bytes(root: &Path, relative: &str, content: &[u8]) {
    let target = root.join(relative);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, content).unwrap();
}

pub fn write_file(root: &Path, relative: &str, content: &str) {
    write_bytes(root, relative, content.as_bytes());
}

pub fn seed_core_state(workspace: &Path) {
    let state_root = workspace.join(".truss/core");
    fs::create_dir_all(&state_root).unwrap();
    fs::write(state_root.join(".gitignore"), CORE_STATE_IGNORE).unwrap();
    fs::write(state_root.join("lock"), b"").unwrap();
    // Decision 0003 clause 13 makes `.truss/core/manifest.json` part of a
    // valid pre-existing core state, so the fixture must be the real installed
    // shape: the embedded core payload, the four core skill trees, and the
    // digest-checked `.truss/core/base/` copies. A `.gitignore` + `lock` only
    // fixture is the incomplete state the add-on path must refuse. Decision
    // 0008 makes `.truss/core` the installed root for this shape.
    let distribution = EmbeddedCoreDistribution.current().unwrap();
    let mut files = Vec::new();
    for file in &distribution.files {
        write_bytes(workspace, file.path.as_str(), &file.content);
        write_bytes(
            workspace,
            &format!(".truss/core/base/{}", file.path.as_str()),
            &file.content,
        );
        files.push(json!({
            "path": file.path.as_str(),
            "upstream_sha256": file.hash.as_str(),
        }));
    }
    let manifest = json!({
        "schema_version": 1,
        "core_version": distribution.version,
        "files": files,
    });
    fs::write(
        state_root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
}

/// A complete workspace snapshot: every path with its type and, for a file,
/// its content digest; for a symlink, its target. Path set, type, and content
/// are compared, so a mutation-free operation must leave this string
/// byte-identical.
pub fn workspace_snapshot(root: &Path) -> String {
    let mut lines = Vec::new();
    walk_snapshot(root, root, &mut lines);
    lines.sort();
    lines.join("\n")
}

fn walk_snapshot(root: &Path, directory: &Path, lines: &mut Vec<String>) {
    let mut entries = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let metadata = fs::symlink_metadata(&path).unwrap();
        if metadata.file_type().is_symlink() {
            let target = fs::read_link(&path).unwrap();
            lines.push(format!("symlink {relative} -> {}", target.display()));
        } else if metadata.is_dir() {
            lines.push(format!("dir {relative}"));
            walk_snapshot(root, &path, lines);
        } else if metadata.is_file() {
            let bytes = fs::read(&path).unwrap();
            lines.push(format!("file {relative} {:x}", Sha256::digest(&bytes)));
        } else {
            lines.push(format!("other {relative}"));
        }
    }
}

pub fn snapshot_digest(snapshot: &str) -> String {
    format!("{:x}", Sha256::digest(snapshot.as_bytes()))
}
