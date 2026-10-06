# Delivery examples

Use these as request-to-action examples, not additional approval or policy.
Commands are illustrative until their values, authority and live capabilities
are verified. Run keys and task IDs must come from actual receipts.

| Request or observed input | Correct next action and output | Avoid |
| --- | --- | --- |
| “Review this proposed public API change.” | Read-only review through the repository workflow; no Delivery run. | Launching BA or Architect because the request mentions a public API. |
| “Fix this local spelling mistake.” | Ordinary bounded workflow; make the requested edit and validate it. | Adding nine-role design ceremony without a Delivery request. |
| “$delivery add the approved order transition; local completion only.” | Prepare the smallest contract and envelope; obtain gate 1. After scoped implementation, focused proof and required risk review, report `IMPLEMENTED_NOT_INTEGRATION_ACCEPTED`. | Starting integration acceptance or pushing without authority. |
| “Implement a new public error contract; repository rules require Delivery.” | Invoke Delivery explicitly, route architecture → detailed design → planning, then obtain design approval before candidate mutation. | Treating a technical design recommendation as human approval. |
| Architect says `risk.level: low`, `tester_task_gate: not_required`. | Plan `Task gate: none`; Implementer returns `SELF_VERIFIED`. | Giving the template separate reviewer and acceptance-owner sessions. |
| A role is pinned to `Codex` or `Codex CLI`. | Normalize either exact documented name to agent ID `codex`; compare the frozen model and effort. | Guessing that `Codexish` names the same agent. |
| Disk tuple changes after approval. | Script exits 2; report observed versus approved tuple and stop that dispatch. | Launching with stale pins or silently adopting the new ones. |
| Missing Orca or missing role rows, with an urgent deadline. | Return `NEEDS_INPUT`; resolve setup/runtime before execution. | Direct/headless launch or Control absorbing specialist duties. |
| Antigravity needs composed launch, with interactive tool approval. | Build `agy` plus approved model/effort; preserve permission settings and route human tool input. | Adding a bypass flag or altering user-wide permissions. |
| Final `audit.md` needs an identity. | Hash its final bytes; store the digest in the existing external envelope or consuming handoff. | Inserting its own digest into `audit.md`. |
| “Now mark the pull request ready to merge.” | Reconcile and commit the complete candidate, run required gates, dispatch fresh integration acceptance on final HEAD, then perform only the authorized action. | Reusing a verdict from an earlier revision. |

## Compact local handoff example

Use [Implementation](implementation.md) for the complete required format. This
example shows the decision fields only; it is not a substitute for a full handoff.

```text
Status: DONE
Session mode: implementation
Verification type: SELF_VERIFIED
Task: orders-transition
Approved risk route: low; tester_task_gate=not_required
Tester task gate: not required
Integration acceptance: not triggered; no release action authorized
Local result: IMPLEMENTED_NOT_INTEGRATION_ACCEPTED
```

Every claimed command, revision and evidence locator in a real handoff must be
observed, not filled from this example.
