---
name: delivery
description: Use when explicitly asked to deliver a change with $delivery, or repository instructions route architectural or public-contract implementation through Delivery. Requires Orca. Excludes read-only reviews, diagnoses, explanations, planning-only requests, routine bounded edits, and investigation spikes.
---

# Delivery

Coordinate an approved change through specialist design, scoped implementation,
risk-routed testing and authorized release. Repository authority is the system
of record; Orca owns transient execution state.

## Trigger and scope

Use the repository workflow for read-only work, ordinary bounded edits and
Spikes. Mentioning architecture or a public API during review does not start a
Delivery run. Explicit-only discovery remains configured in `agents/openai.yaml`;
repository instructions may require an explicit invocation for implementation.

Read `AGENTS.md` and relevant repository authority first. Resolve the default
branch from policy or verified Git/forge metadata, not the current branch.
The co-installed `$business-analyst` supports BA discovery. `$delivery-setup`
alone writes the role configuration; this skill consumes it.

Missing role pins return `NEEDS_INPUT` and route to setup before dispatch.
Control may collect the request and evidence; it cannot absorb specialist
duties. Missing authority or material ambiguity blocks mutation.

## Two human gates

1. Obtain explicit approval of the complete design contract and execution
   envelope before candidate mutation. One approval covers the same scope;
   material scope changes need renewed approval.
2. Present the verified candidate for human acceptance. Local completion is
   `IMPLEMENTED_NOT_INTEGRATION_ACCEPTED`; merge readiness or publishing requires
   release-triggered independent acceptance and the corresponding authority.

The current session is `project-manager` (Control), never a dispatched worker.
It coordinates, supervises, integrates and performs authorized release; it
does not implement, fix or independently accept the candidate. Specialist
sessions hold one role per run. Tester never edits the candidate; Debugger is
outside automatic routing and requires explicit fault-scoped user authorization.

## Work sequence and reference routing

Read each phase reference before performing that phase. Load only the resolved
truss's launch reference and the templates needed for the selected shape.
A dispatched worker receives this entrypoint, its phase references, approved
artifacts, task scope and evidence obligations; a prompt never substitutes a
restated contract for the canonical instructions.

| When | Required reference | Observable result |
| --- | --- | --- |
| Classify or design | [Design and roles](references/design.md) | Bounded contract, or architectural `architect → detailed-designer → planner` with current `READY_FOR_PLANNING` audit |
| Place artifacts or define proof | [Artifacts and proof](references/artifacts.md) | One owner per durable record; mapped criteria and discriminating instruments |
| Preflight, dispatch, supervise or retry | [Execution](references/execution.md), [command recipes](references/command-recipes.md), [truss routing](references/trusses.md) | Valid root/baseline, current tuple match, authorized placement, readiness and settled dispatch |
| Implement or hand off | [Implementation and handoff](references/implementation.md) | Whole scoped task, runnable proof, task commit and complete `SELF_VERIFIED` handoff |
| Required risk review, release acceptance or authorized debugging | [Acceptance and debugging](references/acceptance.md) | Independent verdict or authorized diagnosis; no self-acceptance |
| Finish, release or recover | [Release and recovery](references/release-and-recovery.md) | Revision-bound evidence, required gates and authorized final action |

Architect records `risk.level` and `tester_task_gate` before planning. High-risk
tasks require Tester; medium risk needs an explicit choice and reason; low risk
defaults to no Tester. Required task review is one fresh `risk-review`, not two
review/acceptance sessions. A user-requested release action triggers one fresh
`integration-acceptance` at the final HEAD. Commits, backup pushes, draft pull
requests and ordinary local completion do not trigger release acceptance.

## Authority and stopping conditions

- Freeze scope, ownership, prerequisites, Git targets, deployment and granted
  actions in the approved envelope. Preserve pre-existing dirty changes.
- Use the consumer checkout unless the approved envelope authorizes relocation.
  Concurrent writers need separately approved isolated worktrees.
- Reread and mechanically compare the disk role tuple before every dispatch or
  retry. A changed tuple stops this attempt; only a later delivery adopts it.
- Preserve the configured tool permission posture. A launch fallback never
  grants bypass flags or configuration changes. Additional permission authority
  must name exact actions and scope before such a change.
- Never merge, force-push, stash, reset, clean or edit outside owned scope.
  Stage, commit, push, open pull requests or publish only as authorized.
- Missing Orca, capability, readiness, authority or required evidence stops the
  affected action. No direct/headless fallback or blind mutation replay.
- Preserve candidates on failure; use the documented recovery route. No
  automatic Debugger or repeated repair loop.

## Completion and output

Report outcome, changed paths, exact revision, executed gates, recovered evidence
and unresolved limits. Distinguish worker claims from independently reproduced
proof. An approval, terminal tail or `worker_done` alone proves no unseen check.

Use the canonical handoff in [Implementation](references/implementation.md).
Local completion requires self-verification, required risk review and repository
closure gates. Release readiness additionally requires fresh independent
`ACCEPT` and required remote checks on that exact HEAD.

Repository artifacts are English; conversation follows the user. Preserve enum
values, paths, commands, branch names and SHAs.

For input/output examples and failure cases, read [Examples](references/examples.md).
For skill regression evaluation, read [Evaluation](references/evaluation.md);
these dry-run scenarios do not authorize an actual Delivery run.
