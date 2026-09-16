---
name: plan-product
description: Turn an idea, change intent, or external planning documents into user-approved repository authority and a first implementation slice. Use only when the user explicitly asks to plan a product, initiative, or feature — never implicitly. Drafts are proposals; only user-approved content becomes authority. This skill never implements, scaffolds, installs dependencies, or launches downstream workflows.
---

# Plan Product

Take a vague or documented idea to the smallest approved authority set the
existing workflow can implement. The repository stays the system of record;
the skill produces proposals until the user approves specific content.

## Safety Contract

- Explicit invocation only. Installation alone is not activation.
- Drafts are proposals. Nothing becomes authority until the user approves that
  specific content; approval names its scope.
- Never implement, scaffold, run `git init`, install dependencies, start
  services, or launch `$delivery` or any other workflow. The handoff names the
  next workflow; the user starts it.
- If another lifecycle (for example BMAD) owns this change, stop and request a
  handoff. One implementation owner per change.
- Do not fill unknown product policy with plausible defaults. Record open
  questions instead.

## Method

### 1. Establish The Starting Point

Read `AGENTS.md`, `.truss-core/docs/WORKFLOW.md`, applicable
`.truss-core/docs/decisions/`, and any input documents the user supplies. If
external planning documents (for example BMAD briefs or specs) are supplied,
treat them as source material, not authority. Capture the current revision and
dirty state before proposing anything.

### 2. Clarify Until The Decision Changes

Ask only questions whose answers change intent, scope, acceptance criteria, or
consequential risk. Do not run a fixed questionnaire. Record each unresolved
material question in an explicit Open Questions section; a draft with
unresolved material questions says so and says what depends on them.

### 3. Draft The Smallest Authority Set

Write proposals into the existing owners — nowhere else:

| Content | Destination |
| --- | --- |
| Goal, users, expected behavior, scope, acceptance criteria | `.truss-core/docs/product/<initiative>.md` |
| Lasting product or architecture choices | `.truss-core/docs/decisions/<decision>.md` (proposal status) |
| First implementation slice, owner, and validation intent | One plan in `.truss-core/docs/plans/active/`, only when the work needs durable memory |

Keep each document small and split by topic. The plan references the
authority; it never restates it as a second specification. An acceptance
criterion is testable or it is not an acceptance criterion — say how it would
be checked, or mark it open.

### 4. Obtain Approval

Present exactly the content awaiting decision: purpose, the specific sections,
and the open questions that materially affect them. State the repository
revision the proposal was drafted against.

- Approval covers the named content at that revision. Record accepted
  decisions in the decision record and mark approved product sections
  accepted.
- Content the user does not approve stays draft. Partial approval is normal.
- A material change to approved content requires renewed approval.
- Product approval never grants authority to scaffold, commit, run services,
  or publish.

### 5. Hand Off

End by naming: the workflow that should implement the first slice (bounded
change, durable plan, or `$delivery` when the change is Architectural or the
user requests it), any authority still missing, and any prerequisite (for
example a repository baseline commit). Then stop. Implementation starts only
when the user asks for it.

## Self-Check Before Handing Anything Back

- Every proposed sentence is classified: observed fact, proposal, or open
  question. No unapproved assumption reads as accepted policy.
- No contradictions between draft documents, decisions, and existing
  authorities.
- Every acceptance criterion states how it would be verified.
- Open Questions lists every material unresolved decision and what it blocks.
- One implementation owner is named for the slice, and no other lifecycle is
  currently active on that scope.

## Handback

Report: what was drafted and where, what the user approved (sections and
revision), what stays draft, open questions, the named next workflow, and
missing authorities or prerequisites.
