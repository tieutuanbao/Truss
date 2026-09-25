# Business analysis template

Durable. The `ba` role produces this document from approved product authority
and real project evidence. It records what the business needs and how anyone
can tell whether a later implementation meets it.

Write it to a real project-owned path that the approved envelope names — for
example `.truss/core/docs/product/<feature>.md`. A copy of this template with
its placeholders intact is not completed analysis and must not be handed to the
planner. Every requirement needs a stable ID that the decision record and the
task plan can cite back to this document.

For an approved consumer-local run this analysis is a private artifact: it is
never committed and never staged. Write it under
`.truss/delivery/runs/<run-key>/` and obtain approval through the local receipt
`.truss/delivery/approvals/<run-key>.md` instead of through a baseline commit.

Proposals here remain drafts until a human approves them; this template does not
grant authority and does not replace the repository decision record.

---

# Business analysis — <feature or change>

## Goals

What the business wants to become true, stated as observable outcomes. One goal
per line, each measurable or checkable by a named actor or artifact.

## Actors

Every human or system that takes part, with the role it plays and the outcome it
receives. Name external systems and the repository's own components separately.

| Actor | Kind | Role in the flow | Outcome |
| --- | --- | --- | --- |
| | human / system | | |

## Business flows

The end-to-end flows the change affects. For each flow: trigger, steps in order,
the actor for each step, and the observable end state. Reference the requirement
IDs below as you go.

## Business rules

The rules that must hold regardless of implementation, written so a test or a
review can falsify them. Mark each as new or existing, and cite the current
authority when it is existing.

## Exceptions

What must happen when a rule cannot be satisfied: invalid input, missing
permission, unavailable dependency, partial completion, retry, and compensating
action. Every exception names the resulting business state, not just an error
code.

## Scope and non-goals

In scope, out of scope, and explicitly not addressed. A non-goal states why a
reader might have assumed it was included.

## Requirements

Stable IDs (`REQ-001`, …). Each requirement is atomic, testable, and traceable
to a goal, a business rule, and an acceptance condition. Do not reuse or renumber
an ID after another version has cited it.

| ID | Requirement | Goal | Rule | Acceptance condition |
| --- | --- | --- | --- | --- |
| REQ-001 | | | | |

## Acceptance

How a human or an independent tester confirms the set of requirements is
satisfied, including the negative cases that must be rejected. Name the evidence
each check produces.

## Planner handoff

The concrete handoff the planner consumes: requirement IDs and their priorities,
known dependencies between them, constraints and assumptions, open questions
with their owner, and the product documents the planner must read. State what is
deliberately left for the architect to decide.
