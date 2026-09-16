# Execution Plans

Plans are Git-native working memory for complex tasks.

Use an ephemeral plan for bounded, single-session work. Create one file under
`active/` when work spans sessions, coordinates contributors, has meaningful
dependencies, needs recovery, or cannot safely resume from its diff.

```text
.truss-core/docs/plans/active/<slug>.md
  -> keep progress, decisions, recovery, and validation current
  -> record the verified result
  -> move to .truss-core/docs/plans/completed/<slug>.md
```

Use `.truss-core/docs/templates/exec-plan.md`. Do not split one task into story, design,
trace, and validation records without an independent audience.

## Active

No durable work is currently active.

## History

Completed plans may be removed from the current tree when decisions, code,
tests, and Git history preserve their lasting result. This keeps current
retrieval focused without deleting provenance.
