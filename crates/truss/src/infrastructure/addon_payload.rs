use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::application::{AddOnPayloadPort, AddOnPayloadSpec, InstallationStatePort, PortError};
use crate::domain::{
    AddOnDescriptor, AddOnName, AddOnPayloadFile, ContentHash, DomainError, RelativePath, SourceRef,
};
use crate::infrastructure::FileSystemInstallationState;

#[derive(Clone, Copy, Default)]
pub struct FileSystemAddOnPayload;

impl AddOnPayloadPort for FileSystemAddOnPayload {
    fn describe(&self, spec: &AddOnPayloadSpec<'_>) -> Result<AddOnDescriptor, PortError> {
        let name = AddOnName::parse(spec.name).map_err(domain_error)?;
        let source_ref = SourceRef::parse(spec.source_ref).map_err(domain_error)?;
        let mut files = Vec::new();
        for raw in read_manifest(spec.manifest)? {
            let path = RelativePath::parse(raw.clone()).map_err(|error| {
                PortError::new(format!(
                    "add-on payload path escapes the repository for {name}: {raw}: {error}"
                ))
            })?;
            let bytes = read_payload_file(spec.root, &name, &path)?;
            files.push(AddOnPayloadFile {
                path,
                sha256: hash_content(&bytes)?,
            });
        }
        let descriptor = AddOnDescriptor {
            name,
            source_ref,
            source_core_version: spec.source_core_version.to_owned(),
            files,
        };
        descriptor.validate().map_err(domain_error)?;
        descriptor
            .reject_owned_paths(&read_foreign_owners(spec.foreign_manifests)?)
            .map_err(domain_error)?;
        Ok(descriptor)
    }

    fn verify(
        &self,
        spec: &AddOnPayloadSpec<'_>,
        descriptor: &AddOnDescriptor,
    ) -> Result<(), PortError> {
        descriptor.validate().map_err(domain_error)?;
        let expected = self.describe(spec)?;
        if descriptor.name != expected.name {
            return Err(PortError::new(format!(
                "add-on payload identity mismatch: descriptor names {}, payload names {}",
                descriptor.name, expected.name
            )));
        }
        if descriptor.source_ref != expected.source_ref {
            return Err(PortError::new(format!(
                "add-on payload source ref mismatch for {}: descriptor {}, payload {}",
                expected.name, descriptor.source_ref, expected.source_ref
            )));
        }
        if descriptor.source_core_version != expected.source_core_version {
            return Err(PortError::new(format!(
                "add-on payload core version mismatch for {}: descriptor {}, payload {}",
                expected.name, descriptor.source_core_version, expected.source_core_version
            )));
        }
        let expected_paths = expected
            .files
            .iter()
            .map(|file| &file.path)
            .collect::<Vec<_>>();
        let actual_paths = descriptor
            .files
            .iter()
            .map(|file| &file.path)
            .collect::<Vec<_>>();
        if expected_paths != actual_paths {
            return Err(PortError::new(format!(
                "add-on payload path-set mismatch for {}: manifest path set differs from descriptor path set ({})",
                expected.name,
                first_path_difference(&expected_paths, &actual_paths)
            )));
        }
        for (expected_file, actual_file) in expected.files.iter().zip(&descriptor.files) {
            if expected_file.sha256 != actual_file.sha256 {
                return Err(PortError::new(format!(
                    "add-on payload digest mismatch for {}: {} hashes to {} in the payload but {} in the descriptor",
                    expected.name,
                    expected_file.path,
                    expected_file.sha256.as_str(),
                    actual_file.sha256.as_str()
                )));
            }
        }
        Ok(())
    }
}

/// Read one payload path declared by `descriptor` and require its bytes to
/// match the descriptor digest.
///
/// This is the smallest reader an adapter needs: path safety, regular-file,
/// and restricted-mode checks stay owned by [`read_payload_file`], and digest
/// agreement stays owned by the descriptor, so no caller re-implements payload
/// state logic.
pub(crate) fn read_declared_file(
    root: &Path,
    name: &AddOnName,
    file: &AddOnPayloadFile,
) -> Result<Vec<u8>, PortError> {
    let bytes = read_payload_file(root, name, &file.path)?;
    let actual = hash_content(&bytes)?;
    if actual != file.sha256 {
        return Err(PortError::new(format!(
            "add-on payload digest mismatch for {name}: {} hashes to {} in the payload but {} in the descriptor",
            file.path,
            actual.as_str(),
            file.sha256.as_str()
        )));
    }
    Ok(bytes)
}

fn read_manifest(manifest: &Path) -> Result<Vec<String>, PortError> {
    let content = fs::read_to_string(manifest).map_err(|error| {
        PortError::new(format!(
            "add-on manifest is not readable: {}: {error}",
            manifest.display()
        ))
    })?;
    let paths = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Err(PortError::new(format!(
            "add-on manifest declares no paths: {}",
            manifest.display()
        )));
    }
    Ok(paths)
}

fn read_foreign_owners(
    manifests: &[PathBuf],
) -> Result<Vec<(String, Vec<RelativePath>)>, PortError> {
    let mut owners = Vec::with_capacity(manifests.len());
    for manifest in manifests {
        let owner = manifest
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
            .to_owned();
        let mut paths = Vec::new();
        for raw in read_manifest(manifest)? {
            let path = RelativePath::parse(raw.clone()).map_err(|error| {
                PortError::new(format!(
                    "managed manifest for {owner} has an unsafe path: {raw}: {error}"
                ))
            })?;
            paths.push(path);
        }
        owners.push((owner, paths));
    }
    Ok(owners)
}

fn read_payload_file(
    root: &Path,
    name: &AddOnName,
    path: &RelativePath,
) -> Result<Vec<u8>, PortError> {
    let target = root.join(path.as_str());
    let metadata = match fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(PortError::new(format!(
                "add-on payload path-set mismatch for {name}: manifest path is absent from the payload: {path}"
            )));
        }
        Err(error) => {
            return Err(PortError::new(format!(
                "add-on payload file is not readable for {name}: {path}: {error}"
            )));
        }
    };
    if metadata.file_type().is_symlink() {
        return Err(PortError::new(format!(
            "add-on payload path is a symlink for {name}: {path}"
        )));
    }
    FileSystemInstallationState
        .validate_managed_path(root, path)
        .map_err(|error| {
            PortError::new(format!(
                "add-on payload path is unsafe for {name}: {path}: {error}"
            ))
        })?;
    if !metadata.is_file() {
        return Err(PortError::new(format!(
            "add-on payload entry is not a regular file for {name}: {path}"
        )));
    }
    if let Some(detail) = unsupported_mode(&metadata) {
        return Err(PortError::new(format!(
            "add-on payload file mode is unsupported for {name}: {path}: {detail}"
        )));
    }
    fs::read(&target).map_err(|error| {
        PortError::new(format!(
            "add-on payload file is not readable for {name}: {path}: {error}"
        ))
    })
}

#[cfg(unix)]
fn unsupported_mode(metadata: &fs::Metadata) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;

    let mode = metadata.permissions().mode();
    (mode & 0o111 != 0).then(|| {
        format!("mode {mode:#o} sets an executable bit; only regular non-executable files are supported")
    })
}

#[cfg(not(unix))]
fn unsupported_mode(_metadata: &fs::Metadata) -> Option<String> {
    None
}

fn first_path_difference(expected: &[&RelativePath], actual: &[&RelativePath]) -> String {
    let shared = expected.len().min(actual.len());
    for index in 0..shared {
        if expected[index] != actual[index] {
            return format!(
                "position {index}: manifest has {}, descriptor has {}",
                expected[index], actual[index]
            );
        }
    }
    if expected.len() == actual.len() {
        return "path order differs".to_owned();
    }
    if expected.len() > actual.len() {
        format!(
            "descriptor is missing {} at position {shared}",
            expected[shared]
        )
    } else {
        format!(
            "descriptor has unexpected {} at position {shared}",
            actual[shared]
        )
    }
}

fn hash_content(content: &[u8]) -> Result<ContentHash, PortError> {
    ContentHash::parse(format!("{:x}", Sha256::digest(content))).map_err(domain_error)
}

fn domain_error(error: DomainError) -> PortError {
    PortError::new(error.to_string())
}
