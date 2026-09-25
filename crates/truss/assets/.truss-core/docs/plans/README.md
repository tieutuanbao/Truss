# Execution Plans

Execution plans are Git-native working memory for complex tasks. They preserve
enough context for another agent or human to resume work without reconstructing
intent from chat history or a partial diff.

## When To Create A Plan

Use an ephemeral plan for bounded, single-session work.

Create one durable plan when work spans sessions, coordinates contributors, has
meaningful dependencies or ordering, requires recovery steps, or would be unsafe
to resume from the diff alone.

Use `.truss/core/docs/templates/exec-plan.md` and place the file under `active/`.
A delivery run's transient control artifact is not a durable plan. For a
repository-hosted run it is committed with the candidate and deleted in the
release commit, before the release-binding review. For an approved
consumer-local run it is never committed: it lives at
`.truss/delivery/runs/<run-key>/plan.md`, with the approved envelope beside it
as `approved-envelope.md`, and the approval receipt at
`.truss/delivery/approvals/<run-key>.md`. Nothing that must outlive the run
stays there.

For an explicitly authorized baseline-to-rerun Truss experiment, use
`.truss/core/docs/templates/truss-improvement.md` instead.

## Lifecycle

```text
.truss/core/docs/plans/active/<slug>.md
  -> update progress and decisions during implementation
  -> record final validation and result
  -> move to .truss/core/docs/plans/completed/<slug>.md
```

The plan is the primary task artifact. Promote a lasting product or architecture
decision into `.truss/core/docs/decisions/`; keep task-local choices in the plan.

## Active Plans

No active execution plans are currently indexed.
