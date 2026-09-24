use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

use super::{BaselineFile, ContentHash, DomainError, RelativePath};

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

    /// Require a legacy install to equal this payload exactly.
    ///
    /// Every declared path must be present locally with the payload digest,
    /// and no managed local path may exist that the payload does not declare.
    /// A `None` entry means the path is absent; an entry whose digest differs
    /// from the payload digest is a consumer edit. Adoption never treats local
    /// bytes as upstream, so any disagreement is refused rather than recorded.
    pub fn require_exact_local_match(
        &self,
        local: &BTreeMap<RelativePath, Option<ContentHash>>,
        extra: &[RelativePath],
    ) -> Result<(), DomainError> {
        for file in &self.files {
            match local.get(&file.path) {
                None | Some(None) => {
                    return Err(DomainError::MissingAddOnPath(file.path.clone()));
                }
                Some(Some(actual)) if actual != &file.sha256 => {
                    return Err(DomainError::AddOnAdoptionMismatch {
                        path: file.path.clone(),
                        expected: file.sha256.clone(),
                        actual: actual.clone(),
                    });
                }
                Some(Some(_)) => {}
            }
        }
        if let Some(path) = extra.first() {
            return Err(DomainError::ExtraAddOnPath(path.clone()));
        }
        Ok(())
    }
}

/// One installed add-on: the immutable payload identity plus the baseline
/// bytes of every managed file.
///
/// The baseline bytes are the payload bytes, never the consumer workspace
/// bytes, so a consumer edit is never blessed as upstream
/// (decision `.truss-core/docs/decisions/0003-add-on-state-ownership.md`,
/// item 5).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddOnInstallation {
    pub name: AddOnName,
    pub source_ref: SourceRef,
    pub source_core_version: String,
    pub files: Vec<BaselineFile>,
}

impl AddOnInstallation {
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
}

/// Installed add-on provenance, persisted in `.truss-core/addons.json`.
///
/// This document has its own schema version. It is deliberately separate from
/// the core `.truss-core/manifest.json`, whose schema version must not move so
/// that an older binary keeps reading it (decision
/// `.truss-core/docs/decisions/0003-add-on-state-ownership.md`, item 2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddOnState {
    pub schema_version: u32,
    pub addons: Vec<AddOnInstallation>,
}

impl AddOnState {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn validate(&self) -> Result<(), DomainError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(DomainError::UnsupportedAddOnSchema(self.schema_version));
        }
        let mut names = BTreeSet::new();
        for addon in &self.addons {
            if !names.insert(addon.name.clone()) {
                return Err(DomainError::DuplicateAddOnRecord(addon.name.clone()));
            }
            addon.validate()?;
        }
        Ok(())
    }

    pub fn installation(&self, name: &AddOnName) -> Option<&AddOnInstallation> {
        self.addons.iter().find(|addon| &addon.name == name)
    }

    /// Insert or replace one add-on record, keeping the document ordered by
    /// add-on name so the persisted bytes are deterministic.
    pub fn upsert(&mut self, installation: AddOnInstallation) {
        match self
            .addons
            .iter_mut()
            .find(|addon| addon.name == installation.name)
        {
            Some(existing) => *existing = installation,
            None => self.addons.push(installation),
        }
        self.addons
            .sort_by(|left, right| left.name.cmp(&right.name));
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

    fn baseline(path: &str, content: &str, digest: &str) -> BaselineFile {
        BaselineFile {
            path: RelativePath::parse(path).unwrap(),
            content: content.as_bytes().to_vec(),
            hash: hash(digest),
        }
    }

    fn installation(files: Vec<BaselineFile>) -> AddOnInstallation {
        AddOnInstallation {
            name: AddOnName::parse("demo").unwrap(),
            source_ref: SourceRef::parse("truss-v0.1.13").unwrap(),
            source_core_version: "0.1.13".to_owned(),
            files,
        }
    }

    #[test]
    fn add_on_state_requires_its_own_schema_and_unique_records() {
        let record = installation(vec![baseline(
            ".agents/skills/demo/SKILL.md",
            "demo\n",
            "a",
        )]);
        let empty = AddOnState {
            schema_version: AddOnState::SCHEMA_VERSION,
            addons: Vec::new(),
        };
        empty.validate().unwrap();
        let wrong_schema = AddOnState {
            schema_version: AddOnState::SCHEMA_VERSION + 1,
            addons: Vec::new(),
        };
        assert!(matches!(
            wrong_schema.validate(),
            Err(DomainError::UnsupportedAddOnSchema(_))
        ));
        let duplicate = AddOnState {
            schema_version: AddOnState::SCHEMA_VERSION,
            addons: vec![record.clone(), record.clone()],
        };
        assert!(matches!(
            duplicate.validate(),
            Err(DomainError::DuplicateAddOnRecord(_))
        ));
        let mut state = empty;
        state.upsert(record.clone());
        state.upsert(record);
        assert_eq!(state.addons.len(), 1);
    }

    #[test]
    fn exact_adoption_refuses_missing_edited_and_extra_paths() {
        let record = installation(vec![
            baseline(".agents/skills/demo/SKILL.md", "demo\n", "a"),
            baseline(
                ".agents/skills/demo/agents/openai.yaml",
                "name: demo\n",
                "c",
            ),
        ]);
        let descriptor = AddOnDescriptor {
            name: record.name.clone(),
            source_ref: record.source_ref.clone(),
            source_core_version: record.source_core_version.clone(),
            files: record
                .files
                .iter()
                .map(|file| AddOnPayloadFile {
                    path: file.path.clone(),
                    sha256: file.hash.clone(),
                })
                .collect(),
        };
        let matched = record
            .files
            .iter()
            .map(|file| (file.path.clone(), Some(file.hash.clone())))
            .collect::<BTreeMap<_, _>>();
        descriptor.require_exact_local_match(&matched, &[]).unwrap();

        let mut missing = matched.clone();
        missing.insert(record.files[1].path.clone(), None);
        assert!(matches!(
            descriptor.require_exact_local_match(&missing, &[]),
            Err(DomainError::MissingAddOnPath(_))
        ));

        let mut edited = matched.clone();
        edited.insert(record.files[0].path.clone(), Some(hash("b")));
        assert!(matches!(
            descriptor.require_exact_local_match(&edited, &[]),
            Err(DomainError::AddOnAdoptionMismatch { .. })
        ));

        let extra = vec![RelativePath::parse(".agents/skills/demo/NOTES.md").unwrap()];
        assert!(matches!(
            descriptor.require_exact_local_match(&matched, &extra),
            Err(DomainError::ExtraAddOnPath(_))
        ));
    }
}
