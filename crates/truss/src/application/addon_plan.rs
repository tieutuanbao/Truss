use std::path::Path;

use super::PortError;
use crate::domain::{AddOnDescriptor, UpdatePlan};

/// One named add-on plan request.
///
/// The S1 `descriptor` owns the payload path set and every per-file digest, so
/// the adapter reads exactly the declared paths from `payload_root` and never
/// enumerates a directory. `payload_root` holds the staged payload bytes the
/// descriptor was derived from, never the consumer workspace.
pub struct AddOnPlanRequest<'a> {
    pub descriptor: &'a AddOnDescriptor,
    pub payload_root: &'a Path,
}

/// Classifies one named add-on payload against the workspace, read-only.
///
/// This is the interface layer's only route to add-on planning: the adapter
/// that owns the existing shared lock, the core-state validation, and the
/// recorded baseline implements this port, so no interface module imports the
/// infrastructure layer directly.
pub trait AddOnPlanPort {
    /// Plan one named add-on payload. Nothing is applied, no conflict session
    /// is staged, and no managed file, baseline, or provenance byte changes.
    fn plan(&self, root: &Path, request: &AddOnPlanRequest<'_>) -> Result<UpdatePlan, PortError>;
}
