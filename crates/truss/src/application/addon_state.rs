use std::path::Path;

use super::PortError;
use crate::domain::{AddOnDescriptor, AddOnName, AddOnState, RelativePath};

/// Inputs for installing or adopting one add-on from a staged payload.
pub struct AddOnInstallRequest<'a> {
    /// Descriptor derived from the payload bytes at `payload_root`.
    pub descriptor: &'a AddOnDescriptor,
    /// Root that holds the payload bytes. This is never the consumer
    /// workspace: the baseline is copied from these bytes, so a consumer edit
    /// can never be recorded as upstream.
    pub payload_root: &'a Path,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AddOnRecordReceipt {
    /// True when every managed path already matched the payload exactly and
    /// no workspace file had to be written.
    pub adopted: bool,
}

/// Reads and writes installed add-on provenance.
///
/// The shape mirrors `InstallationStatePort`: one reader and one writer over a
/// versioned state file, with the workspace files written before the
/// provenance, never after. Installed add-on provenance lives in its own file,
/// `.truss-core/addons.json`, with its own schema version, and its baseline
/// copies live under `.truss-core/base-addons/<add-on>/`.
pub trait AddOnStatePort {
    /// Load the installed add-on record, or `None` when no record exists.
    /// An unreadable, schema-mismatched, or digest-mismatched record is an
    /// error rather than an absent record.
    fn load(&self, root: &Path) -> Result<Option<AddOnState>, PortError>;

    /// Every path owned by a distribution other than `own_name`, paired with
    /// the owning distribution's name: the core installation's
    /// `.truss-core/manifest.json` entries and the recorded paths of every
    /// other add-on in `.truss-core/addons.json`.
    ///
    /// This is the ownership set an incoming add-on descriptor must not
    /// collide with, and it is deliberately required rather than defaulted: a
    /// default would let an implementation report an empty set, which is the
    /// empty-ownership hole the real command line must not have. The set comes
    /// from the workspace's own recorded state, so no caller supplies it and no
    /// command-line flag exists for it; an absent or unreadable core state is a
    /// refusal rather than an empty set.
    fn recorded_owners(
        &self,
        root: &Path,
        own_name: &AddOnName,
    ) -> Result<Vec<(String, Vec<RelativePath>)>, PortError>;

    /// Put the payload files in place, then write the baseline copies and the
    /// record.
    ///
    /// A workspace with no managed path yet is a fresh install: every payload
    /// file is written. A workspace that already holds a managed path is a
    /// legacy install and is adopted only when every managed path already
    /// equals the payload bytes and no extra managed path exists; otherwise the
    /// operation stops with no workspace mutation, no baseline, and no record.
    fn apply(
        &self,
        root: &Path,
        request: &AddOnInstallRequest<'_>,
    ) -> Result<AddOnRecordReceipt, PortError>;
}
