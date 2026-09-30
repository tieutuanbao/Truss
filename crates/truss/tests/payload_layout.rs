//! Payload layout contract: the bytes the installers ship.
//!
//! `distribution/payload` is the only payload source. The binary embeds it with
//! `include_bytes!` at compile time, so a silent edit to a payload file ships
//! stale content to every consumer, and no compiler error reports it. This suite
//! is the mechanical guard for that boundary.
//!
//! Each test owns one rule and names the file it rejected:
//!
//! * the layout marker declares the supported layout;
//! * every manifest destination resolves to exactly one source;
//! * no payload file escapes the manifest membership;
//! * every generated destination names a generator input beneath
//!   `distribution/entrypoints/`;
//! * every shipped byte matches the reviewed baseline in
//!   `tests/payload-layout-digests.txt`;
//! * both entrypoint blocks exist and each equals its `scripts/` counterpart;
//! * the mirror carries no legacy `.truss-core` directory.
//!
//! The digest baseline is reviewed data, not a generated report. After a
//! deliberate payload change, regenerate it and commit it in the same change,
//! then say in the commit message that the baseline moved on purpose:
//!
//! ```text
//! python3 -c "
//! import hashlib, os
//! rows = []
//! for root in ('distribution/payload', 'distribution/entrypoints'):
//!     for base, _, names in os.walk(root):
//!         for name in names:
//!             path = os.path.join(base, name)
//!             digest = hashlib.sha256(open(path, 'rb').read()).hexdigest()
//!             rows.append((path, digest))
//! with open('crates/truss/tests/payload-layout-digests.txt', 'w') as out:
//!     for path, digest in sorted(rows):
//!         out.write('%s  %s\n' % (digest, path))
//! "
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The installed layout decision 0008 introduced.
const SUPPORTED_LAYOUT: &str = "3";

/// The legacy directory name decision 0008 replaced.
const LEGACY_DIRECTORY: &str = ".truss-core";

/// Generator inputs the binary embeds; both must exist and stay canonical.
const ENTRYPOINT_BLOCKS: [&str; 2] = ["agent-truss-block.md", "claude-truss-block.md"];

/// The reviewed byte expectations, embedded at compile time so a test run needs
/// no working directory beyond the crate.
const DIGEST_BASELINE: &str = include_str!("payload-layout-digests.txt");

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate directory has a parent")
        .parent()
        .expect("the workspace directory has a parent")
        .to_path_buf()
}

fn payload_root() -> PathBuf {
    repository_root().join("distribution").join("payload")
}

fn entrypoint_root() -> PathBuf {
    repository_root().join("distribution").join("entrypoints")
}

/// Every file beneath `root`, as sorted `/`-separated relative paths.
fn relative_files(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
        for entry in entries {
            let path = entry.expect("a readable directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .expect("a walked path stays beneath its root")
                    .to_string_lossy()
                    .replace('\\', "/");
                found.push(relative);
            }
        }
    }
    found.sort();
    found
}

fn sha256(path: &Path) -> String {
    let bytes =
        fs::read(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    format!("{:x}", Sha256::digest(bytes))
}

/// Every destination with the manifest and line that owns it.
fn manifest_entries() -> Vec<(String, usize, String)> {
    let scripts = repository_root().join("scripts");
    let mut manifests: Vec<PathBuf> = fs::read_dir(&scripts)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", scripts.display()))
        .map(|entry| entry.expect("a readable directory entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("-install-files.txt"))
        })
        .collect();
    manifests.sort();

    let mut entries = Vec::new();
    for manifest in manifests {
        let name = manifest
            .strip_prefix(&scripts)
            .expect("a manifest stays beneath scripts")
            .to_string_lossy()
            .replace('\\', "/");
        let name = format!("scripts/{name}");
        let text = fs::read_to_string(&manifest)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", manifest.display()));
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            entries.push((name.clone(), index + 1, line.to_owned()));
        }
    }
    entries
}

/// `distribution/generated.txt` records as (line, destination, generator, prefix).
fn generated_declarations() -> Vec<(usize, String, String, String)> {
    let path = repository_root().join("distribution").join("generated.txt");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            3,
            "distribution/generated.txt:{number} has {} tab-separated fields, \
             expected 3 (destination, generator input, prefix)",
            fields.len()
        );
        records.push((
            number,
            fields[0].to_owned(),
            fields[1].to_owned(),
            fields[2].to_owned(),
        ));
    }
    records
}

/// The reviewed baseline as {repo-relative path: sha256}.
fn digest_baseline() -> BTreeMap<String, String> {
    let mut baseline = BTreeMap::new();
    for (index, line) in DIGEST_BASELINE.lines().enumerate() {
        let number = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        let Some((digest, path)) = line.split_once("  ") else {
            panic!(
                "crates/truss/tests/payload-layout-digests.txt:{number} is not \
                 '<sha256>  <repo-relative path>'"
            );
        };
        assert_eq!(
            digest.len(),
            64,
            "crates/truss/tests/payload-layout-digests.txt:{number} does not carry a sha256"
        );
        assert!(
            baseline
                .insert(path.to_owned(), digest.to_owned())
                .is_none(),
            "crates/truss/tests/payload-layout-digests.txt:{number} records {path} twice"
        );
    }
    baseline
}

#[test]
fn layout_marker_declares_the_supported_layout() {
    let path = repository_root()
        .join("distribution")
        .join("layout-version");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let declared: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(
        declared,
        vec![SUPPORTED_LAYOUT],
        "distribution/layout-version must declare exactly layout {SUPPORTED_LAYOUT}, \
         because an unknown or missing declaration has to fail before any mutation"
    );
}

#[test]
fn every_manifest_destination_resolves_to_exactly_one_source() {
    let entries = manifest_entries();
    assert!(
        !entries.is_empty(),
        "no manifest destination was read from scripts/*-install-files.txt; the \
         membership owner is missing"
    );

    let mut listed: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for (manifest, line, destination) in &entries {
        if let Some((previous, previous_line)) = listed.get(destination) {
            panic!(
                "{destination} is listed more than once in the manifests \
                 ({previous}:{previous_line} and {manifest}:{line}); each destination \
                 must have exactly one manifest line"
            );
        }
        listed.insert(destination.clone(), (manifest.clone(), *line));
    }

    let mut declared: BTreeMap<String, usize> = BTreeMap::new();
    for (line, destination, _, _) in &generated_declarations() {
        if let Some(previous) = declared.get(destination) {
            panic!(
                "distribution/generated.txt declares {destination} twice \
                 (lines {previous} and {line}); a destination must have exactly one \
                 declaration"
            );
        }
        declared.insert(destination.clone(), *line);
    }

    for (destination, line) in &declared {
        assert!(
            listed.contains_key(destination),
            "distribution/generated.txt:{line} declares {destination}, which no \
             manifest lists; the manifest is the membership owner and a declaration \
             must not introduce a destination"
        );
    }

    for destination in listed.keys() {
        let has_file = payload_root().join(destination).is_file();
        let has_declaration = declared.contains_key(destination);
        match (has_file, has_declaration) {
            (true, true) => panic!(
                "{destination} resolves to both distribution/payload/{destination} and \
                 a generated declaration; a destination must resolve to exactly one \
                 source"
            ),
            (false, false) => panic!(
                "{destination} resolves to no source: expected \
                 distribution/payload/{destination} or a generated declaration"
            ),
            _ => {}
        }
    }
}

#[test]
fn every_payload_file_matches_a_manifest_destination() {
    let listed: BTreeSet<String> = manifest_entries()
        .into_iter()
        .map(|(_, _, destination)| destination)
        .collect();
    let orphans: Vec<String> = relative_files(&payload_root())
        .into_iter()
        .filter(|path| !listed.contains(path))
        .collect();
    assert!(
        orphans.is_empty(),
        "these distribution/payload files match no manifest destination, so the \
         manifest no longer owns the shipped membership: {orphans:?}"
    );
}

#[test]
fn every_generated_destination_names_a_generator_under_entrypoints() {
    let entrypoints = entrypoint_root();
    let canonical = entrypoints
        .canonicalize()
        .unwrap_or_else(|error| panic!("cannot resolve {}: {error}", entrypoints.display()));

    for (line, destination, generator, _prefix) in generated_declarations() {
        assert!(
            generator.starts_with("distribution/entrypoints/"),
            "distribution/generated.txt:{line} destination {destination} names \
             generator input {generator} outside distribution/entrypoints/"
        );
        let path = repository_root().join(&generator);
        assert!(
            path.is_file(),
            "distribution/generated.txt:{line} destination {destination} names \
             generator input {generator}, which does not exist"
        );
        let resolved = path
            .canonicalize()
            .unwrap_or_else(|error| panic!("cannot resolve {}: {error}", path.display()));
        assert!(
            resolved != canonical && resolved.starts_with(&canonical),
            "distribution/generated.txt:{line} destination {destination} names \
             generator input {generator}, which resolves to {} outside {}; a spelling \
             prefix is not containment",
            resolved.display(),
            canonical.display()
        );
    }
}

#[test]
fn every_shipped_file_matches_the_reviewed_digest_baseline() {
    let baseline = digest_baseline();

    let mut shipped: BTreeMap<String, PathBuf> = BTreeMap::new();
    for file in relative_files(&payload_root()) {
        shipped.insert(
            format!("distribution/payload/{file}"),
            payload_root().join(&file),
        );
    }
    for name in ENTRYPOINT_BLOCKS {
        let path = entrypoint_root().join(name);
        assert!(
            path.is_file(),
            "distribution/entrypoints/{name} is missing; a missing generator input \
             is silent, so every entrypoint block must exist there"
        );
        shipped.insert(format!("distribution/entrypoints/{name}"), path);
    }

    let unrecorded: Vec<&String> = shipped
        .keys()
        .filter(|path| !baseline.contains_key(*path))
        .collect();
    assert!(
        unrecorded.is_empty(),
        "these shipped files carry no reviewed digest, so their bytes are \
         unverified: {unrecorded:?}"
    );

    let stale: Vec<&String> = baseline
        .keys()
        .filter(|path| !shipped.contains_key(*path))
        .collect();
    assert!(
        stale.is_empty(),
        "the digest baseline records files the distribution tree does not hold; the \
         baseline and the tree must describe the same file set: {stale:?}"
    );

    let drifted: Vec<String> = shipped
        .iter()
        .filter_map(|(path, full)| {
            let actual = sha256(full);
            let expected = &baseline[path];
            (actual != *expected).then(|| format!("{path}: baseline {expected}, actual {actual}"))
        })
        .collect();
    assert!(
        drifted.is_empty(),
        "these shipped bytes moved since the baseline was reviewed; update the \
         baseline deliberately or revert the payload edit: {drifted:#?}"
    );
}

#[test]
fn entrypoint_blocks_match_their_scripts_counterparts() {
    for name in ENTRYPOINT_BLOCKS {
        let shipped = entrypoint_root().join(name);
        let counterpart = repository_root().join("scripts").join(name);
        let shipped_bytes = fs::read(&shipped)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", shipped.display()));
        let counterpart_bytes = fs::read(&counterpart)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", counterpart.display()));
        if shipped_bytes != counterpart_bytes {
            panic!(
                "distribution/entrypoints/{name} differs from scripts/{name}; both \
                 copies of one entrypoint block must stay byte-identical while both \
                 routes ship"
            );
        }
    }
}

#[test]
fn the_payload_mirror_carries_no_legacy_directory() {
    let legacy = payload_root().join(LEGACY_DIRECTORY);
    assert!(
        !legacy.is_dir(),
        "distribution/payload/{LEGACY_DIRECTORY} exists; the installed destination \
         prefix is .truss/core since decision 0008, and a legacy directory in the \
         mirror means a source was not renamed with its destination"
    );
}
