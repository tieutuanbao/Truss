use std::io::{self, Write};

use clap::Parser;
use truss::application::{AddOnApplication, CoreApplication, SelfUpdateApplication};
use truss::infrastructure::{
    EmbeddedCoreDistribution, FileSystemAddOnApplier, FileSystemAddOnPayload,
    FileSystemAddOnPlanner, FileSystemAddOnState, FileSystemInstallationState, GitThreeWayMerge,
    LatestReleaseCandidates,
};
use truss::interface::{execute, Cli};

fn main() {
    let cli = Cli::parse();
    let application = CoreApplication::new(
        EmbeddedCoreDistribution,
        FileSystemInstallationState,
        GitThreeWayMerge,
    );
    let self_update = SelfUpdateApplication::new(
        LatestReleaseCandidates::default(),
        FileSystemInstallationState,
    );
    let add_on = AddOnApplication::new(
        FileSystemAddOnPayload,
        FileSystemAddOnState,
        FileSystemAddOnPlanner,
        FileSystemAddOnApplier,
    );
    let exit = execute(cli, &application, &self_update, &add_on);
    if !exit.stdout.is_empty() {
        let _ = io::stdout().write_all(exit.stdout.as_bytes());
    }
    if !exit.stderr.is_empty() {
        let _ = io::stderr().write_all(exit.stderr.as_bytes());
    }
    std::process::exit(exit.code);
}
