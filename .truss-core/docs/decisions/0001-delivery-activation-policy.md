# 0001 Delivery Activation Policy

Date: 2026-09-20

## Status

Accepted

## Context

Repository authorities disagreed on when `$delivery` runs. The delivery
skill description said it applies to any Bounded change ("a small, one-line
behavioral fix ... the canonical Bounded case"), the delivery block written by
`$delivery-setup` said "Bounded or Architectural work invokes `$delivery`",
while `.truss-core/docs/WORKFLOW.md` said Delivery is explicit-only and
ordinary bounded work never requires it. An agent reading the installed
surface could route the same request two ways.

## Decision

The repository owner set the activation policy on 2026-09-20:

- Ordinary bounded work proceeds through the repository workflow without
  `$delivery`. It is never mandatory for small daily work.
- `$delivery` applies to Architectural work, public-contract changes, or when
  the user explicitly requests delivered work (design approval plus
  independent review).
- `$delivery-setup` configures role/model/effort pins only. It must not
  silently change this activation policy while writing its block.

## Alternatives Considered

1. Keep "every Bounded change is delivered" (delivery-block wording). Rejected:
   ceremony on small work contradicts the smallest-workflow principle.
2. Keep the two documents as-is and rely on precedence conventions. Rejected:
   contradictory authorities are the failure mode the workflow forbids.

## Consequences

Positive:

- One routing answer across WORKFLOW.md, the delivery skill, the setup block,
  and installed AGENTS.md files.
- Delivery remains the required path where it earns its cost.

Tradeoffs:

- Users who want every change delivered must say so per repository policy;
  the shipped default no longer imposes it.

## Follow-Up

- Fresh-agent check: a bounded request must not route to `$delivery`; an
  Architectural request must propose it.
