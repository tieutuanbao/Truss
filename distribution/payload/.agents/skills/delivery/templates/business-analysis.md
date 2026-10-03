# Business analysis template

Durable. The `ba` role produces this document from approved product authority
and real project evidence. It records what the business needs and how anyone
can tell whether a later implementation meets it. Use `$business-analyst` for
discovery, requirement quality, user-story and scenario guidance, and handoff
review; this template remains Delivery's canonical business artifact.

Write it to the exact repository-authority path named by the approved envelope,
normally `.truss/authority/product/<feature>.md`. A copy of this template with
its placeholders intact is not completed analysis and must not be handed to the
planner. Every requirement needs a stable ID that the decision record and the
task plan can cite back to this document.

For a repository-hosted run, the approved baseline commit carries the exact
authority path and SHA-256. For an approved consumer-local run, the analysis
stays repository-local at that authority path: it is never staged, committed, or
force-added, and the accepting dispatch carries its absolute candidate-local
path and SHA-256. If placement, persistence, or retrieval authority is missing
or ambiguous, return `NEEDS_INPUT` before writing the analysis.

Proposals here remain drafts until a human approves them; this template does not
grant authority and does not replace the repository decision record.

---

# Business analysis — <feature or change>

## Status, authority, and evidence

Name the responsible product owner, approval status, authority sources, observed
current behaviour, and material research. Classify important claims as fact,
assumption, proposal, or approved decision.

## Problem and measurable impact

Who experiences what current behaviour, how often, with what observable impact,
and why the problem matters now. Do not restate the proposed feature as the
problem.

## Goals and success measures

What the business wants to become true, stated as observable outcomes. Give each
goal a stable local ID (`G-01`, ...), a measure or review artifact, and an owner.
Unknown baselines or targets remain owned open questions.

## Stakeholders and actors

Every human or system that initiates, participates in, receives, administers,
audits, or is affected by a flow. Name external systems and the repository's own
components separately.

| Actor | Kind | Role in the flow | Outcome | Authority or concern |
| --- | --- | --- | --- | --- |
| | human / system | | | |

## Current and target business flows

Describe current and target flows separately. For each: trigger, ordered actor
actions, decision points, business states, exceptions, and observable end state.
Reference applicable requirement IDs when they exist.

## Business rules and exceptions

Give rules stable local IDs (`BR-01`, ...). State the condition, obligation,
permission or prohibition, resulting business state, precedence, and source
authority. For invalid input, denied permission, unavailable dependency,
duplicate action, partial completion, timeout, retry, cancellation, and
compensation, record the required resulting business state rather than only an
error code.

## Scope, non-goals, and priorities

State in-scope outcomes, exclusions, deferrals, and owner-defined priorities. A
non-goal explains why a reader might otherwise assume it is included.

## Assumptions, dependencies, and open questions

| Item | Type | Impact if wrong or unavailable | Owner | Blocking? |
| --- | --- | --- | --- | --- |
| | assumption / dependency / question | | | yes / no |

Do not hand a material blocking question to the architect or planner as an
implicit requirement.

## Requirements

Stable IDs (`REQ-001`, ...). Each requirement is atomic, unambiguous, testable,
prioritized by the responsible owner, traceable to authority, goal, rule, and
acceptance, and free of accidental implementation detail. Do not reuse or
renumber an ID after another artifact cites it.

| ID | Requirement | Goal | Rule | Priority | Authority | Acceptance |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-001 | | G-01 | BR-01 | | | ACC-01 |

## User stories and behavioural scenarios

Use stories only where actor, action, and value clarify the requirement. Map
each story and Given-When-Then scenario to one or more `REQ-*` IDs. Review
stories with INVEST, but record real dependencies rather than pretending every
story is independent. Cover happy, boundary, negative, permission, retry, and
recovery behaviour as applicable; do not impose an arbitrary scenario count or
include technical implementation detail.

## Non-functional expectations

Record authorized, business-facing expectations for security and permission,
privacy, accessibility, availability, recovery, performance and volume,
auditability, retention, localization, compliance, and compatibility. Include
the measure, population, observation window, owner, and authority where
applicable. Unknown thresholds remain open questions, not invented targets.

## Acceptance and discrimination

Use one acceptance table. Each row maps requirements to the approved instrument,
the plausible implementation that is present and runs but is wrong, where that
instrument was observed to reject the counterexample, the expected evidence,
and observability limits. A Given-When-Then scenario specifies behaviour; it is
not executable proof by itself.

| ID | Requirements | Approved instrument | Present-but-wrong counterexample | Discrimination observed | Expected evidence | Observability limits |
| --- | --- | --- | --- | --- | --- | --- |
| ACC-01 | REQ-001 | | | | | |

Where a row names artifact provenance — a released artifact, version boundary,
legacy installation, tag, or revision — the instrument must name the command
that constructs that artifact, not only the artifact. Running an existing suite
alone does not establish fixture provenance. State what the construction command
proves that a substitute could not.

## Traceability

Show the complete chain without inventing downstream technical or task choices:

`Evidence/authority → Goal → Business rule → REQ-ID → story/scenario → acceptance row`

Enumerate gaps; do not sample. The architect extends this mapping to technical
decisions and boundaries, and the planner extends it to task criteria and mapped
test cases.

## Architect and planner handoff

For the architect: approved requirements, priorities, flows, states, rules,
constraints, business-facing non-functional expectations, protected public
behaviour, assumptions, open questions, and choices deliberately left to
technical architecture.

For the planner: stable requirement and scenario IDs, dependencies, priorities,
acceptance rows and evidence, scope and non-goals, forbidden interpretations,
and unresolved blockers. The BA does not choose implementation paths, task DAGs,
commands, or mapped test cases.
