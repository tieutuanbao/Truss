use std::path::Path;

use crate::domain::AddOnDescriptor;

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
