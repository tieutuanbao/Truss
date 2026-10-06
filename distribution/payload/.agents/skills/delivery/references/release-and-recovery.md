# Delivery release-and-recovery

Read for this phase only. Paths written in backticks are relative to the skill
root; Markdown links are relative to this file. Shared routing and authority:
[Delivery](../SKILL.md). Follow applicable repository instructions first.

## Release

Control performs release with native Git and forge tools; release dispatches
no release worker and makes no post-acceptance candidate edit.

1. Complete implementation self-verification and every required risk review.
2. Reconcile owning documentation, move anything durable to its one authority
   owner without retaining a second canonical run copy, and commit the complete
   candidate. For a consumer-local run, nothing private is committed and no run
   artifact is deleted here: the transient plan stays on disk, because delivery
   does not delete run artifacts on its own.
3. Run the focused instruments and project closure gates on exact HEAD.
4. Trigger one fresh Tester `integration-acceptance` only when the user asks to
   merge into the default branch, mark a pull request ready to merge, create a tag or release,
   or perform a repository-policy release publish/deploy. A task
   commit, backup push, draft pull request, or ordinary local completion does
   not trigger integration/E2E.
5. Require that Tester session's `ACCEPT` — plus required remote checks green —
   bound to the exact final HEAD.
6. Report for the second human gate: mark the pull request ready for human
   merge, or perform only the separately authorized tag/release action.

Without a release-triggering request, the local result is
`IMPLEMENTED_NOT_INTEGRATION_ACCEPTED`: focused gates are green, but the
candidate is not represented as release-ready.

Evidence is revision-bound: a verdict earned on one HEAD validates only that
HEAD. Any candidate mutation after the applicable integration acceptance
invalidates that verdict; Control reruns the affected gates and acceptance on
the new exact HEAD. Affected gates are those that can observe the change class;
a project may name that subset.

A release-triggered delivery ends at step 6 with the candidate committed on its
branch, gates green, and integration acceptance accepted on that exact HEAD.
Publishing later repeats steps 4–6 on the current HEAD; it never reuses a
verdict from an earlier revision.

Delivery never merges, force-pushes, or publishes outside the approved
target and authority. If project policy cannot publish work in progress,
Delivery delays the push and pull request until the applicable acceptance
accepts.

### Maintenance log

Maintenance logging is machine-local and opt-in at `~/.truss/delivery-log`.
Never create the directory or file; a missing path is skipped silently.
After acceptance and all required checks are green, append one line only
when the path exists, using `references/maintenance-log.md`.
Never use this log for routing, recovery, or runtime decisions.
An append failure warns but does not invalidate or block release.

## Evidence

Evidence is a property of a dispatch, not a skill-owned journal. Control asks
Orca for the dispatch-bound command, output, and outcome. Orca owns transcript
selection and cursor mechanics. For each cited read, report its source,
exactness, completeness, and any fallback or clipping the response identifies.

When required evidence is unavailable, Control must name the missing item
and label the worker's account unverified. Neither a clipped terminal tail
nor worker_done proves an unseen command, output, or counterexample check.
Cite any independent reproduction separately, with its actor and revision;
it does not establish that the worker performed the claimed check. Keep
unresolved required evidence visible rather than declaring evidence complete.

Durable candidate and release facts come from Git, CI, and the pull-request
state. This skill duplicates none of those stores.

## Failure and recovery

Recovery uses Orca records, Git, CI, and pull-request state — never inferred
from an ambiguous, missing, or merely transport-level outcome.

A failed readiness probe does not authorize a blind retry. Confirm the exact
agent/worktree trust prompt, the consumer Git root, and the selected baseline
first. Released terminals may leave child worktrees or archived resources;
inspect Orca resource accounting before deciding whether cleanup or a new Run
is safe.

| Failure | Disposition |
| --- | --- |
| In-contract implementation defect | Original Implementer gets one scoped remediation; a fresh Tester checks the new HEAD only when risk routing or release requires it |
| Scoped remediation does not accept | `NEEDS_USER_DECISION`; do not loop or auto-dispatch Debugger |
| Scope or architecture must change | Return to the design gate |
| New authority or destructive action is required | Ask the human |
| Orca or a required capability is unavailable | Stop; no headless fallback |
| A dispatch never reaches readiness (`failedStage` names the readiness stage) while the runtime still reports ready | Read `failedStage`, `lastError`, and `residualResources` first. The terminal the failed start created is still owned by that dispatch: release it with `worker-release`, never by closing it by hand. A `failed` or `stopped` attempt is replaced once with `--task` and `--retry-of` and explicit placement. A second identical failure on the same agent and worktree is an execution-plane defect, not an in-contract defect: preserve the candidate, escalate to the human with the receipt and a terminal read, and report it upstream without opening any direct or headless path |
| The candidate location would move out of the consumer checkout | Not a Control decision: present it as a gate 1 decision with the named path, or deliver in the consumer checkout when the envelope authorizes no relocation |
| Truss fails or evidence is insufficient | Preserve the candidate, report the native outcome and disposition |
| Dispatch wait times out or receipt is ambiguous | Treat as transport-unknown: re-enter the wait or read the terminal; retry only with `--task` and `--retry-of` and explicit placement when the receipt is `failed` or `stopped` and it is not progressing |
| Implementer encounters structural ambiguity | Pause the affected work and return it through Control to `detailed-designer` with the conflicting interpretations and evidence; no best-judgment structural choice. The old audit is no longer usable for the affected design. `detailed-designer` revises the package and re-audits; high-level contract changes return to `architect`, and product-policy changes return to `ba`/owner. A material correction follows renewed design approval and affected re-planning before coding resumes |
| Detailed-design dispatch fails, is cancelled, unavailable, or times out | Follow existing supervised recovery. Absence of completion never counts as a successful audit and does not authorize planner or implementer to absorb the role |
| Architect rejects the drafted contract | Control routes findings back to the owning `ba`, `architect`, `detailed-designer`, or `planner`. Missing role pins return `NEEDS_INPUT` and route to `$delivery-setup`; Control never redrafts specialist-owned content as a substitute. Gate 1 is not presented until the contract is settled |
| Control session is interrupted | Resume from Git state, the durable decision record or execution plan, and Orca run records; re-verify a live dispatch before re-engaging it; never start a competing implementer or accepting session for work already in flight |
| Idempotent release step is interrupted | Verify Git and pull-request state, then resume |

## Changing this skill

Only when the same failure recurs under the current contract — one incident
is not policy. Before adding a rule, check whether a mechanism can enforce
the fact instead. A human decides whether to promote a proposal; this skill
never mutates itself, `AGENTS.md`, or project instructions from telemetry.

## Language

Repository artifacts are English. Conversation follows the user. Enum values,
paths, commands, branch names and SHAs are never translated.
