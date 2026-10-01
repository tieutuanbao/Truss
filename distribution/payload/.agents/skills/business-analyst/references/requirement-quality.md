# Requirement quality

Review requirements individually and as a set. A well-written sentence can
still be wrong, unauthorized, contradictory, or incomplete.

## Requirement checklist

A requirement is ready when it is:

| Quality | Review question |
| --- | --- |
| Necessary | Which approved goal and business rule require it? |
| Atomic | Can it pass or fail without hiding a second independent obligation? |
| Unambiguous | Would two informed readers expect the same observable result? |
| Feasible to assess | Is enough context present for architecture and planning without choosing the implementation? |
| Testable | Is there an observable condition that can distinguish compliance from non-compliance? |
| Traceable | Does it link to authority, goal, rule, scenarios, and acceptance? |
| Prioritized | Does the responsible owner define its priority or sequencing constraint? |
| Consistent | Does it agree with other requirements, terms, states, and scope? |
| Bounded | Are actor, trigger, inputs, state, and applicable limits clear? |
| Stable in identity | Will its `REQ-*` ID continue to mean the same obligation after downstream citation? |

## Ambiguity scan

Challenge words such as `fast`, `simple`, `intuitive`, `secure`, `appropriate`,
`normally`, `as needed`, `support`, `handle`, `etc.`, and `user-friendly`.
Replace them with an observable condition only when authority supplies one;
otherwise raise an owned open question.

Check quantifiers and boundaries: all/some, inclusive/exclusive limits, time
zones, currencies, units, rounding, ordering, duplicate handling, empty states,
and data freshness.

## Business rule form

A rule should identify:

- trigger or condition;
- actor or governed entity;
- obligation, permission, or prohibition;
- resulting business state;
- exception or precedence when another rule conflicts; and
- source authority when the rule already exists.

Avoid hiding rules only inside examples. Examples illustrate a rule; they do
not define its full population unless the authority says so.

## Non-functional expectations

Elicit only categories relevant to the outcome:

- authentication, authorization, segregation, and abuse resistance;
- privacy, retention, consent, residency, and deletion;
- accessibility and supported interaction modes;
- availability, recovery, idempotency, and continuity;
- latency, throughput, volume, concurrency, and growth boundaries;
- audit events, observability, and evidence retention;
- localization, time, number, and currency behaviour;
- compatibility, migration, and version boundaries; and
- legal, regulatory, or contractual obligations.

Record the measure, population, observation window, responsible owner, and
authority. Never translate “should be scalable” into an invented number.

## Set-level review

Verify that goals have requirements, rules have positive and exception
coverage, actor permissions do not conflict, state transitions are closed, and
non-goals are not silently reintroduced by a scenario. Identify duplicates,
gaps, contradictions, and orphan requirements.

Classify every finding as:

- **Blocking ambiguity** — materially different implementations could satisfy
  the wording; requires owner input.
- **Missing coverage** — a goal, rule, actor, state, exception, or acceptance
  path has no requirement.
- **Traceability gap** — content exists but has no authoritative upstream or
  downstream link.
- **Quality improvement** — wording can be clearer without changing meaning.
