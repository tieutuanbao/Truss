use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use fs2::FileExt;
use serde::{Deserialize, Serialize};

use super::state_io::{
    acquire_existing_lock, copy_bytes, copy_bytes_atomic, domain_error, ensure_workspace_root,
    hash_bytes, io_error, read_json, reject_symlink, remove_dir_if_exists,
    resolve_state_root as resolve_root, state_root, transaction_id, validate_core_state,
    validate_path, validate_state_path, validate_workspace_root, write_json_atomic,
};
use super::FileSystemInstallationState;
use crate::application::{
    AddOnInstallRequest, AddOnRecordReceipt, AddOnStatePort, InstallationStatePort, PortError,
};
use crate::domain::{
    AddOnDescriptor, AddOnInstallation, AddOnName, AddOnState, BaselineFile, ContentHash,
    DomainError, RelativePath, SourceRef,
};

pub(crate) const ADDONS_FILE: &str = "addons.json";
pub(crate) const BASE_ADDONS_DIR: &str = "base-addons";
/// Owner label the core installation's manifest entries are reported under.
pub(crate) const CORE_OWNER: &str = "truss-core";

/// Filesystem implementation of the installed add-on record.
///
/// State lives in `.truss-core/addons.json` with its own schema version, and
/// each baseline copy lives under `.truss-core/base-addons/<add-on>/`. The
/// writer shares the core lock, the atomic write style, and the `.truss-core`
/// root so there is one writer per repository, not one per distribution.
#[derive(Clone, Copy, Default)]
pub struct FileSystemAddOnState;

impl AddOnStatePort for FileSystemAddOnState {
    fn resolve_state_root(&self, root: &Path) -> Result<PathBuf, PortError> {
        resolve_root(root)
    }

    fn load(&self, root: &Path) -> Result<Option<AddOnState>, PortError> {
        if !root.exists() {
            return Ok(None);
        }
        validate_workspace_root(root)?;
        let state_root = state_root(root);
        if !state_root.exists() {
            return Ok(None);
        }
        reject_symlink(&state_root, ".truss-core")?;
        load_state(&state_root)
    }

    fn recorded_owners(
        &self,
        root: &Path,
        own_name: &AddOnName,
    ) -> Result<Vec<(String, Vec<RelativePath>)>, PortError> {
        validate_workspace_root(root)?;
        foreign_owners(root, &state_root(root), own_name)
    }

    fn apply(
        &self,
        root: &Path,
        request: &AddOnInstallRequest<'_>,
    ) -> Result<AddOnRecordReceipt, PortError> {
        ensure_workspace_root(root)?;
        request.descriptor.validate().map_err(domain_error)?;
        let state_root = state_root(root);
        // Core install and core update exclusively own `.truss-core/`, its
        // `.gitignore`, and its `lock`. This read-only prerequisite check runs
        // before any payload or workspace observation and refuses without
        // creating or repairing anything.
        validate_core_state(&state_root)?;
        // Fast-fail only: the locked observation below re-derives the verdict
        // and is the only one that authorizes `commit`.
        preflight(root, request.descriptor)?;
        addon_observation_witness::run_barrier(root);
        let lock = acquire_existing_lock(&state_root)?;
        // State loading, workspace observation, and commit run under the
        // existing shared lock, and `commit` consumes exactly this locked
        // observation, so no other writer can change a managed path between
        // observation and write.
        let observed = locked_apply(root, &state_root, request);
        FileExt::unlock(&lock).map_err(io_error)?;
        let plan = observed?;
        Ok(AddOnRecordReceipt {
            adopted: plan.adopted,
        })
    }
}

struct AddOnPlan {
    installation: AddOnInstallation,
    state: AddOnState,
    adopted: bool,
    fresh: bool,
}

/// Fast-fail refusal before the shared lock is taken.
///
/// The locked observation re-derives this verdict and is the only one that
/// authorizes a write, so a race here can only make the locked observation
/// refuse more. The preflight exists so an obviously refused request does not
/// take the lock at all; it never creates or repairs core state, and it never
/// authorizes a mutation.
fn preflight(root: &Path, descriptor: &AddOnDescriptor) -> Result<(), PortError> {
    assess_local(root, descriptor)?.adopted(descriptor)?;
    Ok(())
}

/// The consumer side of an observation: which managed paths are present,
/// which are edited, and which extra paths the payload does not declare.
struct LocalAssessment {
    local: BTreeMap<RelativePath, Option<ContentHash>>,
    extra: Vec<RelativePath>,
    present: usize,
}

impl LocalAssessment {
    /// `Ok(false)` is a fresh install, `Ok(true)` is an exact adoption, and an
    /// error is a refusal. Payload bytes are not needed for this verdict.
    fn adopted(&self, descriptor: &AddOnDescriptor) -> Result<bool, PortError> {
        if self.present == 0 {
            if let Some(path) = self.extra.first() {
                return Err(domain_error(DomainError::ExtraAddOnPath(path.clone())));
            }
            return Ok(false);
        }
        descriptor
            .require_exact_local_match(&self.local, &self.extra)
            .map_err(domain_error)?;
        Ok(true)
    }
}

fn assess_local(root: &Path, descriptor: &AddOnDescriptor) -> Result<LocalAssessment, PortError> {
    let extra = collect_extra_managed_paths(root, descriptor)?;
    let mut local = BTreeMap::new();
    let mut present = 0usize;
    for file in &descriptor.files {
        validate_path(root, &file.path)?;
        let target = root.join(file.path.as_str());
        match fs::symlink_metadata(&target) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(PortError::new(format!(
                        "refusing symlink for add-on managed path: {}",
                        file.path
                    )));
                }
                if !metadata.is_file() {
                    return Err(PortError::new(format!(
                        "add-on managed path is not a regular file: {}",
                        file.path
                    )));
                }
                let bytes = fs::read(&target).map_err(io_error)?;
                local.insert(file.path.clone(), Some(hash_bytes(&bytes)?));
                present += 1;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                local.insert(file.path.clone(), None);
            }
            Err(error) => return Err(io_error(error)),
        }
    }
    Ok(LocalAssessment {
        local,
        extra,
        present,
    })
}

/// Load state, observe the workspace, and commit, all under the shared lock
/// the caller already holds.
fn locked_apply(
    root: &Path,
    state_root: &Path,
    request: &AddOnInstallRequest<'_>,
) -> Result<AddOnPlan, PortError> {
    let existing = load_state(state_root)?;
    let plan = observe(root, request, existing)?;
    commit(root, state_root, &plan)?;
    Ok(plan)
}

fn observe(
    root: &Path,
    request: &AddOnInstallRequest<'_>,
    existing: Option<AddOnState>,
) -> Result<AddOnPlan, PortError> {
    // Runs at the entry of the observation that authorizes `commit`, which is
    // always entered while the shared lock is held.
    addon_observation_witness::run(root)?;
    let descriptor = request.descriptor;
    let name = descriptor.name.clone();

    // The baseline is built from the staged payload bytes, never from the
    // target workspace, so a consumer edit is never recorded as upstream.
    let mut files = Vec::with_capacity(descriptor.files.len());
    for file in &descriptor.files {
        let payload = fs::read(request.payload_root.join(file.path.as_str())).map_err(|error| {
            PortError::new(format!(
                "add-on payload file is not readable for {name}: {}: {error}",
                file.path
            ))
        })?;
        let digest = hash_bytes(&payload)?;
        if digest != file.sha256 {
            return Err(PortError::new(format!(
                "add-on payload digest mismatch for {name}: {} hashes to {} in the payload but {} in the descriptor",
                file.path,
                digest.as_str(),
                file.sha256.as_str()
            )));
        }
        files.push(BaselineFile {
            path: file.path.clone(),
            content: payload,
            hash: file.sha256.clone(),
        });
    }
    let installation = AddOnInstallation {
        name: name.clone(),
        source_ref: descriptor.source_ref.clone(),
        source_core_version: descriptor.source_core_version.clone(),
        files,
    };

    if existing
        .as_ref()
        .is_some_and(|state| state.installation(&name).is_some())
    {
        return Err(PortError::new(format!(
            "add-on {name} is already recorded; this slice does not implement add-on update"
        )));
    }

    let assessment = assess_local(root, descriptor)?;
    let adopted = assessment.adopted(descriptor)?;

    let mut state = existing.unwrap_or(AddOnState {
        schema_version: AddOnState::SCHEMA_VERSION,
        addons: Vec::new(),
    });
    state.upsert(installation.clone());
    state.validate().map_err(domain_error)?;
    Ok(AddOnPlan {
        installation,
        state,
        adopted,
        fresh: assessment.present == 0,
    })
}

fn commit(root: &Path, state_root: &Path, plan: &AddOnPlan) -> Result<(), PortError> {
    let id = transaction_id()?;
    if plan.fresh {
        for file in &plan.installation.files {
            copy_bytes_atomic(&file.content, &root.join(file.path.as_str()), &id)?;
        }
    }
    write_provenance(state_root, &plan.installation, &plan.state, &id)
}

/// Write the baseline copies first and the record last, mirroring the core's
/// files-first, provenance-last ordering.
fn write_provenance(
    state_root: &Path,
    installation: &AddOnInstallation,
    state: &AddOnState,
    id: &str,
) -> Result<(), PortError> {
    publish_baseline(state_root, installation, id)?;
    write_addons_record(state_root, state, id)
}

/// Publish `.truss-core/base-addons/<name>/` from the payload bytes and only
/// then make it visible, so the baseline never reflects a partial payload.
pub(crate) fn publish_baseline(
    state_root: &Path,
    installation: &AddOnInstallation,
    id: &str,
) -> Result<(), PortError> {
    let base_root = base_addons_root(state_root);
    let next = base_root.join(format!("{}.next-{id}", installation.name));
    remove_dir_if_exists(&next)?;
    for file in &installation.files {
        let actual = hash_bytes(&file.content)?;
        if actual != file.hash {
            return Err(PortError::new(format!(
                "provided add-on baseline hash differs for {}",
                file.path
            )));
        }
        copy_bytes(&file.content, &next.join(file.path.as_str()))?;
    }
    let published = base_root.join(installation.name.as_str());
    remove_dir_if_exists(&published)?;
    fs::rename(&next, &published).map_err(io_error)
}

/// Write `.truss-core/addons.json` last, after the workspace and the baseline
/// are already in place.
pub(crate) fn write_addons_record(
    state_root: &Path,
    state: &AddOnState,
    id: &str,
) -> Result<(), PortError> {
    write_json_atomic(&state_root.join(ADDONS_FILE), &state_to_dto(state), id)
}

fn load_state(state_root: &Path) -> Result<Option<AddOnState>, PortError> {
    let record_path = state_root.join(ADDONS_FILE);
    if !record_path.exists() {
        return Ok(None);
    }
    reject_symlink(&record_path, ".truss-core/addons.json")?;
    let dto: AddOnsDto = read_json(&record_path)?;
    if dto.schema_version != AddOnState::SCHEMA_VERSION {
        return Err(PortError::new(format!(
            "unsupported add-on state schema: {}",
            dto.schema_version
        )));
    }
    let mut addons = Vec::new();
    for addon in dto.addons {
        let name = AddOnName::parse(addon.name).map_err(domain_error)?;
        let source_ref = SourceRef::parse(addon.source_ref).map_err(domain_error)?;
        let mut files = Vec::new();
        for file in addon.files {
            let path = RelativePath::parse(file.path).map_err(|error| {
                PortError::new(format!("add-on record path is unsafe: {error}"))
            })?;
            let expected = ContentHash::parse(file.upstream_sha256).map_err(domain_error)?;
            let baseline = base_addons_root(state_root)
                .join(name.as_str())
                .join(path.as_str());
            validate_state_path(state_root, &baseline)?;
            let content = fs::read(&baseline).map_err(|error| {
                PortError::new(format!(
                    "could not read add-on baseline {}/{}: {error}",
                    name.as_str(),
                    path.as_str()
                ))
            })?;
            let actual = hash_bytes(&content)?;
            if actual != expected {
                return Err(PortError::new(format!(
                    "add-on base hash mismatch for {}/{}: expected {}, got {}",
                    name.as_str(),
                    path.as_str(),
                    expected.as_str(),
                    actual.as_str()
                )));
            }
            files.push(BaselineFile {
                path,
                content,
                hash: expected,
            });
        }
        addons.push(AddOnInstallation {
            name,
            source_ref,
            source_core_version: addon.source_core_version,
            files,
        });
    }
    let state = AddOnState {
        schema_version: dto.schema_version,
        addons,
    };
    state.validate().map_err(domain_error)?;
    Ok(Some(state))
}

/// A managed local path the payload does not declare stops adoption.
///
/// Decision 0003 clause 13: the extra-path validation runs only inside the
/// add-on's scan roots. A declared directory is a scan root when it is not an
/// ancestor-or-equal of any path owned by another owner, where ownership means
/// the core `.truss-core/manifest.json` entry list or another add-on's recorded
/// paths in `.truss-core/addons.json`. A shared ancestor is demoted, so the
/// content of another owner under it is never an extra path for this add-on,
/// while the hazard check still covers the add-on's own subtree.
fn collect_extra_managed_paths(
    root: &Path,
    descriptor: &AddOnDescriptor,
) -> Result<Vec<RelativePath>, PortError> {
    let state_root = state_root(root);
    let foreign = foreign_owners(root, &state_root, &descriptor.name)?
        .into_iter()
        .flat_map(|(_, paths)| paths)
        .collect::<Vec<_>>();
    let mut declared_paths = BTreeSet::new();
    let mut declared_dirs = BTreeSet::new();
    for file in &descriptor.files {
        declared_paths.insert(file.path.as_str().to_owned());
        let mut parts = file.path.as_str().split('/').collect::<Vec<_>>();
        parts.pop();
        let mut prefix = String::new();
        for part in parts {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            declared_dirs.insert(prefix.clone());
        }
    }
    let eligible = declared_dirs
        .iter()
        .filter(|directory| {
            !foreign
                .iter()
                .any(|owned| is_ancestor_or_equal(directory, owned.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();
    let roots = eligible
        .iter()
        .filter(|directory| {
            !eligible.iter().any(|other| {
                other.len() < directory.len()
                    && directory.starts_with(other.as_str())
                    && directory.as_bytes().get(other.len()) == Some(&b'/')
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut extra = Vec::new();
    for directory in roots {
        walk_managed_dir(
            root,
            &directory,
            &declared_paths,
            &declared_dirs,
            &mut extra,
        )?;
    }
    extra.sort();
    extra.dedup();
    Ok(extra)
}

/// `directory` is `path` itself or a real ancestor of `path`.
fn is_ancestor_or_equal(directory: &str, path: &str) -> bool {
    path == directory
        || (path.len() > directory.len()
            && path.starts_with(directory)
            && path.as_bytes().get(directory.len()) == Some(&b'/'))
}

/// Every path owned by an owner other than `own_name`, paired with the owner's
/// name: the core installation state's manifest entries under [`CORE_OWNER`],
/// and the recorded paths of every other recorded add-on.
///
/// The core set is read through the same state reader the rest of the code
/// uses, so a missing `.truss-core/manifest.json` is a refusal and the foreign
/// set is never silently empty. The other add-ons come from the recorded paths
/// in `.truss-core/addons.json`, and this add-on's own record is not foreign.
///
/// This is the one reader of the recorded ownership set: the extra-path
/// scan-root rule flattens it, and the add-on state port reports it so the
/// application can reject a descriptor that collides with it.
fn foreign_owners(
    root: &Path,
    state_root: &Path,
    own_name: &AddOnName,
) -> Result<Vec<(String, Vec<RelativePath>)>, PortError> {
    let Some(core) = FileSystemInstallationState.load(root)? else {
        return Err(PortError::new(
            "the core installation state has no .truss-core/manifest.json; add-on operations refuse"
                .to_owned(),
        ));
    };
    let mut owners = vec![(
        CORE_OWNER.to_owned(),
        core.files
            .into_iter()
            .map(|file| file.path)
            .collect::<Vec<_>>(),
    )];
    if let Some(state) = load_state(state_root)? {
        for installation in state.addons {
            if &installation.name != own_name {
                owners.push((
                    installation.name.as_str().to_owned(),
                    installation
                        .files
                        .into_iter()
                        .map(|file| file.path)
                        .collect(),
                ));
            }
        }
    }
    Ok(owners)
}

fn walk_managed_dir(
    root: &Path,
    relative_dir: &str,
    declared_paths: &BTreeSet<String>,
    declared_dirs: &BTreeSet<String>,
    extra: &mut Vec<RelativePath>,
) -> Result<(), PortError> {
    let directory = root.join(relative_dir);
    match fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io_error(error)),
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(PortError::new(format!(
                "refusing symlink in add-on managed path: {relative_dir}"
            )));
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err(PortError::new(format!(
                "add-on managed path is not a directory: {relative_dir}"
            )));
        }
        Ok(_) => {}
    }
    let mut entries = fs::read_dir(&directory)
        .map_err(io_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(io_error)?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{relative_dir}/{file_name}");
        let metadata = fs::symlink_metadata(entry.path()).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(PortError::new(format!(
                "refusing symlink in add-on managed path: {relative}"
            )));
        }
        if metadata.is_dir() {
            if declared_dirs.contains(&relative) {
                walk_managed_dir(root, &relative, declared_paths, declared_dirs, extra)?;
            } else {
                extra.push(RelativePath::parse(relative).map_err(domain_error)?);
            }
        } else if metadata.is_file() && !declared_paths.contains(&relative) {
            extra.push(RelativePath::parse(relative).map_err(domain_error)?);
        }
    }
    Ok(())
}

pub(crate) fn base_addons_root(state_root: &Path) -> PathBuf {
    state_root.join(BASE_ADDONS_DIR)
}

fn state_to_dto(state: &AddOnState) -> AddOnsDto {
    AddOnsDto {
        schema_version: state.schema_version,
        addons: state
            .addons
            .iter()
            .map(|addon| AddOnDto {
                name: addon.name.as_str().to_owned(),
                source_ref: addon.source_ref.as_str().to_owned(),
                source_core_version: addon.source_core_version.clone(),
                files: addon
                    .files
                    .iter()
                    .map(|file| AddOnFileDto {
                        path: file.path.as_str().to_owned(),
                        upstream_sha256: file.hash.as_str().to_owned(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct AddOnsDto {
    schema_version: u32,
    addons: Vec<AddOnDto>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AddOnDto {
    name: String,
    source_ref: String,
    source_core_version: String,
    files: Vec<AddOnFileDto>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AddOnFileDto {
    path: String,
    upstream_sha256: String,
}

/// Test-only hooks for the synchronization contract around the shared lock.
///
/// Integration tests cannot reach into a production process to place a
/// competing writer, so this module exposes two root-keyed hooks:
///
/// * [`arm`] and [`disarm`]: a witness run at the entry of the observation
///   that authorizes `commit`, which is only ever entered while the shared lock
///   is held. It spawns and joins a competing writer that tries the lock
///   without blocking; [`SHARED_LOCK_HELD`] proves the observation ran under
///   the lock, and [`COMPETING_WRITER_MUTATED`] proves it did not.
/// * [`arm_barrier`] and [`disarm_barrier`]: an action run between the
///   fast-fail preflight and existing-lock acquisition, which forces the
///   interleaving amended acceptance row 3 requires ("preflight passes, then a
///   competing writer changes a managed path before the lock is taken").
///
/// Both hooks are inert unless a test arms them, and arming is keyed by the
/// exact workspace root so parallel tests in one process cannot interfere.
#[doc(hidden)]
pub mod addon_observation_witness {
    use super::{fs, io_error, state_root, FileExt, OpenOptions, Path, PathBuf, PortError};
    use std::sync::Mutex;

    /// Bytes the competing writer would write if it could bypass the lock.
    pub const COMPETING_BYTES: &[u8] = b"competing writer\n";
    /// Outcome recorded when the shared lock was held during observation.
    pub const SHARED_LOCK_HELD: &str = "shared_lock_held";
    /// Outcome recorded when the competing writer bypassed the lock.
    pub const COMPETING_WRITER_MUTATED: &str = "competing_writer_mutated";

    #[derive(Clone)]
    struct Armed {
        root: PathBuf,
        witness: PathBuf,
        target: PathBuf,
    }

    static ARMED: Mutex<Vec<Armed>> = Mutex::new(Vec::new());

    type BarrierAction = Box<dyn FnOnce() + Send + 'static>;

    static BARRIER: Mutex<Vec<(PathBuf, BarrierAction)>> = Mutex::new(Vec::new());

    /// Arm the lock-holding witness for `root`; the outcome is written to
    /// `witness`.
    pub fn arm(root: &Path, witness: &Path, target: &Path) {
        let mut armed = ARMED.lock().unwrap_or_else(|error| error.into_inner());
        armed.retain(|entry| entry.root != root);
        armed.push(Armed {
            root: root.to_path_buf(),
            witness: witness.to_path_buf(),
            target: target.to_path_buf(),
        });
    }

    /// Disarm the lock-holding witness for `root`.
    pub fn disarm(root: &Path) {
        let mut armed = ARMED.lock().unwrap_or_else(|error| error.into_inner());
        armed.retain(|entry| entry.root != root);
    }

    /// Arm `action` to run once, after preflight and before lock acquisition,
    /// for `root`.
    pub fn arm_barrier<F>(root: &Path, action: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut armed = BARRIER.lock().unwrap_or_else(|error| error.into_inner());
        armed.retain(|(armed_root, _)| armed_root != root);
        armed.push((root.to_path_buf(), Box::new(action)));
    }

    /// Disarm any barrier action armed for `root`.
    pub fn disarm_barrier(root: &Path) {
        let mut armed = BARRIER.lock().unwrap_or_else(|error| error.into_inner());
        armed.retain(|(armed_root, _)| armed_root != root);
    }

    pub(super) fn run(root: &Path) -> Result<(), PortError> {
        let entry = {
            let armed = ARMED.lock().unwrap_or_else(|error| error.into_inner());
            armed.iter().find(|entry| entry.root == root).cloned()
        };
        let Some(entry) = entry else {
            return Ok(());
        };
        let state_root = state_root(root);
        let target = entry.target.clone();
        let outcome = std::thread::spawn(move || {
            if fs::create_dir_all(&state_root).is_err() {
                return "state_root_unavailable";
            }
            let Ok(lock) = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(false)
                .open(state_root.join("lock"))
            else {
                return "lock_file_unavailable";
            };
            match FileExt::try_lock_exclusive(&lock) {
                Ok(()) => {
                    if let Some(parent) = target.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::write(&target, COMPETING_BYTES);
                    let _ = FileExt::unlock(&lock);
                    COMPETING_WRITER_MUTATED
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => SHARED_LOCK_HELD,
                Err(_) => "lock_try_failed",
            }
        })
        .join()
        .unwrap_or("witness_thread_failed");
        fs::write(&entry.witness, format!("{outcome}\n")).map_err(io_error)
    }

    pub(super) fn run_barrier(root: &Path) {
        let action = {
            let mut armed = BARRIER.lock().unwrap_or_else(|error| error.into_inner());
            match armed.iter().position(|(armed_root, _)| armed_root == root) {
                Some(index) => armed.remove(index).1,
                None => return,
            }
        };
        action();
    }
}
