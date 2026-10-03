# Plan template

Transient run coordination. Its path is
`.truss/delivery/runs/<run-key>/plan.md` in both profiles, beside the immutable
`approved-envelope.md`; filename alone does not make it durable. For a
repository-hosted run, commit it only where the approved Delivery contract
requires it in the baseline and delete it in the release commit, before the
release-binding review, after durable content has moved to its authority owner.
For an approved consumer-local run, it is never committed and delivery does not
delete it. It is not a durable execution plan and is never duplicated as one.

One coherent contract. One acceptance table. Split tasks only when each has
its own test cycle and can be reviewed independently; batch same-shaped
mechanical changes and keep tightly coupled work together.
No status duplicated anywhere else.

---

# Plan — <the contract this delivers>

Decision record: `<path>`

**Baseline:** for a repository-hosted run, the SHA of the approved baseline
commit carrying durable authority and, where the Delivery contract requires it,
this transient plan; leave it empty until that commit exists rather than
guessing it. For an approved consumer-local run, the exact
pre-implementation commit of the candidate, whose approval identity lives in the
local receipt `.truss/delivery/approvals/<run-key>.md` instead of in a commit.

## Goal

One paragraph. What is true when this is done, and what is out of reach.

## Allowed scope

```
<paths that may change>
```

Before approval, enumerate the source paths, applicable colocated tests,
registry tests, and owning documents this plan needs to change. Include
those paths in allowed scope; relevance alone does not grant ownership.
State which searches yielded nothing — checked, not assumed. Report each
enumeration command and its output. Check all proposed paths against
protected pre-existing dirty changes under the execution envelope.

## Forbidden scope

Paths a reader might expect to be included, with why they are not. Name anything
whose filename suggests relevance but whose contents do not.

## Execution envelope

Record the approved common envelope from `../SKILL.md` § Execution envelope.
Use this plan's Allowed scope, Forbidden scope, Acceptance, and Closure gates
sections by reference; do not duplicate their contents.

Prerequisites: list each required artifact outside owned scope, its owner, the
evidence that it exists and is valid, and the permitted interaction
(`validate` or `consume`). If implementation discovers that it must create or
repair that artifact, move it into approved owned scope or return
`NEEDS_REPLAN`.

Protected dirty paths: name pre-existing changes and their ownership, or
record that the worktree was clean at baseline.

Git target: exact worktree, feature branch, baseline, base, remote, and
pull-request target. For local delivery, state that publishing is not authorized.

Resolved role pins: the truss, model, and effort for each dispatched role,
taken from `AGENTS.md` and resolved against the live truss surface.

Authority: record only granted branch, owned-path commit, gate, push, and
pull-request actions. Local delivery explicitly excludes push and pull-request
authority. Never authorized: merge, force-push, stash, reset, clean or other
cleanup, or edits outside owned scope.

## Tasks

### 1. <behaviour, not activity>

**Behaviour.** What becomes true. Observable, not internal.

**Direction.** Enough for an implementer to start without re-deciding the contract.
Not a pasted implementation.

**Inputs.** The approved artifacts this task consumes — business analysis,
decision record, plans, existing code — and the permitted interaction with each.

**Outputs.** The artifacts and behaviour this task produces, including the
commit it owns.

**Owned paths.** Exact paths this task may change. If a task needs a file
outside allowed scope, the plan is wrong — fix it now, not at acceptance.
Exact ownership means no two tasks in one wave own the same path.

**Dependencies.** Task numbers that must be integrated first, or `none`.

**Acceptance criteria.** Number each observable condition that must be true for
this task to pass. Criteria define the review boundary: they must be specific
enough for a reviewer to return a verdict without inventing requirements.

**Test cases.** Map every case to one or more acceptance criteria. For each case,
record its preconditions, action or exact command, expected result, and the
criterion IDs it proves. Include success, failure, and boundary cases required
by the approved behaviour. A criterion with no mapped case is unfinished.

**Focused verification.** The command or check that proves this task, its
expected result, and how it fails if the task is not done.

**Worktree / branch / base.** The worktree, branch, and base commit the task
runs in. Concurrent writers get separately approved isolated worktrees; coupled
work is serialized in one checkout.

**Wave.** The parallel wave that may run this task, and which tasks share it.

**Review owner.** The fresh `tester-debugger` session expected to review only
this task's acceptance criteria and mapped test cases. It is not the acceptance
owner.

**Acceptance owner.** A different fresh `tester-debugger` session expected to
accept this task. The author, fixer, planner, advisor, or reviewer for a
candidate cannot accept it.

**Document impact.** Which owning documents this task obliges you to reconcile, and
why each one — ownership, not habit.

## Integration, waves, and recovery

- **Integration order:** the exact order tasks integrate, and who integrates
  (the `project-manager`), before fresh exact-HEAD integration acceptance.
- **Waves:** which tasks may run concurrently in separately approved worktrees,
  and which are serialized because they share a checkout or a path.
- **Conflict and recovery:** how overlapping or failing work is recovered from
  the task's base and recorded commits, without reset, stash, or clean.
- **Acceptance owner:** the fresh `tester-debugger` session for integration
  acceptance of the combined candidate at its exact final HEAD.

## Out-of-scope review backlog

Reviewers record findings that do not map to this plan's acceptance criteria
here. These findings do not change the current task's review disposition and do
not block that task.

| Finding | Location | Observed behaviour | Impact | Evidence | Why outside criteria | Recommended next step | Project-manager disposition |
| --- | --- | --- | --- | --- | --- | --- | --- |
| | | | | | | | Pending |

Before the run closes, the `project-manager` replaces every `Pending` value
with exactly one outcome: moved to the repository's existing backlog; promoted
to a durable plan or owning record; dismissed with a reason; or escalated to
the repository owner because no durable destination exists. Do not delete the
transient plan while any row remains `Pending`.

## Acceptance

| Requirement | Instrument | Counterexample | Observed red |
| --- | --- | --- | --- |
| | | | |

Design fills Counterexample; implement fills Observed red. An empty cell is an
unfinished row. "The feature is absent" does not satisfy Counterexample. A row
with no counterexample must say so, name the responsible human reader, and
state what reading the diff cannot prove. Its Observed red cell records the
completed manual inspection and its limit instead of an executable failure.

Every row's instrument must tell a pass from a failure. A row that cannot
discriminate is not acceptance; either replace the instrument or record that no
instrument exists and that a human reads the diff.

**Cannot be observed:** what the available instruments do not cover. A green suite
that never exercises a surface is not evidence about that surface.

## Stop conditions

What makes this `BLOCKED` or `NEEDS_REPLAN` rather than something to work around.
Include any assumption that holds for one tool, truss or environment and has not
been verified for the others this plan touches.

## Closure gates

Closure includes a check that `Out-of-scope review backlog` has no `Pending`
disposition. This check governs run closure, not the pass/fail result of the
current implementation task.

```
<exact commands, with the directory each runs from>
```

If a gate is skipped, record the boundary and why the change cannot be observed by
it. Report every gate with the exact command, the summary line verbatim, and the
exit code. `$?` after a pipe reports the last command in the pipe, not the gate.
