use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

use super::{ContentHash, DomainError, RelativePath};

/// Name of an optional add-on distribution.
///
/// The name is used as a directory component under the installed-state root,
/// so it must be a single safe path segment.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AddOnName(String);

impl AddOnName {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let invalid = value.is_empty()
            || value.trim() != value
            || value == "."
            || value == ".."
            || value.contains('/')
            || value.contains('\\')
            || value.contains('\0')
            || value.contains(':');
        if invalid {
            return Err(DomainError::InvalidAddOnName(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for AddOnName {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Immutable source identity of an add-on payload.
///
/// A released payload records the exact `truss-vX.Y.Z` tag; a git payload
/// records the exact commit SHA. A branch name, `HEAD`, or a short SHA can
/// move, so none of them is an accepted source ref
/// (decision `.truss-core/docs/decisions/0003-add-on-state-ownership.md`, item 6).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourceRef(String);

impl SourceRef {
    pub fn parse(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let release_tag = value
            .strip_prefix("truss-v")
            .is_some_and(|version| semver::Version::parse(version).is_ok());
        let commit = (value.len() == 40 || value.len() == 64)
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if !release_tag && !commit {
            return Err(DomainError::MutableSourceRef(value));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_commit(&self) -> bool {
        !self.0.starts_with("truss-v")
    }
}

impl Display for SourceRef {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// One payload file: its repository-relative path and the SHA-256 of its
/// payload bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddOnPayloadFile {
    pub path: RelativePath,
    pub sha256: ContentHash,
}

/// Immutable description of one add-on payload.
///
/// `files` keeps the membership order of the manifest it was read from; the
/// manifest remains the membership owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddOnDescriptor {
    pub name: AddOnName,
    pub source_ref: SourceRef,
    pub source_core_version: String,
    pub files: Vec<AddOnPayloadFile>,
}

impl AddOnDescriptor {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.source_core_version.trim().is_empty() {
            return Err(DomainError::EmptyVersion);
        }
        if self.files.is_empty() {
            return Err(DomainError::EmptyAddOnPayload(self.name.clone()));
        }
        let mut paths = BTreeSet::new();
        for file in &self.files {
            if !paths.insert(file.path.clone()) {
                return Err(DomainError::DuplicatePath(file.path.clone()));
            }
        }
        Ok(())
    }

    /// Reject a payload path already owned by the core distribution or by
    /// another add-on. `owners` pairs an owner name with that owner's
    /// manifest paths.
    pub fn reject_owned_paths(
        &self,
        owners: &[(String, Vec<RelativePath>)],
    ) -> Result<(), DomainError> {
        for file in &self.files {
            for (owner, paths) in owners {
                if paths.contains(&file.path) {
                    return Err(DomainError::OverlappingDistributionPath {
                        owner: owner.clone(),
                        path: file.path.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(value: &str) -> ContentHash {
        ContentHash::parse(value.repeat(64)).unwrap()
    }

    #[test]
    fn source_refs_accept_only_release_tags_and_exact_commit_shas() {
        assert_eq!(
            SourceRef::parse("truss-v0.1.13").unwrap().as_str(),
            "truss-v0.1.13"
        );
        assert!(SourceRef::parse("b11d79ad2310890713ab281e6d0a8a57e449bef6")
            .unwrap()
            .is_commit());
        assert_eq!(
            SourceRef::parse("a".repeat(64)).unwrap().as_str(),
            "a".repeat(64)
        );
        for mutable in [
            "main",
            "master",
            "HEAD",
            "develop",
            "truss-v0.1",
            "v0.1.13",
            "b11d79a",
            "b11d79ad2310890713ab281e6d0a8a57e449befg",
            "",
        ] {
            assert!(
                matches!(
                    SourceRef::parse(mutable),
                    Err(DomainError::MutableSourceRef(_))
                ),
                "accepted mutable ref {mutable}"
            );
        }
    }

    #[test]
    fn add_on_names_reject_path_segments() {
        assert_eq!(AddOnName::parse("delivery").unwrap().as_str(), "delivery");
        for invalid in ["", ".", "..", "a/b", "a\\b", "a:b", " delivery", "a\0b"] {
            assert!(
                matches!(
                    AddOnName::parse(invalid),
                    Err(DomainError::InvalidAddOnName(_))
                ),
                "accepted add-on name {invalid:?}"
            );
        }
    }

    #[test]
    fn descriptor_requires_version_files_and_unique_paths() {
        let name = AddOnName::parse("delivery").unwrap();
        let source_ref = SourceRef::parse("truss-v0.1.13").unwrap();
        let file = AddOnPayloadFile {
            path: RelativePath::parse(".agents/skills/delivery/SKILL.md").unwrap(),
            sha256: hash("a"),
        };
        let empty = AddOnDescriptor {
            name: name.clone(),
            source_ref: source_ref.clone(),
            source_core_version: "0.1.13".to_owned(),
            files: Vec::new(),
        };
        assert!(matches!(
            empty.validate(),
            Err(DomainError::EmptyAddOnPayload(_))
        ));
        let duplicate = AddOnDescriptor {
            name,
            source_ref,
            source_core_version: "0.1.13".to_owned(),
            files: vec![file.clone(), file],
        };
        assert!(matches!(
            duplicate.validate(),
            Err(DomainError::DuplicatePath(_))
        ));
    }

    #[test]
    fn descriptor_rejects_paths_owned_by_another_distribution() {
        let upload = AddOnPayloadFile {
            path: RelativePath::parse(".agents/skills/delivery/SKILL.md").unwrap(),
            sha256: hash("a"),
        };
        let descriptor = AddOnDescriptor {
            name: AddOnName::parse("delivery").unwrap(),
            source_ref: SourceRef::parse("truss-v0.1.13").unwrap(),
            source_core_version: "0.1.13".to_owned(),
            files: vec![upload],
        };
        let owners = vec![(
            "delivery-setup".to_owned(),
            vec![RelativePath::parse(".agents/skills/delivery/SKILL.md").unwrap()],
        )];
        assert!(matches!(
            descriptor.reject_owned_paths(&owners),
            Err(DomainError::OverlappingDistributionPath { .. })
        ));
        let unrelated = vec![(
            "core".to_owned(),
            vec![RelativePath::parse("AGENTS.md").unwrap()],
        )];
        descriptor.reject_owned_paths(&unrelated).unwrap();
    }
}
