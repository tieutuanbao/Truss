//! Persistence for the add-on conflict-session namespace.
//!
//! A staged add-on conflict session lives under
//! `<state-root>/addon-update/<name>/` (`.truss/core/addon-update/<name>/` for a
//! 0008 installation, `<state-root>/addon-update/<name>/` for a legacy one), never
//! `<state-root>/update/`, because an
//! older binary reads that path as a core session (decision
//! `.truss-core/docs/decisions/0003-add-on-state-ownership.md`, clause 7).
//!
//! Clause 12 makes the session **self-contained**: it records the immutable
//! source identity and the complete descriptor, stores the payload bytes for
//! every descriptor path, and persists the materialised plan plus every frozen
//! workspace observation. Resume takes only the workspace root and the add-on
//! name, verifies this stored material, incorporates only the operator-edited
//! resolutions, rechecks every frozen observation under the existing shared
//! lock, and applies that frozen decision without re-planning.
//!
//! Layout of one schema-2 session:
//!
//! ```text
//! <state-root>/addon-update/<name>/
//!   session.json                 schema 2: identity, ordered descriptor paths
//!                                and digests, plan digest, path lists
//!   candidate/<managed-path>     payload bytes for every descriptor path
//!   base/ local/ incoming/ resolved/   per conflict path
//!   frozen/<managed-path>        every frozen workspace observation
//!   plan.json                    changes, conflicts, and clean mutations
//! ```
//!
//! `session.json` is written last, so a partial session is never visible. The
//! plan is a decision, not a cache: resume applies it verbatim, so a later
//! binary cannot change the classification of an already-staged update. A
//! schema-1 session (which lacks the candidate payload and the plan) can be
//! inspected and aborted but never continued, and an unsupported schema fails
//! closed instead of being reinterpreted.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::addon_payload::read_declared_file;
use super::state_io::{
    copy_bytes, io_error, read_json, reject_symlink, remove_dir_if_exists, state_label,
    write_json_atomic,
};
use crate::application::PortError;
use crate::domain::{
    AddOnDescriptor, AddOnName, AddOnPayloadFile, ConflictReason, ContentHash, FileChangeKind,
    FrozenWorkspaceFile, PlannedFileChange, RelativePath, ResolutionConflict, SourceRef,
    UpdateConflict, UpdatePlan, WorkspaceMutation,
};

/// Directory under the state root that holds every add-on conflict session.
pub(crate) const ADDON_UPDATE_DIR: &str = "addon-update";
const SESSION_FILE: &str = "session.json";
const PLAN_FILE: &str = "plan.json";
const CANDIDATE_DIR: &str = "candidate";
/// The self-contained schema this revision writes.
const SESSION_SCHEMA_VERSION: u32 = 2;
/// The pre-self-contained schema. It is abortable but not continuable.
const LEGACY_SESSION_SCHEMA_VERSION: u32 = 1;
/// Per-session subdirectories, mirroring the core session layout plus the
/// candidate payload that makes the session self-contained.
const SESSION_DIRECTORIES: [&str; 6] = [
    "base",
    "local",
    "incoming",
    "resolved",
    "frozen",
    CANDIDATE_DIR,
];

pub(crate) fn addon_update_root(state_root: &Path) -> PathBuf {
    state_root.join(ADDON_UPDATE_DIR)
}

/// The owned session directory for one add-on.
pub(crate) fn addon_session_root(state_root: &Path, name: &AddOnName) -> PathBuf {
    addon_update_root(state_root).join(name.as_str())
}

/// Everything a self-contained session carries, validated on read.
///
/// `plan` is the materialised frozen plan: its `changes`, `conflicts`, and
/// clean `mutations` come from `plan.json`, its `resolution_conflicts` and
/// `frozen_files` are read from the session directories, and its `frozen_files`
/// path set is proven to equal the classified path set before the value is
/// returned.
pub(crate) struct StagedAddOnSession {
    pub(crate) from_ref: String,
    pub(crate) descriptor: AddOnDescriptor,
    pub(crate) plan: UpdatePlan,
}

/// Persist one conflicted add-on plan for `descriptor.name`.
///
/// Every conflict input and its working `resolved` copy, the candidate payload
/// bytes for every descriptor path, every frozen workspace observation, and the
/// materialised `plan.json` are written before `session.json`, so a session is
/// never visible while its material is partial. The caller already holds the
/// shared `<state-root>/lock` and has validated the pre-existing core state;
/// this function writes nothing outside the owned add-on namespace.
pub(crate) fn stage_addon_session(
    state_root: &Path,
    descriptor: &AddOnDescriptor,
    payload_root: &Path,
    from_ref: &str,
    plan: &UpdatePlan,
) -> Result<(), PortError> {
    descriptor
        .validate()
        .map_err(|error| PortError::new(error.to_string()))?;
    if plan.conflicts.is_empty() || plan.resolution_conflicts.is_empty() {
        return Err(PortError::new(
            "cannot stage an empty add-on resolution session",
        ));
    }
    validate_path_sets(descriptor, plan)?;
    let name = &descriptor.name;
    // Read and verify every candidate byte before the session directory is
    // touched, so a refused stage leaves no partial session behind.
    let mut candidate = Vec::with_capacity(descriptor.files.len());
    for file in &descriptor.files {
        candidate.push((
            file.path.clone(),
            read_declared_file(payload_root, name, file)?,
        ));
    }
    let parent = addon_update_root(state_root);
    if parent.exists() {
        reject_symlink(
            &parent,
            &format!("{}/addon-update", state_label(state_root)),
        )?;
    }
    let session_root = addon_session_root(state_root, name);
    if session_root.exists() {
        reject_symlink(
            &session_root,
            &format!("{}/addon-update/{name}", state_label(state_root)),
        )?;
    }
    remove_dir_if_exists(&session_root)?;
    fs::create_dir_all(&session_root).map_err(io_error)?;

    for conflict in &plan.resolution_conflicts {
        validate_session_path(&session_root, &conflict.path)?;
        for (directory, content) in [
            ("base", &conflict.base),
            ("local", &conflict.local),
            ("incoming", &conflict.incoming),
            ("resolved", &conflict.resolved),
        ] {
            copy_bytes(
                content,
                &session_root.join(directory).join(conflict.path.as_str()),
            )?;
        }
    }

    // The complete candidate: every descriptor path, digest-checked against the
    // descriptor before it is written into the session.
    for (path, bytes) in &candidate {
        validate_session_path(&session_root, path)?;
        copy_bytes(bytes, &session_root.join(CANDIDATE_DIR).join(path.as_str()))?;
    }

    let mut frozen_files = Vec::with_capacity(plan.frozen_files.len());
    for frozen in &plan.frozen_files {
        validate_session_path(&session_root, &frozen.path)?;
        if let Some(content) = &frozen.content {
            copy_bytes(
                content,
                &session_root.join("frozen").join(frozen.path.as_str()),
            )?;
        }
        frozen_files.push(FrozenWorkspaceFileDto {
            path: frozen.path.as_str().to_owned(),
            present: frozen.content.is_some(),
        });
    }

    // The frozen plan is a decision, not a cache: write it (and its digest)
    // before `session.json`, which is the last thing written.
    let plan_bytes = encode_plan(plan)?;
    let plan_sha256 = hex_digest(&plan_bytes);
    copy_bytes(&plan_bytes, &session_root.join(PLAN_FILE))?;

    let dto = ResolutionSessionDto {
        schema_version: SESSION_SCHEMA_VERSION,
        name: name.as_str().to_owned(),
        from_ref: from_ref.to_owned(),
        source_ref: descriptor.source_ref.as_str().to_owned(),
        source_core_version: descriptor.source_core_version.clone(),
        files: descriptor
            .files
            .iter()
            .map(|file| DescriptorFileDto {
                path: file.path.as_str().to_owned(),
                sha256: file.sha256.as_str().to_owned(),
            })
            .collect(),
        plan_sha256,
        conflicts: plan
            .resolution_conflicts
            .iter()
            .map(|conflict| ResolutionConflictDto {
                path: conflict.path.as_str().to_owned(),
            })
            .collect(),
        frozen_files,
    };
    write_json_atomic(&session_root.join(SESSION_FILE), &dto, "addon-resolution")
}

/// Read and validate the staged add-on conflict session for `name`, or `None`
/// when none is pending. The caller already holds the shared lock.
///
/// A schema-1 session refuses with a message naming abort and re-stage, and an
/// unsupported schema fails closed rather than being reinterpreted.
pub(crate) fn load_addon_session(
    state_root: &Path,
    name: &AddOnName,
) -> Result<Option<StagedAddOnSession>, PortError> {
    let session_root = addon_session_root(state_root, name);
    let session_path = session_root.join(SESSION_FILE);
    if !session_path.exists() {
        return Ok(None);
    }
    reject_symlink(
        &session_root,
        &format!("{}/addon-update/{name}", state_label(state_root)),
    )?;
    reject_symlink(
        &session_path,
        &format!(
            "{}/addon-update/{name}/session.json",
            state_label(state_root)
        ),
    )?;
    let probe: SchemaProbeDto = read_json(&session_path)?;
    match probe.schema_version {
        SESSION_SCHEMA_VERSION => {}
        LEGACY_SESSION_SCHEMA_VERSION => return Err(legacy_session_refusal(name)),
        other => return Err(unsupported_schema_refusal(name, other)),
    }
    let dto: ResolutionSessionDto = read_json(&session_path)?;
    if dto.name != name.as_str() {
        return Err(PortError::new(format!(
            "staged add-on session at {}/addon-update/{name} names {}; refusing to reinterpret it",
            state_label(state_root),
            dto.name
        )));
    }

    let mut files = Vec::with_capacity(dto.files.len());
    for file in &dto.files {
        let path = RelativePath::parse(file.path.clone())
            .map_err(|error| PortError::new(error.to_string()))?;
        let sha256 = ContentHash::parse(file.sha256.clone())
            .map_err(|error| PortError::new(error.to_string()))?;
        files.push(AddOnPayloadFile { path, sha256 });
    }
    let descriptor = AddOnDescriptor {
        name: name.clone(),
        source_ref: SourceRef::parse(dto.source_ref.clone())
            .map_err(|error| PortError::new(error.to_string()))?,
        source_core_version: dto.source_core_version.clone(),
        files,
    };
    descriptor
        .validate()
        .map_err(|error| PortError::new(error.to_string()))?;
    verify_candidate(&session_root, &descriptor)?;

    let plan_path = session_root.join(PLAN_FILE);
    reject_symlink(
        &plan_path,
        &format!("{}/addon-update/{name}/plan.json", state_label(state_root)),
    )?;
    let plan_bytes = fs::read(&plan_path).map_err(|error| {
        PortError::new(format!(
            "staged add-on session for {name} has no readable plan.json: {error}"
        ))
    })?;
    let plan_sha256 = hex_digest(&plan_bytes);
    if plan_sha256 != dto.plan_sha256 {
        return Err(PortError::new(format!(
            "staged add-on plan digest mismatch for {name}: plan.json hashes to {plan_sha256} but the session records {}",
            dto.plan_sha256
        )));
    }
    let mut plan = decode_plan(&plan_bytes)?;

    let mut resolution_conflicts = Vec::with_capacity(dto.conflicts.len());
    for item in &dto.conflicts {
        let path = RelativePath::parse(item.path.clone())
            .map_err(|error| PortError::new(error.to_string()))?;
        validate_session_path(&session_root, &path)?;
        resolution_conflicts.push(ResolutionConflict {
            base: read_session_file(&session_root, "base", &path)?,
            local: read_session_file(&session_root, "local", &path)?,
            incoming: read_session_file(&session_root, "incoming", &path)?,
            resolved: read_session_file(&session_root, "resolved", &path)?,
            path,
        });
    }

    let mut frozen_files = Vec::with_capacity(dto.frozen_files.len());
    for item in &dto.frozen_files {
        let path = RelativePath::parse(item.path.clone())
            .map_err(|error| PortError::new(error.to_string()))?;
        validate_session_path(&session_root, &path)?;
        let content = if item.present {
            Some(read_session_file(&session_root, "frozen", &path)?)
        } else {
            let target = session_root.join("frozen").join(path.as_str());
            if target.exists() {
                return Err(PortError::new(format!(
                    "absent frozen add-on path unexpectedly exists: {path}"
                )));
            }
            None
        };
        frozen_files.push(FrozenWorkspaceFile { path, content });
    }

    plan.resolution_conflicts = resolution_conflicts;
    plan.frozen_files = frozen_files;
    validate_path_sets(&descriptor, &plan)?;

    Ok(Some(StagedAddOnSession {
        from_ref: dto.from_ref,
        descriptor,
        plan,
    }))
}

/// Remove only the owned add-on session directory.
///
/// Returns `true` when a session was present. It is idempotent: a second call
/// removes nothing and returns `false`. The `<state-root>/addon-update/`
/// container is removed only when this was its last session, so another
/// add-on's staged session is never touched, and `<state-root>/update/` is never
/// read, written, or cleared. It never reads the session document, so a schema
/// 1 or unsupported session stays abortable.
pub(crate) fn clear_addon_session(state_root: &Path, name: &AddOnName) -> Result<bool, PortError> {
    let parent = addon_update_root(state_root);
    if parent.exists() {
        reject_symlink(
            &parent,
            &format!("{}/addon-update", state_label(state_root)),
        )?;
    }
    let session_root = addon_session_root(state_root, name);
    let removed = session_root.exists();
    if removed {
        reject_symlink(
            &session_root,
            &format!("{}/addon-update/{name}", state_label(state_root)),
        )?;
        remove_dir_if_exists(&session_root)?;
    }
    if parent.exists() {
        // `remove_dir` fails while another add-on session remains; that failure
        // is the guard that keeps a sibling session intact.
        let _ = fs::remove_dir(&parent);
    }
    Ok(removed)
}

fn legacy_session_refusal(name: &AddOnName) -> PortError {
    PortError::new(format!(
        "add-on conflict session for {name} uses the pre-self-contained schema 1, which stores neither the candidate payload nor the materialised plan, so it cannot be continued without re-planning. Abort it with `truss addon abort --name {name}` and re-stage the update from the immutable source."
    ))
}

fn unsupported_schema_refusal(name: &AddOnName, version: u32) -> PortError {
    PortError::new(format!(
        "unsupported add-on resolution schema {version} for {name}; refusing to reinterpret the session. Abort it with `truss addon abort --name {name}` and re-stage the update from the immutable source."
    ))
}

/// Reject any stored material that does not partition the managed paths.
///
/// The candidate path set equals the descriptor path set (checked separately),
/// the conflict path set equals the resolution-input path set, every frozen
/// observation is exactly one classified path, every descriptor path is
/// classified, and a mutation exists only for a classified change. Nothing is
/// trusted because of the directory layout.
fn validate_path_sets(descriptor: &AddOnDescriptor, plan: &UpdatePlan) -> Result<(), PortError> {
    let change_paths = collect_unique("change", plan.changes.iter().map(|item| item.path.clone()))?;
    let conflict_paths = collect_unique(
        "conflict",
        plan.conflicts.iter().map(|item| item.path.clone()),
    )?;
    let resolution_paths = collect_unique(
        "resolution",
        plan.resolution_conflicts
            .iter()
            .map(|item| item.path.clone()),
    )?;
    let mutation_paths = collect_unique(
        "mutation",
        plan.mutations.iter().map(|item| item.path().clone()),
    )?;
    let frozen_paths = collect_unique(
        "frozen",
        plan.frozen_files.iter().map(|item| item.path.clone()),
    )?;
    let descriptor_paths = collect_unique(
        "descriptor",
        descriptor.files.iter().map(|file| file.path.clone()),
    )?;

    if conflict_paths != resolution_paths {
        return Err(PortError::new(format!(
            "staged add-on plan for {} conflicts with its resolution inputs: {} conflict path(s) vs {} resolution path(s)",
            descriptor.name,
            conflict_paths.len(),
            resolution_paths.len()
        )));
    }
    if !change_paths.is_disjoint(&conflict_paths) {
        return Err(PortError::new(format!(
            "staged add-on plan for {} classifies one path as both a change and a conflict",
            descriptor.name
        )));
    }
    let classified = change_paths
        .union(&conflict_paths)
        .cloned()
        .collect::<BTreeSet<_>>();
    if frozen_paths != classified {
        return Err(PortError::new(format!(
            "staged add-on plan for {} does not freeze exactly its classified paths: {} frozen vs {} classified",
            descriptor.name,
            frozen_paths.len(),
            classified.len()
        )));
    }
    if !descriptor_paths.is_subset(&classified) {
        let missing = descriptor_paths
            .difference(&classified)
            .next()
            .expect("a strict subset has a difference");
        return Err(PortError::new(format!(
            "staged add-on plan for {} omits descriptor path {missing}",
            descriptor.name
        )));
    }
    if !mutation_paths.is_subset(&change_paths) {
        let unclassified = mutation_paths
            .difference(&change_paths)
            .next()
            .expect("a strict subset has a difference");
        return Err(PortError::new(format!(
            "staged add-on plan for {} stages a mutation for unclassified path {unclassified}",
            descriptor.name
        )));
    }
    Ok(())
}

/// Verify the stored candidate against the recorded descriptor: the candidate
/// path set must equal the descriptor path set and every stored byte must hash
/// to its recorded digest. Symlinks, non-regular entries, duplicates, and
/// escapes are refused.
fn verify_candidate(session_root: &Path, descriptor: &AddOnDescriptor) -> Result<(), PortError> {
    let candidate_root = session_root.join(CANDIDATE_DIR);
    if !candidate_root.exists() {
        return Err(PortError::new(format!(
            "staged add-on session for {} has no candidate payload",
            descriptor.name
        )));
    }
    reject_symlink(
        &candidate_root,
        &format!("addon-update/{}/candidate", descriptor.name),
    )?;
    let found = collect_candidate_paths(&candidate_root)?;
    let declared = descriptor
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    if found != declared {
        let detail = found
            .difference(&declared)
            .next()
            .map(|path| format!("unexpected {path}"))
            .or_else(|| {
                declared
                    .difference(&found)
                    .next()
                    .map(|path| format!("missing {path}"))
            })
            .unwrap_or_else(|| "path order differs".to_owned());
        return Err(PortError::new(format!(
            "staged add-on candidate path set differs from the recorded descriptor for {}: {detail}",
            descriptor.name
        )));
    }
    for file in &descriptor.files {
        validate_session_path(session_root, &file.path)?;
        let bytes = read_session_file(session_root, CANDIDATE_DIR, &file.path)?;
        let actual = hex_digest(&bytes);
        if actual != file.sha256.as_str() {
            return Err(PortError::new(format!(
                "staged add-on candidate digest mismatch for {}: {} hashes to {actual} in the session but {} in the recorded descriptor",
                descriptor.name,
                file.path,
                file.sha256.as_str()
            )));
        }
    }
    Ok(())
}

fn collect_unique<I>(label: &str, paths: I) -> Result<BTreeSet<RelativePath>, PortError>
where
    I: IntoIterator<Item = RelativePath>,
{
    let mut set = BTreeSet::new();
    for path in paths {
        if !set.insert(path.clone()) {
            return Err(PortError::new(format!(
                "staged add-on material repeats a {label} path: {path}"
            )));
        }
    }
    Ok(set)
}

/// Enumerate every regular file below the candidate root as a relative path.
///
/// A symlink, a non-regular entry, a duplicate, or a path that cannot be a
/// managed relative path is a refusal: the candidate payload is a managed path
/// set, not an arbitrary tree.
fn collect_candidate_paths(candidate_root: &Path) -> Result<BTreeSet<RelativePath>, PortError> {
    let mut found = BTreeSet::new();
    walk_candidate(candidate_root, candidate_root, &mut found)?;
    Ok(found)
}

fn walk_candidate(
    candidate_root: &Path,
    directory: &Path,
    found: &mut BTreeSet<RelativePath>,
) -> Result<(), PortError> {
    let mut entries = fs::read_dir(directory)
        .map_err(io_error)?
        .map(|entry| entry.map_err(io_error))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(PortError::new(format!(
                "refusing symlink in the staged candidate payload: {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            walk_candidate(candidate_root, &path, found)?;
            continue;
        }
        if !metadata.is_file() {
            return Err(PortError::new(format!(
                "staged candidate entry is not a regular file: {}",
                path.display()
            )));
        }
        let relative = path
            .strip_prefix(candidate_root)
            .map_err(|_| PortError::new("staged candidate path escaped the session"))?
            .to_string_lossy()
            .replace('\\', "/");
        let parsed = RelativePath::parse(relative.clone()).map_err(|error| {
            PortError::new(format!(
                "staged candidate path is unsafe: {relative}: {error}"
            ))
        })?;
        if !found.insert(parsed) {
            return Err(PortError::new(format!(
                "staged candidate payload repeats a path: {relative}"
            )));
        }
    }
    Ok(())
}

fn read_session_file(
    session_root: &Path,
    directory: &str,
    path: &RelativePath,
) -> Result<Vec<u8>, PortError> {
    let target = session_root.join(directory).join(path.as_str());
    let metadata = fs::symlink_metadata(&target).map_err(|error| {
        PortError::new(format!(
            "could not read {directory} input for {path}: {error}"
        ))
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(PortError::new(format!(
            "session input is not a regular file: {}",
            target.display()
        )));
    }
    fs::read(target).map_err(io_error)
}

fn validate_session_path(session_root: &Path, path: &RelativePath) -> Result<(), PortError> {
    for directory in SESSION_DIRECTORIES {
        let mut current = session_root.join(directory);
        if current.exists() {
            reject_symlink(&current, directory)?;
        }
        for component in path.as_str().split('/') {
            current.push(component);
            if current.exists() {
                reject_symlink(&current, path.as_str())?;
            }
        }
    }
    Ok(())
}

fn encode_plan(plan: &UpdatePlan) -> Result<Vec<u8>, PortError> {
    let dto = PlanDto {
        changes: plan
            .changes
            .iter()
            .map(|change| ChangeDto {
                path: change.path.as_str().to_owned(),
                kind: kind_str(&change.kind).to_owned(),
            })
            .collect(),
        conflicts: plan
            .conflicts
            .iter()
            .map(|conflict| ConflictDto {
                path: conflict.path.as_str().to_owned(),
                reason: reason_str(&conflict.reason).to_owned(),
                detail: conflict.detail.clone(),
            })
            .collect(),
        mutations: plan
            .mutations
            .iter()
            .map(|mutation| match mutation {
                WorkspaceMutation::Write { path, content } => MutationDto::Write {
                    path: path.as_str().to_owned(),
                    content_hex: encode_hex(content),
                },
                WorkspaceMutation::Delete { path } => MutationDto::Delete {
                    path: path.as_str().to_owned(),
                },
            })
            .collect(),
    };
    serde_json::to_vec_pretty(&dto)
        .map_err(|error| PortError::new(format!("could not encode the add-on plan: {error}")))
}

fn decode_plan(bytes: &[u8]) -> Result<UpdatePlan, PortError> {
    let dto: PlanDto = serde_json::from_slice(bytes).map_err(|error| {
        PortError::new(format!("could not parse the staged add-on plan: {error}"))
    })?;
    let mut plan = UpdatePlan::default();
    for change in dto.changes {
        plan.changes.push(PlannedFileChange {
            path: RelativePath::parse(change.path)
                .map_err(|error| PortError::new(error.to_string()))?,
            kind: parse_kind(&change.kind)?,
        });
    }
    for conflict in dto.conflicts {
        plan.conflicts.push(UpdateConflict {
            path: RelativePath::parse(conflict.path)
                .map_err(|error| PortError::new(error.to_string()))?,
            reason: parse_reason(&conflict.reason)?,
            detail: conflict.detail,
        });
    }
    for mutation in dto.mutations {
        plan.mutations.push(match mutation {
            MutationDto::Write { path, content_hex } => WorkspaceMutation::Write {
                path: RelativePath::parse(path)
                    .map_err(|error| PortError::new(error.to_string()))?,
                content: decode_hex(&content_hex)?,
            },
            MutationDto::Delete { path } => WorkspaceMutation::Delete {
                path: RelativePath::parse(path)
                    .map_err(|error| PortError::new(error.to_string()))?,
            },
        });
    }
    Ok(plan)
}

fn kind_str(kind: &FileChangeKind) -> &'static str {
    match kind {
        FileChangeKind::Create => "create",
        FileChangeKind::Update => "update",
        FileChangeKind::Delete => "delete",
        FileChangeKind::Preserve => "preserve",
        FileChangeKind::Adopt => "adopt",
    }
}

fn parse_kind(value: &str) -> Result<FileChangeKind, PortError> {
    Ok(match value {
        "create" => FileChangeKind::Create,
        "update" => FileChangeKind::Update,
        "delete" => FileChangeKind::Delete,
        "preserve" => FileChangeKind::Preserve,
        "adopt" => FileChangeKind::Adopt,
        other => {
            return Err(PortError::new(format!(
                "staged add-on plan carries an unknown change kind: {other}"
            )))
        }
    })
}

fn reason_str(reason: &ConflictReason) -> &'static str {
    match reason {
        ConflictReason::OverlappingChanges => "overlapping_changes",
        ConflictReason::MissingManagedFile => "missing_managed_file",
        ConflictReason::ExistingUnmanagedPath => "existing_unmanaged_path",
        ConflictReason::ModifiedRemovedFile => "modified_removed_file",
        ConflictReason::UnsafePath => "unsafe_path",
    }
}

fn parse_reason(value: &str) -> Result<ConflictReason, PortError> {
    Ok(match value {
        "overlapping_changes" => ConflictReason::OverlappingChanges,
        "missing_managed_file" => ConflictReason::MissingManagedFile,
        "existing_unmanaged_path" => ConflictReason::ExistingUnmanagedPath,
        "modified_removed_file" => ConflictReason::ModifiedRemovedFile,
        "unsafe_path" => ConflictReason::UnsafePath,
        other => {
            return Err(PortError::new(format!(
                "staged add-on plan carries an unknown conflict reason: {other}"
            )))
        }
    })
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from_digit((byte >> 4) as u32, 16).expect("nibble is a digit"));
        encoded.push(char::from_digit((byte & 0x0f) as u32, 16).expect("nibble is a digit"));
    }
    encoded
}

fn decode_hex(value: &str) -> Result<Vec<u8>, PortError> {
    if !value.len().is_multiple_of(2) {
        return Err(PortError::new(
            "staged add-on mutation content is not valid hexadecimal",
        ));
    }
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks(2) {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        decoded.push((high << 4) | low);
    }
    Ok(decoded)
}

fn hex_nibble(byte: u8) -> Result<u8, PortError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(PortError::new(
            "staged add-on mutation content is not valid hexadecimal",
        )),
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Deserialize)]
struct SchemaProbeDto {
    schema_version: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResolutionSessionDto {
    schema_version: u32,
    name: String,
    from_ref: String,
    source_ref: String,
    source_core_version: String,
    files: Vec<DescriptorFileDto>,
    plan_sha256: String,
    conflicts: Vec<ResolutionConflictDto>,
    frozen_files: Vec<FrozenWorkspaceFileDto>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DescriptorFileDto {
    path: String,
    sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResolutionConflictDto {
    path: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct FrozenWorkspaceFileDto {
    path: String,
    present: bool,
}

/// The frozen decision. Resolution inputs and frozen observations are excluded
/// on purpose: their bytes live in the session directories, and this document
/// records the classification resume applies verbatim.
#[derive(Debug, Deserialize, Serialize)]
struct PlanDto {
    changes: Vec<ChangeDto>,
    conflicts: Vec<ConflictDto>,
    mutations: Vec<MutationDto>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChangeDto {
    path: String,
    kind: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ConflictDto {
    path: String,
    reason: String,
    detail: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum MutationDto {
    Write { path: String, content_hex: String },
    Delete { path: String },
}
