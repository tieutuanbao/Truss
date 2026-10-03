# Repository Workflow

Repository product behavior, architecture, decisions, plans, code, tests, and
runtime signals are the system of record.

## Communication

Every user-facing reply — answers, questions, progress updates, and completion
reports — follows the reply-style level recorded in
`.truss/authority/communication.md`. The level belongs to the project, so it
lives under `.truss/authority/`; `.truss/core/` is never a write target.

### Levels

- `expert` — precise terminology and abbreviations without expansion; minimal
  background; evidence, risks, and exact commands preserved. Lead with the
  result; skip motivation the reader already has.
- `intermediate` — terminology allowed, explained on first use; enough
  background for an informed reader; outcome, effect, and next step stated.
  May be organized around the work performed.
- `layperson` — plain language, no unexplained term or abbreviation;
  analogies welcome; explicit cause, effect, and next step. Must also satisfy
  *What `layperson` requires* below.

### Resolution order

1. An explicit style request in the current conversation (not persisted).
2. Constraints in the `Notes` of `communication.md` (language, length).
3. The configured level.
4. Otherwise: `intermediate`, in the language the person is writing in. A
   missing, unconfigured, or invalid record never blocks work.

When the record exists but its level cannot be read — absent, misspelled, or an
unrecognised value — say once in the reply that the level could not be read and
which level is being used instead, then continue. Never fall through silently:
an unreadable level is a defect the owner can only fix if it is named.

Unless 1 or 2 says otherwise, reply in the language the person writes in.
Code, commands, paths, flags, and identifiers are never translated.

### Invariants at every level

Adapting explanation never drops: risks, uncertainty, safety instructions,
what was verified and what was not, and exact commands and identifiers.

### Do not infer

Do not infer expertise, identity, or preferences from vocabulary, pasted
material, or product audience descriptions.

### Initial configuration

When the record is missing or its level is `Not configured`, offer the choice
once per conversation, at the first reply that goes beyond a one-line answer
(an explanation, plan, or report). Append the offer after the answer; never
precede or replace it. Give a one-line description of each level and the key
point of the pending answer rendered at each level (max 3 sentences each).

- On a choice: create the record from `.truss/core/docs/templates/communication.md`
  and fill in the level, date, and source.
- On a decline: do not re-offer in this conversation. If the owner says not to
  ask again, record `declined` with date and source.

### Changing the level

- The person in the conversation is treated as the owner. A durable change
  needs an explicit instruction to save or change the project default;
  one-off style requests are not persisted.
- Propose a switch only on observed behavior: the same re-explanation requested
  twice, or an explicit "too long / too basic / too technical". Name the target
  level and the signal. Propose at most once per conversation; record the change
  only after the owner chooses.

### What `layperson` requires

**Wording (every reply, including one-liners):** no unexplained term or
abbreviation; paths, commands, flags, and identifiers stay exact, and may
appear whenever the reader needs them to act. An analogy must not change the
facts; say where it stops being accurate.

**Shape (reports and multi-part replies; a short answer needs only wording
and a next step):**

1. Organize around the reader's problems, not the work performed. Section
   count follows problems, not work items.
2. Open each section on a problem in the reader's terms; headings state a plain
   claim about their situation, not a process label.
3. Report work as before/after: the symptom the reader would notice, then what
   happens now.
4. Internal work vocabulary (session, phase, gate, dispatch, checkpoint) is
   never the subject of a sentence; translate it into an observable effect.
   Do not narrate who did what, in which file, in what order.
5. State why each item matters.
6. State limits plainly: what was not checked, and any author's error that
   affects trust.
7. End with the reader's next action and its exact command, or say that
   nothing is needed.

**Contrast**

- ✗ "Phase 2 complete: auth middleware refactored across 3 sessions; gate passed."
- ✓ "Before: typing a wrong password blanked the page, so people thought the
  site was down. Now: a message says the password is wrong. Not checked: the
  phone layout. Next: run `npm test` to confirm on your machine."

## Repository Map

- `AGENTS.md`: entry map and authority boundary.
- `README.md`, `.truss/authority/product/`, architecture, and decisions: current intent and
  constraints.
- `.truss/authority/plans/`: durable work; `.truss/core/docs/templates/`: optional structures.
- Code, tests, CI, and runtime signals: executable and observable truth.

Use `.truss/core/docs/README.md` for the complete map.

## Select The Work Shape

### Does The Work Need Durable Memory?

Use an ephemeral plan for bounded work. Create one plan in
`.truss/authority/plans/active/` when work spans sessions, coordinates contributors, has
meaningful dependencies, needs recovery, or cannot safely resume from its diff.

Use `.truss/core/docs/templates/exec-plan.md`. Keep progress and task-local decisions in the
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
the result and move the plan to `.truss/authority/plans/completed/`.

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
plan is a per-run control artifact, not repository authority. Classify every
Delivery artifact by purpose and lifecycle: content required after run closure
is authority; content used only to dispatch, communicate, hand off, bind an
approval, or track ephemeral run state is run coordination. Filename alone is
not decisive. A mixed-purpose artifact is split between a run-local artifact and
the owning authority record, and missing or ambiguous classification returns
`NEEDS_INPUT`.

Business analysis belongs under `.truss/authority/product/`, decisions under
`.truss/authority/decisions/`, architecture/design under the repository
authority map, durable plans under `.truss/authority/plans/`, and communication
choice at `.truss/authority/communication.md`. The transient plan, approved
envelope, prompts, handoffs, run state, gate output, and maintenance evidence
belong under `.truss/delivery/runs/<run-key>/`, with the approval receipt at
`.truss/delivery/approvals/<run-key>.md`. Nothing durable may be left only in
run coordination; move its content to the owning authority record before closure
without retaining a second canonical run copy.

For a repository-hosted run, commit approved durable authority in the baseline.
If an ignore rule matches an approved authority path, use explicit path-scoped
`git add -f -- <path>` only when the approved envelope grants staging and
commit authority. For an approved consumer-local run, durable authority stays
repository-local and is never staged, committed, or force-added.

Repository-hosted acceptance checks authority availability with
`git cat-file -e <approved-baseline-commit>:<authority-path>` and identity with
`git show <approved-baseline-commit>:<authority-path> | sha256sum`. Consumer-local
acceptance receives each absolute candidate-local path and SHA-256 through an
authorized Orca handoff, then runs `test -r <absolute-path>` and
`sha256sum <absolute-path>`. A missing path, unreadable bytes, digest mismatch,
wrong root, or absent authorization returns `NEEDS_INPUT` and blocks acceptance.

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
`$improve-truss`, use `.truss/core/docs/templates/truss-improvement.md` to:

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
