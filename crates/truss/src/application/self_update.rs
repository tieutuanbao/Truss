use std::path::Path;

use semver::Version;

use crate::application::{CandidateRequest, InstallationStatePort, PortError, UpdateCandidatePort};

pub struct SelfUpdateApplication<C, S> {
    candidates: C,
    state: S,
}

#[derive(Debug)]
pub enum SelfUpdateExit {
    Forwarded {
        code: i32,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
    Recovery(ExecutableRecoveryReport),
}

#[derive(Debug)]
pub struct ExecutableRecoveryReport {
    pub executable_version: String,
    pub core_version: String,
    pub dry_run: bool,
    pub applied: bool,
}

impl<C, S> SelfUpdateApplication<C, S>
where
    C: UpdateCandidatePort,
    S: InstallationStatePort,
{
    pub fn new(candidates: C, state: S) -> Self {
        Self { candidates, state }
    }

    pub fn execute(
        &self,
        root: &Path,
        dry_run: bool,
        continue_update: bool,
        json: bool,
    ) -> Result<SelfUpdateExit, PortError> {
        // A repository holding both trees is refused before any other port call:
        // the handoff below resolves the installed tree, so a conflict would
        // otherwise be written into the new root while the legacy root is the
        // installed one.
        self.state.resolve_state_root(root)?;
        let installed = self
            .state
            .load(root)?
            .ok_or_else(|| PortError::new("core is not installed; run `truss install`"))?;
        let installed_version = parse_version("installed core", &installed.core_version)?;
        let executable_version = parse_version("installed executable", env!("CARGO_PKG_VERSION"))?;

        if installed_version > executable_version {
            let candidate = self
                .candidates
                .staged(root, &installed.core_version)
                .or_else(|_| self.candidates.exact(&installed.core_version))?;
            self.verify_identity(&candidate, &installed_version)?;
            if !dry_run {
                self.candidates.validate_replacement_target(root)?;
                self.candidates.persist(root, &candidate)?;
                self.candidates.replace(&candidate)?;
                self.candidates.clear_persisted(root)?;
            }
            return Ok(SelfUpdateExit::Recovery(ExecutableRecoveryReport {
                executable_version: executable_version.to_string(),
                core_version: installed_version.to_string(),
                dry_run,
                applied: !dry_run,
            }));
        }

        let candidate = if continue_update {
            let session = self
                .state
                .load_resolution(root)?
                .ok_or_else(|| PortError::new("no update resolution is pending"))?;
            self.candidates.staged(root, &session.to_version)?
        } else {
            self.candidates.latest()?
        };
        let candidate_version = self.verify_identity(&candidate, &installed_version)?;
        if candidate_version < executable_version {
            return Err(PortError::new(format!(
                "release candidate {candidate_version} is older than installed executable {executable_version}"
            )));
        }
        let replaces_executable = candidate_version > executable_version;
        if replaces_executable && !dry_run {
            self.candidates.validate_replacement_target(root)?;
        }
        if !continue_update && !dry_run {
            // Starting a normal update explicitly abandons any pinned plan and
            // retained candidate before handing off to the latest candidate.
            self.state.clear_resolution(root)?;
            self.candidates.clear_persisted(root)?;
        }
        if replaces_executable && !dry_run {
            self.candidates.persist(root, &candidate)?;
        }

        let output = self.candidates.execute(
            &candidate,
            &CandidateRequest {
                root,
                dry_run,
                continue_update,
                json,
            },
        )?;

        if output.code == 2 && !dry_run {
            self.candidates.persist(root, &candidate)?;
        } else if output.code == 0 && !dry_run && replaces_executable {
            self.candidates.replace(&candidate).map_err(|error| {
                PortError::new(format!(
                    "core files updated; executable recovery remains pending: {error}"
                ))
            })?;
            self.candidates.clear_persisted(root)?;
        } else if output.code == 0 && !dry_run {
            self.candidates.clear_persisted(root)?;
        }

        Ok(SelfUpdateExit::Forwarded {
            code: output.code,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    pub fn discard_retained_candidate(&self, root: &Path) -> Result<(), PortError> {
        self.state.resolve_state_root(root)?;
        self.candidates.clear_persisted(root)
    }

    fn verify_identity(
        &self,
        candidate: &C::Candidate,
        installed_version: &Version,
    ) -> Result<Version, PortError> {
        let release_version = parse_version(
            "release pointer",
            self.candidates.release_version(candidate),
        )?;
        let reported = self.candidates.reported_version(candidate)?;
        let candidate_version = parse_version("release candidate", &reported)?;
        if candidate_version != release_version {
            return Err(PortError::new(format!(
                "release identity mismatch: pointer={release_version}, candidate={candidate_version}"
            )));
        }
        if candidate_version < *installed_version {
            return Err(PortError::new(format!(
                "release candidate {candidate_version} would downgrade installed core {installed_version}"
            )));
        }
        Ok(candidate_version)
    }
}

fn parse_version(label: &str, value: &str) -> Result<Version, PortError> {
    Version::parse(value)
        .map_err(|error| PortError::new(format!("invalid {label} version {value}: {error}")))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    use super::*;
    use crate::application::{CandidateExit, UpdateCandidatePort};
    use crate::domain::{
        ApplyReceipt, FrozenWorkspaceFile, InstallationState, UpdateResolutionSession,
        WorkspaceMutation,
    };

    const CONFLICTING_ROOT: &str = "conflicting-root";

    #[derive(Default)]
    struct CandidateFixture {
        calls: Rc<RefCell<Vec<String>>>,
    }

    impl UpdateCandidatePort for CandidateFixture {
        type Candidate = String;

        fn latest(&self) -> Result<String, PortError> {
            self.calls.borrow_mut().push("latest".to_owned());
            Ok("1.0.0".to_owned())
        }
        fn exact(&self, _version: &str) -> Result<String, PortError> {
            self.calls.borrow_mut().push("exact".to_owned());
            Ok("1.0.0".to_owned())
        }
        fn staged(&self, _root: &Path, _version: &str) -> Result<String, PortError> {
            self.calls.borrow_mut().push("staged".to_owned());
            Ok("1.0.0".to_owned())
        }
        fn release_version<'a>(&self, candidate: &'a String) -> &'a str {
            candidate
        }
        fn reported_version(&self, _candidate: &String) -> Result<String, PortError> {
            self.calls.borrow_mut().push("reported_version".to_owned());
            Ok("1.0.0".to_owned())
        }
        fn execute(
            &self,
            _candidate: &String,
            _request: &CandidateRequest<'_>,
        ) -> Result<CandidateExit, PortError> {
            self.calls.borrow_mut().push("execute".to_owned());
            Ok(CandidateExit {
                code: 0,
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
        fn persist(&self, _root: &Path, _candidate: &String) -> Result<(), PortError> {
            self.calls.borrow_mut().push("persist".to_owned());
            Ok(())
        }
        fn clear_persisted(&self, _root: &Path) -> Result<(), PortError> {
            self.calls.borrow_mut().push("clear_persisted".to_owned());
            Ok(())
        }
        fn validate_replacement_target(&self, _root: &Path) -> Result<(), PortError> {
            self.calls
                .borrow_mut()
                .push("validate_replacement_target".to_owned());
            Ok(())
        }
        fn replace(&self, _candidate: &String) -> Result<(), PortError> {
            self.calls.borrow_mut().push("replace".to_owned());
            Ok(())
        }
    }

    #[derive(Default)]
    struct StateFixture {
        calls: Rc<RefCell<Vec<String>>>,
        installation: RefCell<Option<InstallationState>>,
    }

    impl StateFixture {
        fn conflict(&self, root: &Path) -> bool {
            root == Path::new(CONFLICTING_ROOT)
        }
    }

    impl InstallationStatePort for StateFixture {
        fn resolve_state_root(&self, root: &Path) -> Result<PathBuf, PortError> {
            self.calls
                .borrow_mut()
                .push("resolve_state_root".to_owned());
            if self.conflict(root) {
                return Err(PortError::new(
                    "both .truss/core and .truss-core hold a Truss installation",
                ));
            }
            Ok(root.to_path_buf())
        }
        fn recover_interrupted(&self, _root: &Path) -> Result<bool, PortError> {
            self.calls
                .borrow_mut()
                .push("recover_interrupted".to_owned());
            Ok(false)
        }
        fn transaction_pending(&self, _root: &Path) -> Result<bool, PortError> {
            Ok(false)
        }
        fn load(&self, _root: &Path) -> Result<Option<InstallationState>, PortError> {
            self.calls.borrow_mut().push("load".to_owned());
            Ok(self.installation.borrow().clone())
        }
        fn read_workspace_file(
            &self,
            _root: &Path,
            _path: &crate::domain::RelativePath,
        ) -> Result<Option<Vec<u8>>, PortError> {
            Ok(None)
        }
        fn validate_managed_path(
            &self,
            _root: &Path,
            _path: &crate::domain::RelativePath,
        ) -> Result<(), PortError> {
            Ok(())
        }
        fn apply(
            &self,
            _root: &Path,
            _state: &InstallationState,
            _mutations: &[WorkspaceMutation],
        ) -> Result<ApplyReceipt, PortError> {
            Ok(ApplyReceipt { backup_path: None })
        }
        fn apply_if_unchanged(
            &self,
            _root: &Path,
            _state: &InstallationState,
            _mutations: &[WorkspaceMutation],
            _expected: &[FrozenWorkspaceFile],
        ) -> Result<ApplyReceipt, PortError> {
            Ok(ApplyReceipt { backup_path: None })
        }
        fn resolution_pending(&self, _root: &Path) -> Result<bool, PortError> {
            self.calls
                .borrow_mut()
                .push("resolution_pending".to_owned());
            Ok(false)
        }
        fn stage_resolution(
            &self,
            _root: &Path,
            _session: &UpdateResolutionSession,
        ) -> Result<(), PortError> {
            self.calls.borrow_mut().push("stage_resolution".to_owned());
            Ok(())
        }
        fn load_resolution(
            &self,
            _root: &Path,
        ) -> Result<Option<UpdateResolutionSession>, PortError> {
            self.calls.borrow_mut().push("load_resolution".to_owned());
            Ok(None)
        }
        fn clear_resolution(&self, _root: &Path) -> Result<bool, PortError> {
            self.calls.borrow_mut().push("clear_resolution".to_owned());
            Ok(false)
        }
    }

    /// Finding 1: self-update is a mutating entry point, so a repository holding
    /// both trees must be refused before any other port call. The call log proves
    /// the refusal is the first thing that happens.
    #[test]
    fn self_update_refuses_a_conflicting_root_before_any_other_port_call() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let app = SelfUpdateApplication::new(
            CandidateFixture {
                calls: Rc::clone(&log),
            },
            StateFixture {
                calls: Rc::clone(&log),
                installation: RefCell::default(),
            },
        );

        let error = app
            .execute(Path::new(CONFLICTING_ROOT), false, false, false)
            .unwrap_err()
            .to_string();
        assert!(error.contains(".truss/core"), "names the new root: {error}");
        assert!(
            error.contains(".truss-core"),
            "names the legacy root: {error}"
        );
        assert_eq!(
            log.borrow().as_slice(),
            ["resolve_state_root"],
            "the refusal must precede every other port call"
        );

        let discard = app
            .discard_retained_candidate(Path::new(CONFLICTING_ROOT))
            .unwrap_err()
            .to_string();
        assert!(
            discard.contains(".truss-core"),
            "names the legacy root: {discard}"
        );
        assert_eq!(
            log.borrow().as_slice(),
            ["resolve_state_root", "resolve_state_root"],
            "discarding a retained candidate is also refused before it touches the tree"
        );
    }
}
