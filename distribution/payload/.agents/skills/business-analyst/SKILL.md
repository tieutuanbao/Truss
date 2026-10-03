---
name: business-analyst
description: Use when product intent, stakeholder needs, scope, business rules, requirements, user stories, acceptance criteria, or requirements handoff need discovery, definition, or quality review before architecture, detailed design, or planning.
---

# Business Analyst

Turn uncertain product intent into an evidence-backed business contract that an
architect, a `detailed-designer`, and a planner can consume without inventing
requirements. Use the
smallest artifact that preserves the decision; never make a document more
complete-looking than the authority and evidence allow.

This skill supplies analysis methodology. When `$delivery` is active, its
approval gates, artifact locations, role boundaries, acceptance rules, and
canonical `templates/business-analysis.md` remain authoritative.

## Role boundary

The BA owns problem discovery, stakeholder and actor analysis, current and
target business flows, business rules, exceptions, scope, priorities, stable
requirement IDs, behavioural scenarios, business-facing non-functional
expectations, traceability, and handoff readiness.

The BA does not choose technical architecture, decompose implementation tasks,
decide code structure, signatures, data shapes, state behavior, or error
policy, estimate engineering effort, implement, or accept the candidate. Route
high-level technical choices to the architect, structural design questions to
the `detailed-designer`, and task/test decomposition to the planner. The BA
transfers no product ownership to any of them. Mark a missing product decision
as an open question with an owner; do not convert an assumption into a
requirement.

## Choose the smallest sufficient artifact

| Situation | Output |
| --- | --- |
| A focused question or requirement review | In-chat findings or `templates/requirements-review.md` |
| Intent is still vague or competing outcomes need alignment | `templates/product-brief.md` |
| An approved Delivery design needs durable product authority | Delivery's `templates/business-analysis.md` |
| The repository mandates BRD, SRS, FRS, PRD, or another format | Use that repository-owned format and map this skill's required content into it |

Do not create parallel BRD/SRS/FRS documents by default. One canonical artifact
is easier to approve, trace, and maintain than several overlapping documents.

## Workflow

### 1. Establish authority and evidence

Read the request, repository product authority, existing behaviour, and relevant
research. For each material statement distinguish:

- **Fact** — supported by a cited source or observed behaviour.
- **Assumption** — plausible but unverified; name its owner and validation path.
- **Proposal** — a candidate choice that remains unapproved.
- **Decision** — explicitly approved by the responsible owner.

External research may inform a proposal; it cannot replace product authority.
Use `references/discovery-and-interviews.md` when intent, actors, or flows remain
unclear.

### 2. Frame the problem before the solution

State who experiences the problem, the current behaviour, its measurable impact,
why action is needed now, and the observable outcome. Capture current-state and
target-state flows separately. If a proposed feature is presented without a
validated problem, analyze both the proposal and the need instead of treating
the proposal as the requirement.

### 3. Baseline scope and decisions

Name in-scope outcomes, non-goals, priorities, dependencies, constraints,
business rules, exception states, assumptions, and open questions. Every open
question names the owner whose answer can resolve it and whether it blocks
handoff. Stop with `NEEDS_INPUT` when missing authority could materially change
intent, scope, acceptance, permission, public contract, or consequential risk.

### 4. Define requirements and behavioural scenarios

Assign stable IDs (`REQ-001`, ...) only after a requirement is atomic enough to
remain meaningful. Never renumber or reuse an ID after another artifact cites
it. Each requirement must trace to a goal and business rule, identify priority
and authority, and state an observable acceptance condition.

Use user stories when persona, action, and value clarify behaviour; do not force
all requirements into story syntax. Review stories with INVEST and write
behavioural scenarios in Given-When-Then form where scenarios add precision.
Read `references/requirement-quality.md` and
`references/user-stories-and-acceptance.md` before producing or reviewing these
sections.

### 5. Surface non-functional expectations

Ask for business-facing expectations involving security and permission,
privacy, accessibility, availability, latency or throughput, auditability,
retention, localization, compliance, compatibility, and recovery. Record only
values with authority. An unknown threshold is an open question, not `fast`,
`scalable`, or another invented target.

### 6. Build traceability and acceptance

Maintain one chain:

`Goal → Business rule → REQ-ID → story/scenario → acceptance row`

When Delivery is active, its acceptance table adds the approved instrument,
plausible present-but-wrong implementation, observed discrimination, and
observability limits. A Gherkin scenario describes expected behaviour; it does
not by itself prove that an executable instrument distinguishes correct from
wrong implementation. Use `references/traceability-and-handoff.md`.

### 7. Review and hand off

Run the requirement-quality review before handoff. The artifact is ready only
when:

- the problem, outcomes, scope, non-goals, and priorities are explicit;
- material facts have evidence and material assumptions are labeled;
- actors, flows, rules, exceptions, and business states are defined;
- requirements are atomic, stable, unambiguous, and traceable;
- applicable happy, edge, negative, permission, and recovery behaviours are
  covered without arbitrary scenario quotas;
- non-functional expectations are explicit or owned open questions;
- acceptance can reject at least one plausible wrong result, or names the human
  inspection and its limit;
- every unresolved question has an owner and blocking status; and
- the architect, `detailed-designer`, and planner can proceed without creating
  business policy.

If these conditions are not met, return a gap report or `NEEDS_INPUT`; do not
paper over uncertainty with placeholders.

## Common mistakes

- Starting with the requested feature instead of the observed problem.
- Treating stakeholder opinion, market research, or current implementation as
  approved product policy.
- Using generic actors such as “user” when roles have different permissions or
  outcomes.
- Writing acceptance with vague adjectives or technical implementation detail.
- Requiring exactly three scenarios even when the risk shape requires fewer or
  more.
- Splitting every CRUD operation mechanically instead of by independently
  valuable behaviour.
- Letting a BA document choose architecture, code structure, task paths, APIs,
  or storage.
- Treating a `detailed-designer` structural question as a product decision to
  settle here instead of routing it to architecture or design.
- Duplicating the same requirement across several competing artifacts.
