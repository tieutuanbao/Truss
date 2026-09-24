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

/// One add-on conflict-staging request.
///
/// `plan` is the S3b classification that carries at least one overlap conflict
/// with its resolution inputs. Staging persists those inputs and the complete
/// frozen workspace observation under the owned add-on namespace and mutates no
/// managed file, baseline, or `addons.json`.
pub struct AddOnStageRequest<'a> {
    pub descriptor: &'a AddOnDescriptor,
    pub plan: &'a UpdatePlan,
}

/// One add-on conflict-resume request.
///
/// The resolutions themselves are the staged `resolved` bytes in the owned
/// add-on session; `payload_root` re-supplies the incoming bytes so the resume
/// classifies through the same planner. Resume applies through the S3c applier
/// and its frozen-observation re-check under the shared lock.
pub struct AddOnResumeRequest<'a> {
    pub descriptor: &'a AddOnDescriptor,
    pub payload_root: &'a Path,
}
