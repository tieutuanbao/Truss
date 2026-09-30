use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::Version;
use sha2::{Digest, Sha256};

use crate::application::{CandidateExit, CandidateRequest, PortError, UpdateCandidatePort};

use super::state_io::{legacy_state_root, NEW_STATE_DIR};

/// The installed tree's root components, relative to the repository root.
///
/// Decision 0008 moved the installed tree to `.truss/core`, and the helpers
/// below compare paths by component so a symlinked component can be refused by
/// name. An existing installation that predates 0008 still has its executable
/// at `.truss-core/bin`, so the name is resolved the same way a state read is:
/// whichever tree is installed wins.
fn state_root_components(root: &Path) -> Vec<&'static str> {
    let new = NEW_STATE_DIR.split('/').collect::<Vec<_>>();
    let legacy = legacy_state_root(root);
    let new_root = root.join(NEW_STATE_DIR);
    if new_root.join("manifest.json").exists() || new_root.join("base").exists() {
        return new;
    }
    if legacy.join("manifest.json").exists() || legacy.join("base").exists() {
        return vec![".truss-core"];
    }
    new
}

fn state_path(root: &Path, tail: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for component in state_root_components(root) {
        path.push(component);
    }
    for component in tail {
        path.push(component);
    }
    path
}

fn state_components(root: &Path, tail: &[&str]) -> Vec<String> {
    let mut components = state_root_components(root)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    components.extend(tail.iter().map(|value| (*value).to_owned()));
    components
}

fn state_label(root: &Path, tail: &[&str]) -> String {
    let mut parts = state_root_components(root);
    parts.extend_from_slice(tail);
    parts.join("/")
}

const RELEASE_REPO_ENV: &str = "TRUSS_RELEASE_REPO";

fn configured_release_repo() -> Result<String, PortError> {
    env::var(RELEASE_REPO_ENV)
        .ok()
        .filter(|repo| !repo.trim().is_empty())
        .map(|repo| repo.trim().trim_matches('/').to_owned())
        .ok_or_else(|| {
            PortError::new(format!(
                "self-update release repository is not configured; set {RELEASE_REPO_ENV} to 'owner/name'"
            ))
        })
}

pub struct LatestReleaseCandidates {
    test_release_root: Option<String>,
    replace_executable: bool,
}

pub struct VerifiedCandidate {
    _temp: tempfile::TempDir,
    path: PathBuf,
    release_version: String,
}

impl Default for LatestReleaseCandidates {
    fn default() -> Self {
        let test_release_root = cfg!(debug_assertions)
            .then(|| env::var("TRUSS_TEST_RELEASE_ROOT").ok())
            .flatten();
        let replace_executable =
            !(cfg!(debug_assertions) && env::var_os("TRUSS_TEST_SKIP_SELF_REPLACE").is_some());
        Self {
            test_release_root,
            replace_executable,
        }
    }
}

impl UpdateCandidatePort for LatestReleaseCandidates {
    type Candidate = VerifiedCandidate;

    fn latest(&self) -> Result<Self::Candidate, PortError> {
        let temp = tempfile::NamedTempFile::new().map_err(io_error)?;
        fetch_url(&self.pointer_url()?, temp.path())?;
        let content = fs::read_to_string(temp.path()).map_err(io_error)?;
        let tag = content
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .ok_or_else(|| PortError::new("Truss core release pointer is empty"))?;
        let version = parse_release_tag(tag)?;
        self.download_version(&version)
    }

    fn exact(&self, version: &str) -> Result<Self::Candidate, PortError> {
        Version::parse(version)
            .map_err(|error| PortError::new(format!("invalid core version {version}: {error}")))?;
        self.download_version(version)
    }

    fn staged(&self, root: &Path, version: &str) -> Result<Self::Candidate, PortError> {
        Version::parse(version).map_err(|error| {
            PortError::new(format!("invalid staged version {version}: {error}"))
        })?;
        let staged = require_regular_repository_file(
            root,
            &state_components(root, &["update-candidate", candidate_filename()]),
            "staged update candidate",
        )?;
        let temp = tempfile::tempdir().map_err(io_error)?;
        let path = temp.path().join(candidate_filename());
        fs::copy(staged, &path).map_err(io_error)?;
        let expected = self.fetch_expected_checksum(version, temp.path())?;
        verify_candidate(&path, &expected)?;
        make_executable(&path)?;
        Ok(VerifiedCandidate {
            _temp: temp,
            path,
            release_version: version.to_owned(),
        })
    }

    fn release_version<'a>(&self, candidate: &'a Self::Candidate) -> &'a str {
        &candidate.release_version
    }

    fn reported_version(&self, candidate: &Self::Candidate) -> Result<String, PortError> {
        read_candidate_version(&candidate.path)
    }

    fn execute(
        &self,
        candidate: &Self::Candidate,
        request: &CandidateRequest<'_>,
    ) -> Result<CandidateExit, PortError> {
        let mut command = Command::new(&candidate.path);
        command
            .arg("update")
            .arg("--candidate")
            .arg("--directory")
            .arg(request.root);
        if request.dry_run {
            command.arg("--dry-run");
        }
        if request.continue_update {
            command.arg("--continue");
        }
        if request.json {
            command.arg("--json");
        }
        let output = command.output().map_err(|error| {
            PortError::new(format!(
                "could not execute verified update candidate: {error}"
            ))
        })?;
        Ok(CandidateExit {
            code: output.status.code().unwrap_or(1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    fn persist(&self, root: &Path, candidate: &Self::Candidate) -> Result<(), PortError> {
        let target_root = persisted_root(root);
        reject_existing_repository_symlinks(
            root,
            &state_components(root, &["update-candidate"]),
            &state_label(root, &["update-candidate"]),
        )?;
        fs::create_dir_all(&target_root).map_err(io_error)?;
        require_repository_directories(
            root,
            &state_components(root, &["update-candidate"]),
            &state_label(root, &["update-candidate"]),
        )?;
        let target = persisted_candidate(root);
        reject_existing_repository_symlinks(
            root,
            &state_components(root, &["update-candidate", candidate_filename()]),
            "staged update candidate",
        )?;
        if target.exists() {
            require_regular_repository_file(
                root,
                &state_components(root, &["update-candidate", candidate_filename()]),
                "staged update candidate",
            )?;
        }
        fs::copy(&candidate.path, &target).map_err(io_error)?;
        make_executable(&target)
    }

    fn clear_persisted(&self, root: &Path) -> Result<(), PortError> {
        let target_root = persisted_root(root);
        match fs::symlink_metadata(&target_root) {
            Ok(_) => {
                require_repository_directories(
                    root,
                    &state_components(root, &["update-candidate"]),
                    &state_label(root, &["update-candidate"]),
                )?;
                fs::remove_dir_all(target_root).map_err(io_error)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(error)),
        }
        Ok(())
    }

    fn validate_replacement_target(&self, root: &Path) -> Result<(), PortError> {
        if !self.replace_executable {
            return Ok(());
        }
        let expected_path = require_regular_repository_file(
            root,
            &state_components(root, &["bin", candidate_filename()]),
            "installed repository executable",
        )?;
        let expected = expected_path.canonicalize().map_err(io_error)?;
        let current = env::current_exe()
            .and_then(|path| path.canonicalize())
            .map_err(io_error)?;
        if current != expected {
            return Err(PortError::new(format!(
                "refusing to update {} while running {}; invoke the repository-local executable",
                expected.display(),
                current.display()
            )));
        }
        Ok(())
    }

    fn replace(&self, candidate: &Self::Candidate) -> Result<(), PortError> {
        if !self.replace_executable {
            return Ok(());
        }
        self_replace::self_replace(&candidate.path).map_err(|error| {
            PortError::new(format!("could not replace installed executable: {error}"))
        })
    }
}

impl LatestReleaseCandidates {
    fn pointer_url(&self) -> Result<String, PortError> {
        if let Some(root) = self.test_release_root.as_ref() {
            return Ok(format!("{}/truss-release-tag", root.trim_end_matches('/')));
        }
        Ok(format!(
            "https://raw.githubusercontent.com/{}/main/scripts/truss-release-tag",
            configured_release_repo()?
        ))
    }

    fn download_version(&self, version: &str) -> Result<VerifiedCandidate, PortError> {
        let temp = tempfile::tempdir().map_err(io_error)?;
        let path = temp.path().join(candidate_filename());
        let artifact = candidate_artifact()?;
        let base_url = self.release_base_url(version)?;
        fetch(&base_url, artifact, &path)?;
        let expected = self.fetch_expected_checksum(version, temp.path())?;
        verify_candidate(&path, &expected)?;
        make_executable(&path)?;
        Ok(VerifiedCandidate {
            _temp: temp,
            path,
            release_version: version.to_owned(),
        })
    }

    fn fetch_expected_checksum(&self, version: &str, temp: &Path) -> Result<String, PortError> {
        let artifact = candidate_artifact()?;
        let checksum = temp.join(format!("{artifact}.sha256"));
        fetch(
            &self.release_base_url(version)?,
            &format!("{artifact}.sha256"),
            &checksum,
        )?;
        read_expected_checksum(&checksum)
    }

    fn release_base_url(&self, version: &str) -> Result<String, PortError> {
        let tag = format!("truss-v{version}");
        if let Some(root) = self.test_release_root.as_ref() {
            return Ok(format!("{}/{tag}", root.trim_end_matches('/')));
        }
        Ok(format!(
            "https://github.com/{}/releases/download/{tag}",
            configured_release_repo()?
        ))
    }

    #[cfg(test)]
    fn for_test(root: &Path) -> Self {
        Self {
            test_release_root: Some(format!("file://{}", root.display())),
            replace_executable: false,
        }
    }
}

fn parse_release_tag(tag: &str) -> Result<String, PortError> {
    let version = tag
        .strip_prefix("truss-v")
        .ok_or_else(|| PortError::new(format!("invalid Truss core release tag: {tag}")))?;
    Version::parse(version)
        .map_err(|error| PortError::new(format!("invalid release tag {tag}: {error}")))?;
    Ok(version.to_owned())
}

fn read_candidate_version(path: &Path) -> Result<String, PortError> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| PortError::new(format!("could not query candidate version: {error}")))?;
    if !output.status.success() {
        return Err(PortError::new("candidate did not report its version"));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.split_whitespace()
        .last()
        .map(str::to_owned)
        .ok_or_else(|| PortError::new("candidate version output is empty"))
}

fn fetch(base_url: &str, asset: &str, destination: &Path) -> Result<(), PortError> {
    let url = format!("{}/{asset}", base_url.trim_end_matches('/'));
    fetch_url(&url, destination)
}

fn fetch_url(url: &str, destination: &Path) -> Result<(), PortError> {
    if let Some(path) = url.strip_prefix("file://") {
        fs::copy(path, destination).map_err(io_error)?;
        return Ok(());
    }
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--connect-timeout",
            "15",
            "--max-time",
            "120",
            url,
            "-o",
        ])
        .arg(destination)
        .output()
        .map_err(|error| PortError::new(format!("could not run curl: {error}")))?;
    if !output.status.success() {
        return Err(PortError::new(format!(
            "could not download {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

fn read_expected_checksum(path: &Path) -> Result<String, PortError> {
    let content = fs::read_to_string(path).map_err(io_error)?;
    let expected = content
        .split_whitespace()
        .next()
        .ok_or_else(|| PortError::new("candidate checksum file is empty"))?;
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PortError::new("candidate checksum is not a SHA-256 digest"));
    }
    Ok(expected.to_ascii_lowercase())
}

fn verify_candidate(path: &Path, expected: &str) -> Result<(), PortError> {
    let bytes = fs::read(path).map_err(io_error)?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(PortError::new(format!(
            "candidate checksum mismatch: expected {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn candidate_artifact() -> Result<&'static str, PortError> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok("truss-macos-arm64"),
        ("macos", "x86_64") => Ok("truss-macos-x64"),
        ("linux", "aarch64") => Ok("truss-linux-arm64"),
        ("linux", "x86_64") => Ok("truss-linux-x64"),
        ("windows", "x86_64") => Ok("truss-windows-x64.exe"),
        (os, arch) => Err(PortError::new(format!(
            "unsupported Truss update platform: {os}/{arch}"
        ))),
    }
}

fn candidate_filename() -> &'static str {
    if cfg!(windows) {
        "truss.exe"
    } else {
        "truss"
    }
}

fn persisted_root(root: &Path) -> PathBuf {
    state_path(root, &["update-candidate"])
}

fn persisted_candidate(root: &Path) -> PathBuf {
    persisted_root(root).join(candidate_filename())
}

fn reject_existing_repository_symlinks(
    root: &Path,
    components: &[String],
    label: &str,
) -> Result<(), PortError> {
    let mut current = root.to_path_buf();
    for component in components {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(PortError::new(format!("refusing symlink for {label}")));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(io_error(error)),
        }
    }
    Ok(())
}

fn require_repository_directories(
    root: &Path,
    components: &[String],
    label: &str,
) -> Result<PathBuf, PortError> {
    let mut current = root.to_path_buf();
    for component in components {
        current.push(component);
        let metadata = fs::symlink_metadata(&current).map_err(io_error)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(PortError::new(format!(
                "{label} must use repository directories without symlinks"
            )));
        }
    }
    Ok(current)
}

fn require_regular_repository_file(
    root: &Path,
    components: &[String],
    label: &str,
) -> Result<PathBuf, PortError> {
    let (file, directories) = components
        .split_last()
        .ok_or_else(|| PortError::new(format!("missing path for {label}")))?;
    let parent = require_repository_directories(root, directories, label)?;
    let path = parent.join(file);
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        PortError::new(format!(
            "{label} is unavailable at {}: {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PortError::new(format!(
            "{label} must be a regular file without symlinks"
        )));
    }
    Ok(path)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), PortError> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path).map_err(io_error)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).map_err(io_error)
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), PortError> {
    Ok(())
}

fn io_error(error: std::io::Error) -> PortError {
    PortError::new(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_candidate_that_does_not_match_its_release_checksum() {
        let release = tempfile::tempdir().unwrap();
        let version = env!("CARGO_PKG_VERSION");
        let tag_root = release.path().join(format!("truss-v{version}"));
        fs::create_dir_all(&tag_root).unwrap();
        fs::write(
            release.path().join("truss-release-tag"),
            format!("truss-v{version}\n"),
        )
        .unwrap();
        let artifact = candidate_artifact().unwrap();
        fs::write(tag_root.join(artifact), b"candidate").unwrap();
        fs::write(
            tag_root.join(format!("{artifact}.sha256")),
            format!("{}\n", "0".repeat(64)),
        )
        .unwrap();
        let candidates = LatestReleaseCandidates::for_test(release.path());
        let error = candidates.latest().err().unwrap();
        assert!(error.to_string().contains("checksum mismatch"));
    }

    #[cfg(unix)]
    #[test]
    fn repository_executable_must_not_be_a_symlink() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::create_dir_all(root.path().join(".truss/core/bin")).unwrap();
        symlink(outside.path(), root.path().join(".truss/core/bin/truss")).unwrap();

        let error = require_regular_repository_file(
            root.path(),
            &[
                ".truss/core".to_owned(),
                "bin".to_owned(),
                "truss".to_owned(),
            ],
            "installed repository executable",
        )
        .unwrap_err();
        assert!(error.to_string().contains("regular file without symlinks"));
    }

    #[cfg(unix)]
    #[test]
    fn retained_candidate_write_does_not_follow_a_symlink() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let candidate_root = tempfile::tempdir().unwrap();
        let candidate_path = candidate_root.path().join("candidate");
        fs::write(&candidate_path, b"verified candidate").unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::write(outside.path(), b"outside binary").unwrap();
        fs::create_dir_all(root.path().join(".truss/core/update-candidate")).unwrap();
        symlink(
            outside.path(),
            root.path().join(".truss/core/update-candidate/truss"),
        )
        .unwrap();
        let candidate = VerifiedCandidate {
            _temp: candidate_root,
            path: candidate_path,
            release_version: env!("CARGO_PKG_VERSION").to_owned(),
        };

        let error = LatestReleaseCandidates::default()
            .persist(root.path(), &candidate)
            .unwrap_err();
        assert!(error.to_string().contains("refusing symlink"));
        assert_eq!(fs::read(outside.path()).unwrap(), b"outside binary");
    }
}
