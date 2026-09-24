use std::path::Path;

use crate::domain::{AddOnDescriptor, UpdatePlan};

/// One add-on apply request.
///
/// `plan` is the classification of `descriptor` against the workspace: the
/// applier consumes it and never re-plans, re-classifies, or merges. A plan
/// carrying any conflict is refused with no mutation at all.
pub struct AddOnApplyRequest<'a> {
    pub descriptor: &'a AddOnDescriptor,
    pub payload_root: &'a Path,
    pub plan: &'a UpdatePlan,
}

/// One add-on conflict-staging request.
///
/// `plan` is the classification that carries at least one overlap conflict with
/// its resolution inputs. `payload_root` holds the candidate payload bytes the
/// descriptor was read from, so staging can store the complete candidate for
/// every descriptor path and the session becomes self-contained (decision 0003
/// clause 12). Staging persists those bytes, the resolution inputs, the
/// materialised plan, and the complete frozen workspace observation under the
/// owned add-on namespace, and mutates no managed file, baseline, or
/// `addons.json`.
pub struct AddOnStageRequest<'a> {
    pub descriptor: &'a AddOnDescriptor,
    pub payload_root: &'a Path,
    pub plan: &'a UpdatePlan,
}

// A resume request is deliberately absent: the staged session is
// self-contained, so resume takes only the workspace root and the add-on name
// and reads the candidate payload, the descriptor identity, the materialised
// plan, and the operator-edited resolutions out of the session itself.
