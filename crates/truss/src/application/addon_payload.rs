use std::path::{Path, PathBuf};

use super::PortError;
use crate::domain::AddOnDescriptor;

/// Inputs for describing or checking one add-on payload.
pub struct AddOnPayloadSpec<'a> {
    /// Root that holds the payload files and the manifests.
    pub root: &'a Path,
    /// Membership manifest for this add-on; its non-comment lines are the
    /// ordered path set.
    pub manifest: &'a Path,
    /// Add-on name, validated as one safe path segment.
    pub name: &'a str,
    /// Immutable source ref the payload bytes came from.
    pub source_ref: &'a str,
    /// Core version the payload was acquired with.
    pub source_core_version: &'a str,
    /// Manifests of every other managed distribution (the core and the other
    /// add-ons), used to reject an overlapping path.
    pub foreign_manifests: &'a [PathBuf],
}

/// Builds and checks the immutable descriptor of one add-on payload.
pub trait AddOnPayloadPort {
    /// Build the descriptor for the named add-on from its membership manifest
    /// and the payload bytes at `root`.
    fn describe(&self, spec: &AddOnPayloadSpec<'_>) -> Result<AddOnDescriptor, PortError>;

    /// Re-derive the payload descriptor and require `descriptor` to agree with
    /// it on identity, ordered path set, and every digest. A descriptor derived
    /// from anything other than the payload bytes is refused.
    fn verify(
        &self,
        spec: &AddOnPayloadSpec<'_>,
        descriptor: &AddOnDescriptor,
    ) -> Result<(), PortError>;
}
