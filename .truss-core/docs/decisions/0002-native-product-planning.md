# 0002 Native Product Planning Skill

Date: 2026-09-20

## Status

Accepted

## Context

Truss requires pre-existing repository authority and therefore supports a
repository only after product truth exists. Greenfield work — starting from an
idea — has no Truss-owned path. BMAD-METHOD fills that gap with facilitated
elicitation, product briefs, PRDs, and architecture documents, but embedding
BMAD would import a second lifecycle, a disk state machine, and a second
approval model that conflict with Truss's single-owner, no-control-plane
principles (consultation with an independent advisor, 2026-09-20).

Key clarification adopted with this decision: "do not invent policy" does not
mean "do not propose policy". The agent may analyze, draft, and propose; the
user decides; documents record the decision. Planning adds elicitation
capability without loosening the authority gate.

## Decision

Build one Truss-native, explicit-only planning skill (`$plan-product`) in an
optional installer profile. Do not embed or depend on BMAD.

Scope: the skill ends at (a) user-approved authority documents and (b) a first
implementation slice. It never scaffolds, runs `git init`, installs
dependencies, implements, or launches downstream workflows.

Borrowed from BMAD:

- Facilitated elicitation with open questions, asked only when the answer
  changes intent, scope, acceptance, or risk.
- Approved content is recorded with the exact section and repository revision
  it was approved at; material changes require renewed approval.
- Artifacts stay small and split by topic.
- A self-check for contradictions, unapproved assumptions, and verifiable
  acceptance criteria.

Rejected for v1:

- Spec frontmatter state machines, epic/story hierarchies, sprint tracking.
- Mandatory review fan-out or finding quotas.
- A separate handoff adapter, import schema, or two-way synchronization with
  BMAD.

Authority promotion: a draft is a proposal. Authority exists only after the
user approves specific content; the skill then marks those sections accepted
in the owning document. Unapproved sections remain drafts. Approval of a
product document never grants authority to scaffold, commit, run services, or
publish.

Conflict contract with BMAD (or any second lifecycle):

- One implementation owner per change, declared in the plan (or in-chat
  contract): which system executes, scope, accepted authority, baseline, and
  what the other system must leave untouched. This is a coordination rule, not
  a concurrency lock.
- Each system edits only its own instruction markers; a consumer AGENTS.md
  must state which system owns a change. Conflicting blocks stop before
  mutation for an owner decision.
- BMAD artifacts are accepted only when the user selects them, critical open
  questions are resolved or scoped out, and the content is recorded in the
  Truss/consumer owner. Accepted BMAD output becomes reference material, never
  a parallel authority.
- BMAD review loopbacks must not revert or re-derive code a Truss workflow
  owns; findings transfer as input instead.
- BMAD state files (`_bmad/`, sprint status) are left untouched. "No
  control-plane state" means Truss creates and owns no additional control
  plane; it does not forbid the consumer from keeping BMAD tooling.

## Alternatives Considered

1. Bless BMAD as the required upstream planner. Rejected: makes Truss's
   greenfield path depend on an external system and dual lifecycles.
2. Ship nothing. Rejected: leaves the ideation gap closed only by a competitor
   workflow.

## Consequences

Positive:

- Greenfield: idea to approved authority to first slice, inside Truss's
  existing document owners.
- BMAD remains usable as an external planning source through the same
  acceptance gate, with one lifecycle owner per change.

Tradeoffs:

- Planning quality depends on the interviewing agent; the skill mitigates with
  question discipline and an explicit open-questions section, not machinery.

## Follow-Up

- Fresh-agent scenarios must pass before calling this settled: vague idea,
  partial approval, BMAD draft with unresolved questions, conflicting
  instruction blocks, scope change after approval.
- Extend only after observed recurring failure, never to match BMAD's
  feature list.
