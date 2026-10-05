# Repository Workflow

Repository product behavior, architecture, decisions, plans, code, tests, and
runtime signals are the system of record.

## Communication

Apply one standard to every user-facing reply: answers, questions, progress
updates, reviews, plans, and completion reports.

- Reply in the language of the person's own words, not the language of pasted
  material.
- Never translate code, commands, paths, flags, or identifiers.
- If rules conflict, this order wins: required facts, then safety warnings,
  then brevity and format.

### Plain language

- Write for a reader without specialist knowledge, unless the person states
  their expertise or asks for a technical answer. Then follow that instead.
- Keep sentences under about 20 words. Put one idea in each sentence.
- Use common words before technical words.

### Terms and abbreviations

- Use a technical term or abbreviation only when the reply needs it.
- These need no explanation: `AI`, `URL`, `OK`, `PDF`, `email`, `Wi-Fi`.
  Explain every other term on first use, in parentheses, with one short
  sentence. Skip this when the person has stated their expertise.
- Never use an unfamiliar abbreviation before explaining it.
- With three or more terms, collect them in one terms line at the end.

For example, write "CI (an automated check run by the repository)" before
using `CI` alone.

### Conclusion first

- Open with one or two sentences that answer the question directly.
- Do not repeat the question. Do not begin with a disclaimer or throat-clearing.
- Exception: a safety or data-loss warning comes before the conclusion.
- If the whole reply is a question to the person, open with that question.

### Prefer tables and diagrams

Choose the form that carries the information most compactly:

| Information | Preferred form |
| --- | --- |
| Comparisons, choices, or specifications | Table |
| A process or processing flow | Mermaid `flowchart` |
| Communication among several participants | Mermaid `sequenceDiagram` |
| Steps the reader should follow | Numbered list |
| A short explanation | Prose |

- Keep a table to about four columns with short cells, so it stays readable on
  a small screen.
- Casual conversation, an emotional topic, and any reply under about three
  sentences use prose.

For example, compare two deployment choices in a table instead of hiding the
differences in two paragraphs.

### Mermaid rules

- Keep a diagram to about eight blocks or fewer.
- Make each diagram communicate only one idea.
- Use short labels in the person's language. Put a label in double quotes
  whenever it carries punctuation, as in `A["Gửi yêu cầu (HTTP)"]`.
- If the interface cannot display Mermaid, use a table or list instead.

For example, a flowchart can show how a request moves from a browser to a
server and then to a database.

### Everyday examples

Give an everyday example when a concept is abstract or the person seems unsure.
State where the analogy stops matching the real behavior when that limit
matters.

For example, describe a cache as a nearby cupboard that avoids another trip to
the store, then explain that cached data can become out of date.

### Length and ending

- Keep replies short by default.
- Order the end of a reply as: terms line (if any), then the closing question
  (if any).
- When a reply was shortened and more depth remains, end with one question
  asking, in the person's language, which part to explain further. For a
  Vietnamese reader: "Bạn muốn mình giải thích sâu hơn phần nào?"
- Skip that question for a one-line answer, a completion report, and a reply
  that already ends with a question.

### Required facts

Simple wording never removes risks, uncertainty, safety instructions, exact
commands or identifiers, what was verified, or what was not verified.

Do not infer expertise, identity, or preferences from vocabulary, pasted
material, or the kind of people the product is meant for.

## Repository Map

- `AGENTS.md`: entry map and authority boundary.
- `README.md`, `.truss/authority/product/`, architecture, and decisions: current intent and
  constraints.
- `.truss/authority/plans/`: durable work; `.truss/core/docs/templates/`: optional structures.
- Code, tests, CI, and runtime signals: executable and observable truth.

Use `.truss/core/docs/README.md` for the complete map.

## Karpathy Behavioral Defaults

These four principles are mandatory core workflow defaults when writing,
reviewing, or refactoring code. They require no separate skill invocation or
optional add-on. Apply their qualifications together: repository-owned authority,
authorized scope, safety, and behavior-appropriate proof remain controlling.
They do not authorize edits during a read-only request. Engineering Wisdom,
when explicitly invoked, remains contextual advice rather than new authority.

### Think Before Coding

State material assumptions and trade-offs before implementation. If materially
different interpretations remain, present them and ask for the smallest missing
decision instead of choosing silently. Keep explanation proportional to the
risk; do not bury a small, reversible task in exhaustive caveats.

### Simplicity First

Make the smallest coherent change that satisfies the authorized outcome and
its proof. Do not add speculative features, configurability, or abstractions.
A single implementation is not by itself a reason to prohibit an abstraction:
a repository-supported volatile boundary, testing need, or stable shared concept
may justify one. Explain the concrete need and preserve required validation,
error handling, and safety behavior rather than minimizing line count.

### Surgical Changes

Keep every changed line attributable to the authorized outcome, its necessary
proof, or explicitly authorized preparatory work. Do not perform unrelated
cleanup or refactoring. Requested refactoring and justified behavior-preserving
preparation are permitted within authorized scope even when the existing code
is not broken; keep their purpose and preservation proof explicit. Follow
repository conventions unless an authorized change requires otherwise; existing
style alone does not establish product policy. Remove artifacts made unused by
your change only within authorized scope; report unrelated dead code rather
than deleting it without permission.

### Goal-Driven Execution

Define observable success criteria before implementation and match proof to
the claim. For a bug, reproduce the failure before the fix; for refactoring,
check preserved behavior before and after; for new behavior, check the intended
outcome and relevant rejection cases. Use a brief plan when multiple steps need
coordination, following the existing durable-memory rules rather than creating
a parallel task record. Re-run affected proof after further mutation. Work
toward verification within authorized scope; stop and report a blocker or proof
gap instead of widening scope or claiming success without evidence.

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
when the work needs them, and revision-bound proof. Architectural work
requires a fresh `detailed-designer` dispatch after architecture and before
planning; the `architect` decides high-level contracts, the `detailed-designer`
decides structure and audits it, and `planner` decomposes tasks. Delivery's own
transient plan is a per-run control artifact, not repository authority. Classify
every Delivery artifact by purpose and lifecycle: content required after run
closure is authority; content used only to dispatch, communicate, hand off,
bind an approval, or track ephemeral run state is run coordination. Filename
alone is not decisive. A mixed-purpose artifact is split between a run-local
artifact and the owning authority record, and missing or ambiguous
classification returns `NEEDS_INPUT`.

Business analysis belongs under `.truss/authority/product/`, decisions under
`.truss/authority/decisions/`, detailed design and its separate design audit
under `.truss/authority/design/<design-key>/`, other architecture/design under
the repository authority map, and durable plans under
`.truss/authority/plans/`. The transient plan, approved envelope, prompts,
handoffs, run state, gate output, and maintenance evidence belong under
`.truss/delivery/runs/<run-key>/`, with the approval receipt at
`.truss/delivery/approvals/<run-key>.md`. Nothing durable
may be left only in run coordination; move its content to the owning authority
record before closure without retaining a second canonical run copy.

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
