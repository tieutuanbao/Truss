use std::path::Path;

use crate::domain::{AddOnDescriptor, UpdatePlan};

/// One add-on apply request.
///
/// `plan` is the S3b classification of `descriptor` against the workspace: the
/// applier consumes it and never re-plans, re-classifies, or merges. A plan
/// carrying any conflict is refused with no mutation at all.
pub struct AddOnApplyRequest<'a> {
    pub descriptor: &'a AddOnDescriptor,
    pub payload_root: &'a Path,
    pub plan: &'a UpdatePlan,
}
