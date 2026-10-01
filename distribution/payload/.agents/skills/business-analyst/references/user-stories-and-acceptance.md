# User stories and acceptance criteria

User stories are a communication form, not the requirement system. Use them
when actor, intent, and value improve shared understanding. Keep regulatory,
data, cross-cutting, and non-functional obligations as requirements when story
syntax would distort them.

## Story form

```text
US-<ID>: <outcome-oriented title>
As a <specific actor in a relevant state>
I want <one observable capability>
So that <business or user value distinct from the action>
Maps to: REQ-<ID>[, ...]
```

A story ID does not replace its stable `REQ-*` mappings.

## INVEST review

| Lens | Question | If weak |
| --- | --- | --- |
| Independent | Can it be delivered and judged without silently requiring another story? | Name the dependency or split by an independently valuable boundary. |
| Negotiable | Does it preserve solution space not fixed by business authority? | Remove accidental architecture or UI prescription. |
| Valuable | Is the outcome valuable to a specific actor or business goal? | Clarify value or question whether the story is needed. |
| Estimable | Are behaviour, scope, rules, and uncertainty clear enough for planning? | Add business context; do not estimate for engineering. |
| Small | Is it one coherent outcome with a bounded scenario set? | Split by workflow step, actor, rule, risk, data boundary, or operational state. |
| Testable | Can an independent observer distinguish success from failure? | Tighten scenarios and acceptance evidence. |

INVEST is a diagnostic, not a pass/fail ritual. Document dependencies rather
than pretending every story is independent.

## Behavioural scenarios

```gherkin
Scenario: <observable business outcome>
  Given <business-relevant starting state>
  And <additional authorized precondition>
  When <one actor action or external event>
  Then <observable result and resulting business state>
  And <additional outcome when needed>
```

Cover the scenario classes the rules and risks require:

- successful path;
- boundaries and state transitions;
- invalid or missing input;
- authorization and visibility;
- duplicate, retry, timeout, partial completion, and recovery;
- cancellation or compensation; and
- compatibility or migration, when applicable.

Do not impose an arbitrary minimum scenario count. One scenario should express
one behaviour. Use concrete states, values, and boundaries; avoid “works”,
“valid”, “properly”, or “an error is shown” without the resulting business
state.

## Keep behaviour separate from implementation

Describe what an actor or system observes. Do not prescribe endpoint paths,
database tables, framework calls, button colors, internal classes, or storage
unless an approved public contract makes them business-relevant.

Poor:

```gherkin
Then POST /v1/profile returns 200 and updates users.email
```

Better:

```gherkin
Then the verified account uses the new email for future notifications
And the previous email no longer receives account notifications
```

## Acceptance is stronger than scenario prose

A scenario can guide implementation and testing, but Delivery acceptance must
also name the instrument that proves it, the plausible present-but-wrong
implementation that the instrument rejects, the observed discrimination, and
what remains unobserved. Do not claim Gherkin wording itself is proof.

## Splitting guidance

Split only when each result carries independently valuable behaviour and can be
accepted on its own. Useful boundaries include actor, workflow stage, business
rule, permission, data population, risk, or operational state. CRUD verbs,
scenario count, or a universal number of engineering days are signals to
inspect—not automatic split rules.
