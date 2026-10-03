# Execution Plans

Execution plans are Git-native working memory for complex tasks. They preserve
enough context for another agent or human to resume work without reconstructing
intent from chat history or a partial diff.

## When To Create A Plan

Use an ephemeral plan for bounded, single-session work.

Create one durable plan when work spans sessions, coordinates contributors, has
meaningful dependencies or ordering, requires recovery steps, or would be unsafe
to resume from the diff alone.

Use `.truss/core/docs/templates/exec-plan.md` and place the file under
`.truss/authority/plans/active/`. Classify a plan by purpose and lifecycle, not
filename: cross-session recovery memory is authority; mutable per-run task
control is run coordination.

A delivery run's transient control artifact is not a durable plan. Its path is
`.truss/delivery/runs/<run-key>/plan.md` in both profiles, with the approved
envelope beside it as `approved-envelope.md` and the approval receipt at
`.truss/delivery/approvals/<run-key>.md`. For a repository-hosted run it is
committed only where the Delivery baseline requires it and deleted in the
release commit, before the release-binding review. For an approved
consumer-local run it is never committed. The two plan contracts are never
duplicated: anything that must outlive the run moves to the owning authority
record, and the transient copy is not retained as a second canonical artifact.

For an explicitly authorized baseline-to-rerun Truss experiment, use
`.truss/core/docs/templates/truss-improvement.md` instead.

## Lifecycle

```text
.truss/authority/plans/active/<slug>.md
  -> update progress and decisions during implementation
  -> record final validation and result
  -> move to .truss/authority/plans/completed/<slug>.md
```

The plan is the primary task artifact. Promote a lasting product or architecture
decision into `.truss/authority/decisions/`; keep task-local choices in the plan.

## Active Plans

No active execution plans are currently indexed.
