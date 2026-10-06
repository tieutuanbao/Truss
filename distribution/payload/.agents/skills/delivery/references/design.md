# Delivery design

Read for this phase only. Paths written in backticks are relative to the skill
root; Markdown links are relative to this file. Shared routing and authority:
[Delivery](../SKILL.md). Follow applicable repository instructions first.

Delivery accepts a request that may still be vague, brings it to an approved
design contract, then runs isolated implementation, risk-routed testing, and
release preparation on the Orca execution plane. It is a thin control
protocol, not an orchestrator, SDLC framework, or second source of Git state.

Read `AGENTS.md` for repository instructions and per-role truss/model/effort
pins. The `ba` role uses the co-installed `$business-analyst` skill for discovery,
requirement quality, behavioural scenarios, traceability, and handoff review;
Delivery still owns the artifact, approval, and acceptance boundaries. Follow
repository-owned workflow, developer documentation, and native validation entry
points for required gates and artifact conventions; those
values need not be stored in `AGENTS.md`. Resolve the default branch from
repository policy or verified Git/forge metadata, not the current branch.
Record resolved paths and Git targets in the approved execution envelope.
If required authority remains absent or materially ambiguous, ask before
mutation; do not invent project policy.

For documentation or configuration work with no executable acceptance
instrument, use the manual inspection permitted under [Artifacts and proof](artifacts.md) § Acceptance, naming
the responsible human reader and what that inspection cannot prove.
A missing gate command is not evidence that no gate is required, and manual
inspection does not waive an existing required gate.

## Two human gates

1. Approve the design contract before candidate mutation.
2. Accept the completed candidate: merge or publish only after release-triggered
   integration acceptance, or accept a local `IMPLEMENTED_NOT_INTEGRATION_ACCEPTED`
   completion when the envelope authorizes no release action.

Delivery pauses outside those gates only for a scope or architecture change, a
destructive action, new authority, replan, or an unavailable required runtime.

## The control session

The current interactive session is the `project-manager`. It owns the approval
invariant, task boundaries, exception handling, dispatch supervision,
integration, recovery, and authorized release. It does not implement or fix
the candidate, and it does not accept a candidate it integrated or changed.
When the `ba`, `architect`, `detailed-designer`, and `planner` roles are pinned
it does not draft the business analysis, the technical decisions, the detailed
design package, or the task plan either — it supervises those dispatches and
still owns the human gates. Delegation never moves an approval: every gate
stays with the project-manager.

Control does not prescribe question count, order, format, or skill-selection
precedence. Delivery owns the design outcome and approval boundary, not a
universal interview or planning method. The user, project, and truss
determine which design skills and native modes are active.

Native Plan Mode governs its enforced action constraints, question and plan
surfaces, artifact representation, and mode transitions. Compatible active
design skills may refine exploration and design methodology within those
constraints. When more than one applies, normal truss instruction and tool
precedence governs. One explicit approval satisfies Delivery's boundary for
the same design scope; a material scope change requires renewed approval.
Delivery does not select, activate, configure, emulate, or compose either
mechanism.

Neither one owns or can bypass the approval invariant: before candidate
mutation, Control must obtain explicit human approval of a design contract.
Plan Mode is defense in depth, not proof that requirements are clear or that
no mutation is possible.

Control surfaces unresolved uncertainty that could materially change intent,
acceptance, authority, public contract, architecture, or consequential risk.
The active design method may resolve other details from repository evidence
and convention, but material assumptions must be explicit.

## Shape: Spike, Bounded, Architectural

Use the smallest contract that safely holds the change. Risk may promote an
otherwise small change; diff size never demotes data-loss, security,
permission, or public-compatibility risk.

| Shape | Use | Artifact | Acceptance |
| --- | --- | --- | --- |
| Spike | Investigation only; no candidate is delivered | Approved probe and recommendation; no delivery run | none |
| Bounded | Small change, clear behaviour and ownership | Approved in-chat design and short execution envelope | implementer self-verification; risk review only when Architect requires it |
| Architectural | Multiple behaviours, public-contract change, architecture decision, or promoted risk | Approved decision record, audited detailed design, task plan, execution envelope | implementer self-verification; risk-routed task review; release-triggered integration acceptance |

An approved design contract states intent and success criteria; scope and
authority; affected public contract or architecture; consequential risks and
material assumptions; and a plausible counterexample or failure mode that
distinguishes correct behaviour from a present-but-wrong implementation. If no
executable instrument can discriminate the requirement, the contract names the
manual inspection and its limit.

### Roles

| Role | Owns | Dispatch |
| --- | --- | --- |
| `project-manager` | Coordination, required dev/ops preflight, approvals, dispatch, integration, recovery, authorized release | current session |
| `ba` | Evidence-backed discovery and business analysis: problem and goals, stakeholders and actors, current/target flows, rules and exceptions, scope and priorities, stable requirement IDs, user stories and behavioural scenarios, business-facing non-functional expectations, acceptance, traceability, and architect/planner handoff | fresh dispatched worker |
| `architect` | Technical contracts, interfaces, boundaries, dependencies, risk routing, and the decision record mapping choices to business requirements | fresh dispatched worker |
| `detailed-designer` | The detailed design package: modules and files, API signatures, inputs/outputs, data structures, enums, state machines, interfaces, dependency order and sequence, error handling and error codes; a compile-ready placeholder-body skeleton; and the pre-implementation completeness and ambiguity audit | fresh dispatched worker |
| `planner` | The detailed task plan: inputs, outputs, exact path ownership, dependency DAG, commands, waves, integration and recovery, plus explicit acceptance criteria and mapped test cases for every task | fresh dispatched worker |
| `implement` | Task-scoped code and documentation, unit/focused tests, self-verification evidence, and commits | fresh dispatched worker |
| `visual-engineering` | UI/UX/visual advice and authorized UI implementation, including relevant code, docs, and tests | fresh dispatched worker when the task needs it |
| `tester` | Spec-first risk review and release-triggered integration/E2E/exploratory acceptance in a fresh read-only session | fresh dispatched worker only when risk routing or a release action requires it |
| `debugger` | Narrow diagnosis or an explicitly authorized fix; never acceptance | fresh dispatched worker only after explicit user authorization |

`project-manager` is never dispatched and `visual-engineering` remains
conditional. Debugger is outside the automatic Delivery pipeline. One session
holds one role per run: a business analyst never implements or accepts, an
architect, detailed-designer, or planner never implements, an implementer never
accepts, Tester never edits the candidate, and Debugger does not accept its own
fix.

The `ba` chooses the smallest sufficient artifact under `$business-analyst`:
a focused review, a product brief when intent still needs alignment, or
Delivery's canonical business analysis for the approved design contract. It
returns `NEEDS_INPUT` rather than inventing a material product decision, and it
does not choose architecture, implementation tasks, engineering estimates, or
acceptance test decomposition.

When the managed block pins `ba`, `architect`, `detailed-designer`, and
`planner`, Architectural work is analyzed, decided, designed, audited, and
planned by those fresh, separate sessions before Control presents the design
contract at gate 1. Every Architectural run requires a fresh
`detailed-designer` dispatch after completed architecture and before planner
decomposition; missing pins, failed readiness, or a non-UI change are not
exemptions. Bounded work may keep its in-chat design, but a fresh Architect must
still record its risk routing before planning or implementation; the other
design roles remain optional for Bounded work. Those sessions never mutate the
candidate: they produce or audit the contract, then their dispatches end. Missing or incomplete pins return `NEEDS_INPUT` and route to
`$delivery-setup` before any design dispatch or candidate mutation. Control may
collect the user's request and repository evidence, but it does not draft the
specialist-owned contract as a substitute. Missing BA, architect,
detailed-designer, or planner pins never move their duties into the
project-manager.

Architect records risk routing before Planner decomposes work. The record uses
`risk.level: low | medium | high` and
`tester_task_gate: required | not_required`, with evidence-backed reasons,
affected surfaces, required Tester coverage, deferred release coverage, and
residual risk. High-risk tasks always require a Tester gate. Low-risk tasks default to no Tester.
Medium-risk tasks require an explicit choice and reason.
Missing, contradictory, or high-risk `not_required` routing returns to Architect;
Planner and Control never lower it. Architect never routes to Debugger.

After coding, Implementer runs the mapped unit/focused instruments and returns
`SELF_VERIFIED`, not independent acceptance. An ordinary task does not dispatch
Tester. A task whose approved routing requires Tester gets exactly one fresh
`risk-review`. A release-triggering action gets one fresh
`integration-acceptance` at the exact final HEAD. These are complete gates for
their scopes, not separate review and acceptance sessions.

### Architectural design sequence

Every Architectural run requires a fresh `detailed-designer` dispatch after
completed architecture and before planner decomposition. A failed, cancelled,
unavailable, or timed-out detailed-design dispatch follows existing supervised
recovery. Absence of completion never counts as a successful audit and does not
authorize planner or implementer to absorb the role. Missing pins, failed
readiness, or a non-UI change are not exemptions. Bounded work retains its
existing optional design-dispatch behavior; Spike starts no Delivery run.

The Architectural sequence is `architect` → `detailed-designer` → `planner`.
The `detailed-designer` owns the detailed design package:

- modules and file structure;
- API signatures;
- inputs and outputs;
- data structures, enums, and state-machine/state behavior;
- interfaces;
- dependency order and sequence;
- error handling and error codes;
- a compile-ready placeholder-body skeleton; and
- the pre-implementation completeness and ambiguity audit.

The durable audit's readiness value is exactly `NOT_READY` or
`READY_FOR_PLANNING`. An absent audit, absent dispatch, partial output, unknown
state, hash mismatch, or incomplete proof is treated as `NOT_READY`. Delivery
handoff status and readiness are different fields: `DONE` is not itself
permission to plan or implement.

`READY_FOR_PLANNING` requires all of the following together:

- A completed architect output precedes a successfully completed fresh
  `detailed-designer` dispatch for this Architectural change.
- Modules, files, functions, interfaces, inputs, outputs,
  dependencies/sequencing, data/enums, error handling/codes and
  state-machine/state behavior have individually referenced complete audit
  entries or justified non-applicability. The auditor checks substantive
  coverage, not headings alone.
- Structural proof and placeholder-boundary proof passed for the exact indexed
  skeleton, and the audit is bound to the current package identity.
- Every material structural ambiguity is resolved; there are no contradictory
  contracts or unresolved alternatives affecting implementation.

Only then may planner perform task decomposition and finish its plan. Planner
may request clarification, but cannot supply missing signatures, state, or
error policy itself. Even a ready audit does not authorize implementation:
Control still needs the complete planned design contract, human gate 1
approval, verified envelope identity, prerequisites, and ordinary dispatch
preflight. Missing conditions block both planner completion and implementation
start; there is no "ready with material TODOs".

The two skeleton proof results are recorded separately:

1. **Structural proof:** materialize indexed skeleton bytes with only explicitly
   referenced baseline dependencies in a disposable directory; run the
   repository's applicable compile/type/syntax instrument. Record versions,
   construction provenance, commands, exit status, and the exact tested
   digests. Missing tooling, undeclared dependencies, broken imports or
   signatures leave the package not ready.
2. **Placeholder-boundary proof:** inspect all new function/method bodies,
   initializers, macros, callbacks, and embedded executable fragments. They may
   declare shape and explicit placeholders, but must not implement business
   behavior. Compilation alone, keyword presence, and unverified worker
   assertions cannot satisfy this result.

For a Rust consumer repository, canonical new Rust callable bodies are exactly
`todo!("detailed-design placeholder")`; declarations without bodies are allowed
where the language requires them. Shape-defining fields, variants and type
relationships are real; domain algorithms, default-valued returns pretending to
be placeholders, I/O, state changes and domain-computing initializers are not.
A `todo!` somewhere in a behavior-filled function is insufficient.

Architectural work uses `templates/decision-record.md` (durable, with the
requirements-traceability mapping), `templates/business-analysis.md` (durable
product analysis), `templates/detailed-design.md` (durable detailed design and
audit authority), and `templates/plan.md` (transient; its profile-specific lifecycle is in
[Artifacts and proof](artifacts.md)). For a repository-hosted run, commit approved
durable authority before implementation begins — that commit is the acceptance
baseline. Commit a transient plan only when the approved baseline requires it.
