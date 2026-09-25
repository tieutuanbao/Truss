# Repository Workflow

Repository product behavior, architecture, decisions, plans, code, tests, and
runtime signals are the system of record.

## Communication

Every user-facing reply — answers, questions, progress updates, and
completion reports — applies `.truss-core/docs/communication.md` when it is
configured. It records a reply-style level the repository owner chose:
`expert`, `intermediate`, or `layperson`, defined in that file. Missing or
unconfigured preferences never block work; answer in clear, neutral language
and explain unfamiliar terms when needed.

An explicit communication request in the current conversation takes
precedence over the configured level. Do not infer a person's expertise,
identity, or preferences from vocabulary, pasted material, or product
audience descriptions. Adapting explanation never drops risks, uncertainty,
safety instructions, or exact commands and identifiers. In-conversation
style requests are not persisted; only an explicit owner request records a
durable change.

When re-explanation requests recur or the configured level clearly
mismatches the conversation, proactively propose switching levels, naming
the target level and the observed signal. Record the change in
`.truss-core/docs/communication.md` with its date and source only after the
owner chooses it. To configure initially, propose all three levels with a
worked example of the same answer at each level; the owner's pick is
recorded with its date and source.

## Repository Map

- `AGENTS.md`: entry map and authority boundary.
- `README.md`, `.truss-core/docs/product/`, architecture, and decisions: current intent and
  constraints.
- `.truss-core/docs/plans/`: durable work; `.truss-core/docs/templates/`: optional structures.
- Code, tests, CI, and runtime signals: executable and observable truth.

Use `.truss-core/docs/README.md` for the complete map.

## Select The Work Shape

### Does The Work Need Durable Memory?

Use an ephemeral plan for bounded work. Create one plan in
`.truss-core/docs/plans/active/` when work spans sessions, coordinates contributors, has
meaningful dependencies, needs recovery, or cannot safely resume from its diff.

Use `.truss-core/docs/templates/exec-plan.md`. Keep progress and task-local decisions in the
same file; avoid parallel task records without an independent audience.

A delivered change may additionally use its own transient control artifacts
(see Delivered Change below); those are not repository plans and never replace
this durable record when durable memory is needed.

### Does The Work Need Human Judgment?

Before editing, identify authority for new externally observable policy. If
materially different choices remain, stop and request the smallest decision.
Configurable defaults are not authority.

For example, `Add rate limiting` without a quota, trusted key, enforcement
topology, or response contract must stop. `Enforce the documented 20 requests
per minute per authenticated tenant` may proceed.

Approved records carry their own gate. When a document marks content as
accepted or approved — for example an accepted public contract — a change to
that content is a Delivered Change: invoke `$delivery` when it is installed,
or obtain the owner's explicit decision to proceed without it. When such a
change is authorized directly, update the approval record in the same edit —
what changed, who approved it, and at which revision — so no stamp names
content that no longer exists.

Also pause for ambiguous product intent, difficult recovery, weakened
validation, security, or compatibility, and insufficient authority.

### What Proves The Behavior?

Use focused tests for local rules, integration tests for boundaries, end-to-end
interaction for user-visible behavior, recovery rehearsal for dangerous
operations, and measurements for reliability or performance.

Plans, checklists, and completion messages do not prove product behavior by
themselves. Proof is revision-bound: evidence earned on one revision validates
only that revision, so re-run affected proof after any further mutation.

### Does The Work Encode An Invariant?

For architecture, reliability, security, or quality boundaries:

1. Find an accepted repository authority that states the required boundary.
   Conventions, code patterns, tests, defaults, and undocumented preferences do
   not establish policy. Stop when authority is absent or materially ambiguous.
2. Reuse the repository's native validation owner and command. Add the smallest
   mechanical check that covers the accepted scope and emits a diagnostic naming
   the violation, rule, and next action.
3. Require positive proof that allowed behavior passes and negative proof that
   the targeted forbidden behavior fails for the intended reason.
4. Report enforcement precisely: a local command is available or passed; a hook
   is optional developer convenience; CI either invokes the check or does not;
   branch protection is externally configured or unverified. Source or CI
   presence alone does not prove merge blocking.

Do not install hooks or change CI, merge, or branch-protection settings unless
separately authorized. Use the [invariant encoding pattern](patterns/encoding-invariants.md)
for the complete method.

## Task Flows

### Plan From An Idea

When the user asks to plan a product, feature, or initiative from an idea or
from external planning documents, use the product-planning skill when it is
installed: `.agents/skills/plan-product/SKILL.md`. It produces user-approved
authority and a first implementation slice, then hands off to the flows below.
Proposed policy is never recorded as accepted.

### Read-Only Request

Read only what the answer, review, diagnosis, plan, or status needs. Use
read-only inspection; do not edit files or Truss state. Discovery never
grants authority to fix what it finds.

### Bounded Change

First check the target: if the file or section being changed carries an
accepted or approved marking — especially a public contract — this is not a
bounded change; follow the approved-records gate above and route through
`$delivery` or an explicit owner decision.

Restate the outcome, inspect its authority, implementation, patterns, and proof,
make the smallest coherent change, run focused and required checks, and report
the outcome, changes, evidence, and limits.

No parallel lifecycle record is required.

### Durable Planned Change

Create or resume one active plan. Keep outcome, context, approach, risk,
recovery, progress, decisions, and validation current. Implement in verifiable
groups, promote lasting decisions, run focused and repository proof, then record
the result and move the plan to `.truss-core/docs/plans/completed/`.

### Delivered Change

When the work needs design approval before mutation and independent acceptance
before completion — Architectural work, public-contract changes, or any change
where one implementer should not grade its own homework — invoke the delivery
skill when it is installed: `$delivery`. It wraps a Bounded or Architectural
change in an approved design contract, isolated implementation, independent
tester-debugger acceptance, and release on the Orca execution plane.

Delivery is explicit-only. Ordinary bounded work never requires it, and a
delivered change still obeys this workflow: authority gates, durable plans
when the work needs them, and revision-bound proof. Delivery's own transient
plan is a per-run control artifact; anything that must outlive the run moves
into the durable decision record or an execution plan under
`.truss-core/docs/plans/active/`.

For an approved consumer-local run, the delivery control artifacts are private
and are never committed: the approved envelope and the transient plan live under
`.truss/delivery-runs/<run-key>/`, and the approval receipt lives at
`.truss/authority/approvals/<run-key>.md`. Nothing durable may be left only
there; it moves into this repository's decision record or an execution plan
before the run closes, exactly as for a repository-hosted run.

### Operate The Application

When a task requires the real application:

1. Find the consumer-owned runbook and verify prerequisites and ownership.
2. Start only an isolated instance, prove readiness, and create known state.
3. Reproduce through the real interface and inspect correlated runtime evidence.
4. Validate through that interface, then stop only resources this run owns.

If no verified runbook exists, inspect current repository authority and report
or propose the missing guidance. Do not invent commands, credentials, product
policy, or cleanup obligations. The application-runbook template supplies
proposal structure, not proof that the application is operable.

### Improve The Truss

During ordinary work, report reusable agent friction without changing the
Truss for that new purpose. When the user explicitly invokes
`$improve-truss`, use `.truss-core/docs/templates/truss-improvement.md` to:

1. preserve the observed baseline and human intervention;
2. locate the earliest missing context, capability, owner, authority, proof, or
   environment boundary;
3. make the smallest authorized change at that owner;
4. run native proof and require a materially equivalent fresh-agent rerun; and
5. decide to keep, revise, or remove the intervention.

Do not claim improvement when the rerun did not retrieve or exercise the
intervention. Keep the record active while fresh-rerun evidence is pending.

## Completion Standard

A change is complete when the outcome exists or its blocker is explicit,
repository truth remains current, behavior-appropriate proof passed or its gap
is disclosed, any required plan is current, and the report separates facts,
limits, and unattempted work. Descriptions do not replace observed proof.

Completion has levels: locally verified, review-accepted, ready to publish,
and published. Claim the level the evidence supports. A locally verified
change is complete without a pull request when the authorized scope was
local; do not infer publish authority from a local-fix request, and do not
re-run approval that an explicit authorization already covers.
