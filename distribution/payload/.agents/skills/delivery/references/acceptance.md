# Delivery acceptance

Read for this phase only. Paths written in backticks are relative to the skill
root; Markdown links are relative to this file. Shared routing and authority:
[Delivery](../SKILL.md). Follow applicable repository instructions first.

## Risk review and integration acceptance

Tester is independent of the candidate author and declares exactly one mode:

- **`risk-review`** runs only for a task whose approved Architect routing says
  `tester_task_gate: required`. It performs spec-derived tests plus scoped code,
  convention, regression, security, and design-deviation review. It returns one
  disposition: `ACCEPT`, `CHANGES_REQUESTED`, or `BLOCKED`.
- **`integration-acceptance`** runs once at a release-triggering action on the
  exact final HEAD. It performs integration, E2E, exploratory, whole-change,
  interaction, deferred-risk, convention, regression, security, design, and
  release-readiness checks and returns the same dispositions.

Tester never edits production code or existing tests, never authors a candidate
commit in the verdict session, and never invents a product requirement. Task
review is not automatic: an ordinary task does not dispatch Tester.

### Blind-test protocol

For `risk-review`, Tester derives its test design before seeing implementation.
It initially receives only the approved requirements and design, public
interfaces, acceptance criteria, Architect risk record, pre-implementation
baseline, and allowed environment and commands. It does not receive the
implementation diff, task commit, Implementer tests or handoff, or transcript.

Tester writes a run-local, digest-bound blind test manifest containing baseline,
spec, and risk identities; spec-derived cases; expected results; plausible wrong
implementations; and the review checklist. Only after that identity is recorded
does Control disclose the exact candidate HEAD, diff, handoff, and unit evidence.
Mechanical corrections to command paths, fixture locators, setup, or executable
names are recorded. A semantic change to an oracle, expected result, failure
scenario, security property, or acceptance boundary is labelled
`POST_DISCLOSURE_TEST_CHANGE` and is not claimed as blind evidence.

A Tester finding outside the planner's acceptance criteria never changes the
Tester disposition and never blocks the current task, except that concrete
evidence of critical data loss/corruption, privilege escalation, secret
exposure, or an unintended destructive operation pauses release for Control to
escalate to Architect and the user. This safety pause grants no scope to fix or
invent a requirement. Tester records it
under `Out-of-scope review backlog` in the transient plan with: a concise title;
exact path and location when available; observed behaviour; concrete impact;
evidence or reproduction details; why it is outside the approved criteria; and
a recommended next step. Tester does not fix, absorb, or silently discard it. This routing rule applies even when the concern appears important or
security-related; Control separately decides whether authority or safety
requires pausing the wider run.

Before the run closes, the `project-manager` records one disposition for every
backlog item: moved to the repository's existing backlog; promoted to a durable
plan or owning record; dismissed with a reason; or escalated to the repository
owner because no durable destination exists. A finding that must survive closure
moves to exactly one owning authority record; the transient plan may retain a
noncanonical pointer or run evidence, never a second canonical copy. An
unresolved item may remain outside the current task, but it may not disappear
when the transient plan is deleted.

Tester independence is candidate independence: the session did not author, fix,
plan, analyse, decide, advise, or integrate the candidate and does not edit it.
The candidate author, fixer, planner, advisor, and the project-manager that
integrated it cannot act as Tester. Tester gets the approved authority artifacts,
criteria, risk record, baseline, and then the candidate inputs in the blind
protocol order. The phase adds no sandbox by default; `AGENTS.md` may pin one
for a concrete risk.

No worker runs while testing the same working tree runs. The working
tree and its gate surface are shared mutable state, and Tester reproduces gates
in that tree, so a concurrent edit makes another task's work
look like this one's result.

**Reproduce, do not accept.** Run the gates yourself. A claim you did not
reproduce is not evidence. Tester observes the counterexample discriminate for itself — an implementation that is present, runs, returns a
pass, and is wrong. An instrument red only because the behaviour was
absent is not that observation. For a row using the permitted manual
exception under [Artifacts and proof](artifacts.md) § Acceptance, verify that the named human's inspection was
completed and report its stated limit.

Classify findings: **Blocking** — contract failure, regression, data or
security risk. **Important** — missing required behaviour, test, or
reconciliation. **Minor** — useful, does not block. **Out of scope** —
recorded, not absorbed.

Return exactly one disposition: `ACCEPT`, `CHANGES_REQUESTED`, or `BLOCKED`.
State what Tester did not verify — what it did not reproduce or read. A
contradiction you cannot resolve is `CHANGES_REQUESTED`. Do not open
remediation over wording when deterministic checks already prove the contract.

### Remediation

One pass is one remediation pass per finding, not per acceptance. For an
in-contract `CHANGES_REQUESTED` finding, the **original implementer** verifies
it, fixes the root cause, reruns the affected instruments and closure gates, and
writes a separate remediation commit — this is the single original-party
remediation. Tester never fixes its own finding. Control owns the plan and the decision
record for the whole run, including remediating findings inside them. Amending
them is not implementing the candidate. The fixed candidate is checked only by
a **fresh Tester session at the new exact HEAD** when its risk route or release
action requires one; the earlier verdict
does not carry across the mutation. If that scoped check does not accept,
Control routes to `NEEDS_USER_DECISION` — it does not start another repair loop
or dispatch Debugger automatically. `BLOCKED` preserves the candidate and
escalates the unresolved dependency or authority question to Control separately
from a failed worker process; it is not remediated by the original Implementer.
A fresh replacement Tester session is used only when the original Tester is
unavailable or contested.

## Debugger

Debugger is outside the automatic Delivery pipeline. Architect never routes to
Debugger, and Control must not dispatch Debugger automatically. Control may
recommend it only after the user-visible conditions are met: one or two failed
Implementer repair attempts, a hard-to-reproduce or multi-module fault, a race
or performance failure, production/staging log or trace investigation, or a
regression requiring history or bisect. Every dispatch requires explicit user
authorization naming the fault, scope, evidence inputs, prohibited paths, and
whether mutation is allowed.

- **`diagnose-only`** reproduces and investigates, then reports evidence and
  ranked root-cause hypotheses without candidate mutation.
- **`diagnose-and-fix`** makes only the narrowest explicitly authorized fix and
  a separate commit. Debugger never changes, deletes, skips, or weakens an existing test,
  never broadens scope to another fault, and does not accept its own fix. Any
  fix returns to Implementer self-verification and any applicable fresh Tester
  gate.

If Debugger believes a test is wrong, it reports a test-contract dispute; it
does not edit the test. Control routes criterion mapping to Planner, technical
design or risk to Architect, product intent to BA or the user, and a test
implementation defect to Tester. A material contract correction requires
renewed approval.
