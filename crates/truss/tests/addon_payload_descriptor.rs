use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use truss::application::{AddOnPayloadPort, AddOnPayloadSpec};
use truss::domain::{AddOnDescriptor, ContentHash};
use truss::infrastructure::FileSystemAddOnPayload;

/// Fixture payload: three files across three directories.
const FIXTURE_FILES: &[(&str, &str)] = &[
    (".agents/skills/demo/SKILL.md", "demo skill\n"),
    (".agents/skills/demo/agents/openai.yaml", "name: demo\n"),
    (".agents/skills/demo/references/notes.md", "notes\n"),
];

/// Fixture manifest: the membership owner for the fixture payload.
const FIXTURE_MANIFEST: &str = "\
# fixture add-on\n\
.agents/skills/demo/SKILL.md\n\
.agents/skills/demo/agents/openai.yaml\n\
.agents/skills/demo/references/notes.md\n";

fn write_file(root: &Path, relative: &str, content: &str) {
    let target = root.join(relative);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, content).unwrap();
}

fn write_fixture_payload(root: &Path) {
    for (relative, content) in FIXTURE_FILES {
        write_file(root, relative, content);
    }
}

fn spec<'a>(
    root: &'a Path,
    manifest: &'a Path,
    name: &'a str,
    foreign_manifests: &'a [PathBuf],
) -> AddOnPayloadSpec<'a> {
    AddOnPayloadSpec {
        root,
        manifest,
        name,
        source_ref: "truss-v0.1.13",
        source_core_version: "0.1.13",
        foreign_manifests,
    }
}

fn manifest_paths(manifest: &Path) -> Vec<String> {
    fs::read_to_string(manifest)
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

fn descriptor_paths(descriptor: &AddOnDescriptor) -> Vec<String> {
    descriptor
        .files
        .iter()
        .map(|file| file.path.as_str().to_owned())
        .collect()
}

fn flipped(hash: &ContentHash) -> ContentHash {
    let value = hash.as_str();
    let first = if value.starts_with('a') { 'b' } else { 'a' };
    ContentHash::parse(format!("{first}{}", &value[1..])).unwrap()
}

/// Acceptance row 1: descriptor built from the real delivery manifest.
///
/// The two path sets are compared both as ordered lists and as sets, and every
/// descriptor digest is re-derived from the payload bytes. The descriptor path
/// set and `path sha256` pair are also written to the stable, git-ignored
/// `target/s1-evidence/` directory so the `comm`/`sha256sum` instrument can be
/// replayed from a shell after the test run.
#[test]
fn descriptor_from_real_delivery_manifest_carries_manifest_order_and_payload_digests() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repository = crate_dir.parent().unwrap().parent().unwrap().to_path_buf();
    let payload_root = repository.join("distribution").join("payload");
    let manifest = repository.join("scripts/delivery-install-files.txt");
    let foreign = [
        repository.join("scripts/truss-install-files.txt"),
        repository.join("scripts/engineering-wisdom-install-files.txt"),
        repository.join("scripts/plan-install-files.txt"),
    ];
    let spec = spec(&payload_root, &manifest, "delivery", &foreign);

    let descriptor = FileSystemAddOnPayload.describe(&spec).unwrap();
    assert_eq!(descriptor.name.as_str(), "delivery");
    assert_eq!(descriptor.source_ref.as_str(), "truss-v0.1.13");
    assert_eq!(descriptor.source_core_version, "0.1.13");

    let manifest_paths = manifest_paths(&manifest);
    let descriptor_paths = descriptor_paths(&descriptor);
    assert_eq!(descriptor_paths, manifest_paths, "ordered path set differs");

    let manifest_set = manifest_paths.iter().cloned().collect::<BTreeSet<_>>();
    let descriptor_set = descriptor_paths.iter().cloned().collect::<BTreeSet<_>>();
    assert!(
        manifest_set.difference(&descriptor_set).next().is_none(),
        "manifest paths missing from the descriptor"
    );
    assert!(
        descriptor_set.difference(&manifest_set).next().is_none(),
        "descriptor paths missing from the manifest"
    );

    for file in &descriptor.files {
        let bytes = fs::read(payload_root.join(file.path.as_str())).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            file.sha256.as_str(),
            "descriptor digest is not the digest of the payload bytes for {}",
            file.path
        );
    }

    let evidence = repository.join("target/s1-evidence");
    fs::create_dir_all(&evidence).unwrap();
    let mut sorted_paths = descriptor_set.iter().cloned().collect::<Vec<_>>();
    sorted_paths.sort();
    fs::write(
        evidence.join("s1-descriptor-paths.txt"),
        format!("{}\n", sorted_paths.join("\n")),
    )
    .unwrap();
    let mut digests = descriptor
        .files
        .iter()
        .map(|file| format!("{}  {}", file.sha256.as_str(), file.path))
        .collect::<Vec<_>>();
    digests.sort();
    fs::write(
        evidence.join("s1-descriptor-digests.txt"),
        format!("{}\n", digests.join("\n")),
    )
    .unwrap();
}

/// Acceptance row 1 counterexample: a descriptor hashed from the target
/// workspace instead of the payload still passes path completeness and must be
/// refused by the checker once the target file is locally modified.
#[test]
fn checker_refuses_descriptor_hashed_from_target_workspace() {
    let tmp = tempfile::tempdir().unwrap();
    let payload = tmp.path().join("payload");
    let target = tmp.path().join("target");
    write_fixture_payload(&payload);
    write_fixture_payload(&target);
    let manifest = tmp.path().join("demo-install-files.txt");
    fs::write(&manifest, FIXTURE_MANIFEST).unwrap();
    let foreign: [PathBuf; 0] = [];
    let payload_spec = spec(&payload, &manifest, "demo", &foreign);
    let target_spec = spec(&target, &manifest, "demo", &foreign);

    // A target workspace file is locally modified after the install.
    write_file(&target, FIXTURE_FILES[0].0, "locally modified\n");

    let correct = FileSystemAddOnPayload.describe(&payload_spec).unwrap();
    let wrong = FileSystemAddOnPayload.describe(&target_spec).unwrap();
    assert_eq!(
        descriptor_paths(&wrong),
        descriptor_paths(&correct),
        "the wrong descriptor is expected to keep path completeness"
    );

    FileSystemAddOnPayload
        .verify(&payload_spec, &correct)
        .unwrap();

    let result = FileSystemAddOnPayload.verify(&payload_spec, &wrong);
    assert!(
        result.is_err(),
        "checker accepted a descriptor hashed from the target workspace: {result:?}"
    );
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("add-on payload digest mismatch"),
        "expected a digest refusal, got: {error}"
    );
}

/// Acceptance row 2: a manifest path absent from the payload is refused.
#[test]
fn describe_refuses_path_set_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let payload = tmp.path().join("payload");
    write_fixture_payload(&payload);
    let manifest = tmp.path().join("demo-install-files.txt");
    fs::write(
        &manifest,
        format!("{FIXTURE_MANIFEST}.agents/skills/demo/missing.md\n"),
    )
    .unwrap();
    let foreign: [PathBuf; 0] = [];

    let error = FileSystemAddOnPayload
        .describe(&spec(&payload, &manifest, "demo", &foreign))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("add-on payload path-set mismatch"),
        "expected a path-set refusal, got: {error}"
    );
}

/// Acceptance row 2: a descriptor that drops a manifest path is refused even
/// when the payload itself is complete.
#[test]
fn verify_refuses_path_set_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let payload = tmp.path().join("payload");
    write_fixture_payload(&payload);
    let manifest = tmp.path().join("demo-install-files.txt");
    fs::write(&manifest, FIXTURE_MANIFEST).unwrap();
    let foreign: [PathBuf; 0] = [];
    let spec = spec(&payload, &manifest, "demo", &foreign);

    let mut descriptor = FileSystemAddOnPayload.describe(&spec).unwrap();
    descriptor.files.pop();

    let error = FileSystemAddOnPayload
        .verify(&spec, &descriptor)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("add-on payload path-set mismatch"),
        "expected a path-set refusal, got: {error}"
    );
}

/// Acceptance row 2: a tampered payload byte is refused; path completeness is
/// unaffected, so only the digest comparison can catch it.
#[test]
fn verify_refuses_digest_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let payload = tmp.path().join("payload");
    write_fixture_payload(&payload);
    let manifest = tmp.path().join("demo-install-files.txt");
    fs::write(&manifest, FIXTURE_MANIFEST).unwrap();
    let foreign: [PathBuf; 0] = [];
    let spec = spec(&payload, &manifest, "demo", &foreign);

    let mut descriptor = FileSystemAddOnPayload.describe(&spec).unwrap();
    descriptor.files[0].sha256 = flipped(&descriptor.files[0].sha256);
    let tampered = descriptor.files[0].path.clone();
    let error = match FileSystemAddOnPayload.verify(&spec, &descriptor) {
        Ok(()) => panic!("tampered digest accepted for {tampered}"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("add-on payload digest mismatch"),
        "expected a digest refusal, got: {error}"
    );
}

/// Acceptance row 2: a manifest path that escapes the repository is refused.
#[test]
fn describe_refuses_escaping_path() {
    let tmp = tempfile::tempdir().unwrap();
    let payload = tmp.path().join("payload");
    write_fixture_payload(&payload);
    let foreign: [PathBuf; 0] = [];

    for escaping in ["../escape.md", "/absolute.md", "demo/../../escape.md"] {
        let manifest = tmp.path().join("demo-install-files.txt");
        fs::write(&manifest, format!("{escaping}\n")).unwrap();
        let result = FileSystemAddOnPayload.describe(&spec(&payload, &manifest, "demo", &foreign));
        assert!(result.is_err(), "accepted escaping path {escaping}");
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("escapes the repository"),
            "expected an escape refusal for {escaping}, got: {error}"
        );
    }
}

/// Acceptance row 2: a symlink payload entry is refused, both as the leaf and
/// as a parent directory component.
#[cfg(unix)]
#[test]
fn describe_refuses_symlink_entry() {
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().unwrap();
    let foreign: [PathBuf; 0] = [];

    let leaf_root = tmp.path().join("leaf");
    write_file(&leaf_root, "real.md", "demo skill\n");
    write_file(
        &leaf_root,
        ".agents/skills/demo/agents/openai.yaml",
        "name: demo\n",
    );
    write_file(
        &leaf_root,
        ".agents/skills/demo/references/notes.md",
        "notes\n",
    );
    symlink(
        "../../../real.md",
        leaf_root.join(".agents/skills/demo/SKILL.md"),
    )
    .unwrap();
    let leaf_manifest = tmp.path().join("leaf-manifest.txt");
    fs::write(&leaf_manifest, FIXTURE_MANIFEST).unwrap();
    let error = FileSystemAddOnPayload
        .describe(&spec(&leaf_root, &leaf_manifest, "demo", &foreign))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("symlink"),
        "expected a symlink refusal, got: {error}"
    );

    let directory_root = tmp.path().join("directory");
    write_file(&directory_root, "link-source/agents/openai.yaml", "a\n");
    fs::create_dir_all(directory_root.join(".agents/skills")).unwrap();
    symlink(
        directory_root.join("link-source"),
        directory_root.join(".agents/skills/demo"),
    )
    .unwrap();
    let directory_manifest = tmp.path().join("directory-manifest.txt");
    fs::write(
        &directory_manifest,
        ".agents/skills/demo/agents/openai.yaml\n",
    )
    .unwrap();
    let error = FileSystemAddOnPayload
        .describe(&spec(
            &directory_root,
            &directory_manifest,
            "demo",
            &foreign,
        ))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("symlink"),
        "expected a symlink refusal, got: {error}"
    );
}

/// Acceptance row 2: a payload file with an unsupported mode is refused.
#[cfg(unix)]
#[test]
fn describe_refuses_unsupported_file_mode() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let payload = tmp.path().join("payload");
    write_fixture_payload(&payload);
    let manifest = tmp.path().join("demo-install-files.txt");
    fs::write(&manifest, FIXTURE_MANIFEST).unwrap();
    let foreign: [PathBuf; 0] = [];
    fs::set_permissions(
        payload.join(".agents/skills/demo/SKILL.md"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    let error = FileSystemAddOnPayload
        .describe(&spec(&payload, &manifest, "demo", &foreign))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("file mode is unsupported"),
        "expected a file-mode refusal, got: {error}"
    );
}

/// Plan requirement: a payload path already owned by the core or another
/// add-on is refused.
#[test]
fn describe_refuses_path_owned_by_another_distribution() {
    let tmp = tempfile::tempdir().unwrap();
    let payload = tmp.path().join("payload");
    write_fixture_payload(&payload);
    let manifest = tmp.path().join("demo-install-files.txt");
    fs::write(&manifest, FIXTURE_MANIFEST).unwrap();
    let foreign_manifest = tmp.path().join("core-install-files.txt");
    fs::write(&foreign_manifest, ".agents/skills/demo/SKILL.md\n").unwrap();

    let error = FileSystemAddOnPayload
        .describe(&spec(
            &payload,
            &manifest,
            "demo",
            std::slice::from_ref(&foreign_manifest),
        ))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("already owned by"),
        "expected an ownership refusal, got: {error}"
    );
}
