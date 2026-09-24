use std::path::Path;

use super::PortError;
use crate::domain::{AddOnDescriptor, AddOnName, ApplyReceipt, UpdatePlan};

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

/// Applies, stages, resumes, and aborts one add-on update.
///
/// Every method is add-on scoped. In particular `session_pending` asks only
/// whether the owned `.truss-core/addon-update/<name>/` session exists; it must
/// never overload the core [`InstallationStatePort::resolution_pending`],
/// which reads the core-only `.truss-core/update/` namespace.
///
/// [`InstallationStatePort::resolution_pending`]: super::InstallationStatePort::resolution_pending
pub trait AddOnExecutionPort {
    /// Apply one conflict-free plan transactionally. A plan carrying any
    /// conflict is refused with no mutation at all.
    fn apply(
        &self,
        root: &Path,
        request: &AddOnApplyRequest<'_>,
    ) -> Result<ApplyReceipt, PortError>;

    /// Stage one conflicted plan under the owned add-on namespace instead of
    /// applying it. No managed file, baseline, or provenance byte changes.
    fn stage(&self, root: &Path, request: &AddOnStageRequest<'_>) -> Result<(), PortError>;

    /// Resume the self-contained staged session for `name` by name alone. The
    /// session is read, never re-planned, and only the owned session is
    /// cleared after provenance is written.
    fn resume(&self, root: &Path, name: &AddOnName) -> Result<ApplyReceipt, PortError>;

    /// Remove only the owned session for `name`; a repeat call is `false`.
    fn abort(&self, root: &Path, name: &AddOnName) -> Result<bool, PortError>;

    /// Report whether `.truss-core/addon-update/<name>/session.json` exists.
    /// The add-on session is inspected only; the core session namespace is
    /// never read, and a schema-1 session is pending rather than an error.
    fn session_pending(&self, root: &Path, name: &AddOnName) -> Result<bool, PortError>;
}
