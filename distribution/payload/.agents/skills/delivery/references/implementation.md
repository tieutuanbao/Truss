# Delivery implementation

Read for this phase only. Paths written in backticks are relative to the skill
root; Markdown links are relative to this file. Shared routing and authority:
[Delivery](../SKILL.md). Follow applicable repository instructions first.

## Implementation

Control creates a separate task only when that unit has its own test cycle
and an independent session could accept it while rejecting its neighbor.
Same-shaped mechanical changes are batched. Tightly coupled work stays one task
and one writer. Each independent task gets a fresh `implement` or
`visual-engineering` TUI.

For Architectural work, an implementer also reads the current detailed design
package and its `READY_FOR_PLANNING` audit — never a `NOT_READY`, absent, or
stale audit — before coding. An implementer encountering structural ambiguity
pauses the affected work and returns it through Control to
`detailed-designer` with the conflicting interpretations and evidence; no
best-judgment structural choice is authorized. The old audit is no longer
usable for the affected design. `detailed-designer` revises the package and
re-audits; high-level contract changes return to `architect`, and
product-policy changes return to `ba`/owner. A material correction follows
renewed design approval and affected re-planning before coding resumes.
Control records and supervises the pause/recovery in existing run state; no
new independent authority is created.

An implementer reads the business analysis, the decision record, the detailed
design package and audit, the plan, and the baseline — not the design session's
transcript. It owns only its task, runs the planner's mapped unit/focused test
cases and acceptance instrument, and creates one task-scoped commit. Its result
is self-verification labelled `SELF_VERIFIED`; it never returns `ACCEPT` for its
own candidate. For
behaviour with a deterministic executable test it uses TDD; the portable
invariant is smaller: observe a discriminating failure for the intended reason
before changing behaviour. A shell probe, parser fixture, or diff inspection
may be the correct instrument for configuration, documentation, generated
files, or environment-bound integration.

Concurrent implementers run only in separately approved isolated worktrees, one
writer per path, and coupled work is serialized. This is a Delivery ownership
policy stricter than Orca's placement capability. A writer never edits a path
another concurrent task owns.

The counterexample named in each acceptance row is observed red and cited.
That observation is not the behaviour's own absence: one is the feature
absent, the other is an implementation that is present, runs, returns a pass,
and is wrong. For a row using the permitted manual exception under
[Artifacts and proof](artifacts.md) § Acceptance, cite the completed inspection and its stated limit instead.

Implement the whole task before handing back. Stop and return `BLOCKED` or
`NEEDS_REPLAN` instead of a partial solution when the record contradicts the
code, the contract is ambiguous, work outside the task becomes necessary, an
existing test disproves an assumption, or the task cannot fit one session
even with normal context recovery. A task that exceeds one session's real
capacity is a decomposition failure; a worker context that fills mid-task is
ordinary recovery, handled through dispatch supervision — not a reason to
replan.

### Handoff

```text
Status:              DONE | BLOCKED | NEEDS_REPLAN
Session mode:        implementation | risk-review | integration-acceptance | diagnose-only | diagnose-and-fix
Verification type:   SELF_VERIFIED (implementation only)
Disposition:         ACCEPT | CHANGES_REQUESTED | BLOCKED (Tester only; Debugger has no acceptance disposition)
Task:                approved task identifier or exact task heading
Truss:               name, model, effort, sandbox
Dispatch:            dispatch id
Repository:          exact Git root
Worktree:            exact worktree path
Branch:              branch name
Baseline:            approved baseline SHA
HEAD:                observed current HEAD SHA
Task commits:        task and remediation SHAs, or none with reason
Changed paths:       contract-owned paths changed by this task
Contract coverage:   each applicable acceptance row, quoted or identified by
                     its exact requirement
Proof fidelity:      per applicable row, in this order: `Approved instrument`,
                     `Executed instrument`, `Provenance`, `Substitutions`,
                     `Observability limits`. Write `Substitutions: none` only
                     after comparing the two instruments.
Verification:        commands, working directories, reported outcomes, and
                     evidence locators
Deviations from plan:
Residue:             remaining work or the check that returned empty
Git state:           observed status, including protected baseline changes
END OF HANDOFF
```

Write this handoff to the worktree file designated by Control and send its
path as `payload.reportPath`, as required under [Execution](execution.md) § Launching a worker; an
inline final message does not replace the file. A dispatch
prompt names this format and the report path; it never restates the field list.
A restated list is a Control-authored contract that silently omits whatever this
contract adds, and the worker follows the prompt rather than this skill. `END OF HANDOFF` must be
the file's last line; a missing sentinel means the handoff may be truncated.
Resolve SHAs from Git, not from a planned commit.
Identify acceptance rows using existing identifiers or exact requirement
text; do not create another acceptance table or numbering system.

The worker identifies the commands it ran, their working directories,
reported outcomes, and available evidence locators. It does not transcribe
terminal output by hand or claim that its own account is independently
verified. If no recoverable locator is available, say so.

Control retrieves and cites available dispatch-bound evidence under
[Release and recovery](release-and-recovery.md) § Evidence, including its stated limits. An accepting session still reproduces
the required instruments itself.

Under `Residue`, a claim of nothing left names the check that returned empty.
`Git state` distinguishes task changes from protected baseline changes;
a clean HEAD identity alone does not establish a clean working tree.

An implementation handoff additionally records `Verification type:
SELF_VERIFIED`, its unit/focused evidence, the approved risk route, and whether
a Tester task gate is required. A Tester handoff records the mode, blind
manifest identity when applicable, candidate HEAD, risk and integration/E2E
coverage, design deviations, security and out-of-scope findings, and
observability limits. A Debugger handoff records its exact user authorization,
mode, failure, reproduction, root-cause evidence, changed paths and commit when
allowed, `Existing tests changed: none`, and out-of-scope observations; it has
no acceptance disposition.
