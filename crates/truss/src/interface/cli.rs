use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::application::{
    AddOnApplication, AddOnExecutionPort, AddOnPayloadPort, AddOnPayloadSpec, AddOnPlanPort,
    AddOnStatePort, CanonicalEntrypointsPort, CoreApplication, CoreDistributionPort,
    InstallationStatePort, MigrationApplication, MigrationPort, SelfUpdateApplication,
    SelfUpdateExit, ThreeWayMergePort, UpdateCandidatePort,
};
use crate::domain::{AddOnName, SourceRef};
use crate::interface::presenter::{
    present_abort, present_addon_abort, present_addon_continue, present_addon_install,
    present_addon_status, present_addon_update, present_doctor, present_executable_recovery,
    present_install, present_migrate, present_status, present_update, CommandExit,
};

#[derive(Debug, Parser)]
#[command(
    name = "truss",
    version,
    about = "Install and safely maintain a repository Truss core"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Install a fresh core or adopt an existing copy-on-install core.
    Install {
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    /// Preview or apply a conflict-safe three-way core update.
    Update {
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        /// Continue an agent-resolved staged conflict session.
        #[arg(long = "continue", conflicts_with = "abort")]
        continue_update: bool,
        /// Abort and remove a staged conflict session.
        #[arg(long, conflicts_with_all = ["continue_update", "dry_run"])]
        abort: bool,
        /// Apply this executable's embedded core; used by a verified candidate.
        #[arg(long, hide = true)]
        candidate: bool,
    },
    /// Inspect installed version and consumer modifications without mutation.
    Status {
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Validate provenance, paths, merge support, and transaction health.
    Doctor {
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Inspect, install, update, resolve, or abort one add-on distribution.
    Addon {
        #[command(subcommand)]
        command: AddOnCommand,
    },
    /// Preview, or with `--apply` perform, the single-root migration of a
    /// legacy Truss installation. Preview mutates nothing.
    Migrate {
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        json: bool,
        /// Apply the migration as one journaled transaction.
        #[arg(long)]
        apply: bool,
    },
}

/// The add-on operations that reach the add-on application facade.
///
/// Every command here maps arguments and presents a report: it opens no state,
/// session, or manifest file and imports no infrastructure type. `continue` is
/// deliberately payload free, because S4b1 made the staged session
/// self-contained and the session carries the frozen decision.
#[derive(Debug, Subcommand)]
pub enum AddOnCommand {
    /// Report recorded provenance and whether a session is pending.
    Status {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Install an add-on payload, or preview it with `--dry-run`.
    Install {
        #[arg(long)]
        name: String,
        /// Membership manifest; its non-comment lines are the ordered path set.
        #[arg(long)]
        manifest: PathBuf,
        /// Directory holding the staged payload bytes.
        #[arg(long)]
        source: PathBuf,
        /// Immutable ref: a `truss-vX.Y.Z` tag or an exact commit SHA.
        #[arg(long = "source-ref")]
        source_ref: String,
        /// Core version the payload was acquired with.
        #[arg(long = "source-core-version", default_value = env!("CARGO_PKG_VERSION"))]
        source_core_version: String,
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    /// Preview or apply an add-on update and stage any conflicted plan.
    Update {
        #[arg(long)]
        name: String,
        /// Membership manifest; its non-comment lines are the ordered path set.
        #[arg(long)]
        manifest: PathBuf,
        /// Directory holding the staged payload bytes.
        #[arg(long)]
        source: PathBuf,
        /// Immutable ref: a `truss-vX.Y.Z` tag or an exact commit SHA.
        #[arg(long = "source-ref")]
        source_ref: String,
        /// Core version the payload was acquired with.
        #[arg(long = "source-core-version", default_value = env!("CARGO_PKG_VERSION"))]
        source_core_version: String,
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    /// Apply operator-resolved staged content for one add-on; payload free.
    Continue {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Remove only the owned add-on session; safe to repeat.
    Abort {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = ".")]
        directory: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

pub fn execute<D, S, M, C, U, AP, AS, AL, AE, MP, MC>(
    cli: Cli,
    application: &CoreApplication<D, S, M>,
    self_update: &SelfUpdateApplication<C, U>,
    add_on: &AddOnApplication<AP, AS, AL, AE>,
    migration: &MigrationApplication<MP, MC>,
) -> CommandExit
where
    D: CoreDistributionPort,
    S: InstallationStatePort,
    M: ThreeWayMergePort,
    C: UpdateCandidatePort,
    U: InstallationStatePort,
    AP: AddOnPayloadPort,
    AS: AddOnStatePort,
    AL: AddOnPlanPort,
    AE: AddOnExecutionPort,
    MP: MigrationPort,
    MC: CanonicalEntrypointsPort,
{
    let result: Result<CommandExit, String> = match cli.command {
        Command::Install {
            directory,
            dry_run,
            json,
        } => application
            .install(&directory, dry_run)
            .map(|report| present_install(&report, json))
            .map_err(|error| error.to_string()),
        Command::Update {
            directory,
            dry_run,
            json,
            continue_update,
            abort,
            candidate,
        } => {
            if abort {
                self_update
                    .discard_retained_candidate(&directory)
                    .and_then(|_| {
                        application
                            .abort_update(&directory)
                            .map_err(|error| crate::application::PortError::new(error.to_string()))
                    })
                    .map(|removed| present_abort(removed, json))
                    .map_err(|error| error.to_string())
            } else if !candidate {
                self_update
                    .execute(&directory, dry_run, continue_update, json)
                    .map(|exit| match exit {
                        SelfUpdateExit::Forwarded {
                            code,
                            stdout,
                            stderr,
                        } => CommandExit {
                            code,
                            stdout: String::from_utf8_lossy(&stdout).into_owned(),
                            stderr: String::from_utf8_lossy(&stderr).into_owned(),
                        },
                        SelfUpdateExit::Recovery(report) => {
                            present_executable_recovery(&report, json)
                        }
                    })
                    .map_err(|error| error.to_string())
            } else if continue_update {
                application
                    .continue_update(&directory, dry_run)
                    .map(|report| present_update(&report, json))
                    .map_err(|error| error.to_string())
            } else {
                application
                    .update(&directory, dry_run)
                    .map(|report| present_update(&report, json))
                    .map_err(|error| error.to_string())
            }
        }
        Command::Status { directory, json } => application
            .status(&directory)
            .map(|report| present_status(&report, json))
            .map_err(|error| error.to_string()),
        Command::Doctor { directory, json } => application
            .doctor(&directory)
            .map(|report| present_doctor(&report, json))
            .map_err(|error| error.to_string()),
        Command::Addon { command } => execute_addon(command, add_on),
        Command::Migrate {
            directory,
            json,
            apply,
        } => {
            let report = if apply {
                migration.apply(&directory)
            } else {
                migration.preview(&directory)
            };
            report
                .map(|report| present_migrate(&report, json))
                .map_err(|error| error.to_string())
        }
    };
    result.unwrap_or_else(present_error)
}

/// Map the add-on subcommands onto the add-on facade and present the reports.
///
/// Every rejection a caller can provoke from the command line happens here,
/// before the facade is entered: an unsafe add-on name and a mutable source ref
/// are refused by argument mapping, so no state, session, or manifest file is
/// opened and no mutation can precede the refusal.
fn execute_addon<AP, AS, AL, AE>(
    command: AddOnCommand,
    application: &AddOnApplication<AP, AS, AL, AE>,
) -> Result<CommandExit, String>
where
    AP: AddOnPayloadPort,
    AS: AddOnStatePort,
    AL: AddOnPlanPort,
    AE: AddOnExecutionPort,
{
    match command {
        AddOnCommand::Status {
            name,
            directory,
            json,
        } => {
            let name = parse_name(name)?;
            application
                .status(&directory, &name)
                .map(|report| present_addon_status(&report, json))
                .map_err(|error| error.to_string())
        }
        AddOnCommand::Install {
            name,
            manifest,
            source,
            source_ref,
            source_core_version,
            directory,
            dry_run,
            json,
        } => {
            let name = parse_name(name)?;
            let source_ref = SourceRef::parse(source_ref).map_err(|error| error.to_string())?;
            let spec = AddOnPayloadSpec {
                root: &source,
                manifest: &manifest,
                name: name.as_str(),
                source_ref: source_ref.as_str(),
                source_core_version: &source_core_version,
                // Recorded ownership is not enforced from here: the interface
                // opens no state and has exactly one `--manifest` flag, so this
                // caller-supplied list is empty and is never the ownership
                // check. `AddOnApplication` rejects a descriptor path already
                // owned by the core manifest or by another recorded add-on,
                // read from the workspace's own state, before planning or
                // applying anything.
                foreign_manifests: &[],
            };
            application
                .install(&directory, &spec, dry_run)
                .map(|report| present_addon_install(&report, json))
                .map_err(|error| error.to_string())
        }
        AddOnCommand::Update {
            name,
            manifest,
            source,
            source_ref,
            source_core_version,
            directory,
            dry_run,
            json,
        } => {
            let name = parse_name(name)?;
            let source_ref = SourceRef::parse(source_ref).map_err(|error| error.to_string())?;
            let spec = AddOnPayloadSpec {
                root: &source,
                manifest: &manifest,
                name: name.as_str(),
                source_ref: source_ref.as_str(),
                source_core_version: &source_core_version,
                // See `Install`: recorded ownership is enforced by the
                // application from the workspace's own state, not from this
                // empty caller-supplied list.
                foreign_manifests: &[],
            };
            application
                .update(&directory, &spec, dry_run)
                .map(|report| present_addon_update(&report, json))
                .map_err(|error| error.to_string())
        }
        AddOnCommand::Continue {
            name,
            directory,
            json,
        } => {
            let name = parse_name(name)?;
            application
                .continue_update(&directory, &name)
                .map(|report| present_addon_continue(&report, json))
                .map_err(|error| error.to_string())
        }
        AddOnCommand::Abort {
            name,
            directory,
            json,
        } => {
            let name = parse_name(name)?;
            application
                .abort(&directory, &name)
                .map(|removed| present_addon_abort(&name, removed, json))
                .map_err(|error| error.to_string())
        }
    }
}

fn parse_name(name: String) -> Result<AddOnName, String> {
    AddOnName::parse(name).map_err(|error| error.to_string())
}

fn present_error(error: String) -> CommandExit {
    CommandExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("Error: {error}\n"),
    }
}
