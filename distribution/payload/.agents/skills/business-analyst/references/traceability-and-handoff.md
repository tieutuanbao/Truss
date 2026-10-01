# Traceability and handoff

Traceability exists to expose missing reasoning and control change impact. It is
not a numbering exercise.

## Canonical chain

Maintain one forward and backward chain:

`Evidence/authority → Goal → Business rule → REQ-ID → story/scenario → acceptance row`

The architect extends the chain from `REQ-ID` to technical decision and
boundary. The planner extends it to task criteria and mapped test cases. The BA
does not create those downstream choices.

## Minimum mapping

| Requirement | Goal | Rule | Story/scenario | Acceptance | Priority | Authority |
| --- | --- | --- | --- | --- | --- | --- |
| `REQ-001` | `G-01` | `BR-01` | `US-01`, `SC-01` | `ACC-01` | owner-defined | source/decision |

Use identifiers already present in the canonical artifact. Do not invent a
second requirements table solely to satisfy this reference.

## Change impact

When intent changes:

1. Identify the changed authority, goal, rule, or requirement.
2. Enumerate every downstream mapping rather than sampling.
3. Classify each item as unchanged, amended, superseded, deferred, or removed.
4. Never reuse a retired ID for new meaning.
5. Return material scope or acceptance changes to the responsible approval
   boundary before downstream work proceeds.

## Architect handoff

Provide:

- approved or proposed status of the business artifact;
- requirements, priorities, rules, flows, business states, constraints, and
  business-facing NFR expectations;
- protected public behaviour and compatibility expectations;
- assumptions and open questions, each with owner and blocking status; and
- decisions deliberately left to technical architecture.

Do not prescribe modules, APIs, storage, vendors, or deployment unless existing
product authority makes them constraints.

## Planner handoff

Provide:

- stable requirement and scenario IDs;
- dependencies and priority constraints;
- acceptance rows and required evidence;
- actor, state, exception, permission, and recovery coverage;
- scope, non-goals, and forbidden interpretations; and
- unresolved questions that block task decomposition.

The planner owns exact path ownership, dependency DAG, commands, task acceptance
criteria, and mapped test cases.

## Handoff refusal

Return `NEEDS_INPUT` instead of declaring readiness when:

- a material fact has no evidence or responsible authority;
- competing interpretations change scope, acceptance, permission, public
  contract, or consequential risk;
- a requirement cannot be traced to an approved goal or rule;
- acceptance cannot distinguish a plausible wrong result and no permitted human
  inspection is named; or
- the architect or planner would have to invent business policy.
