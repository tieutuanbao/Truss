# Delivery with Orca

Use Delivery when a change needs design approval, separate implementation,
risk-routed testing, and release-triggered independent acceptance. Ordinary questions, reviews, and small changes use the
Core workflow without Delivery.

## Requirements

Delivery requires Orca as its execution plane. Install and launch Orca from
[onorca.dev/docs/install](https://www.onorca.dev/docs/install), then:

1. Register the CLI under **Settings → Experimental → CLI**.
2. Enable **Settings → Experimental → Orchestration**.
3. Verify the runtime:

```bash
orca-ide --version
orca-ide status --json
orca-ide orchestration run-list --json
```

On Linux, use `orca-ide`; `orca` commonly names the GNOME screen reader.

## Configure roles once

From the installed repository, ask your coding agent:

```text
$delivery-setup configure the delivery roles for this repository.
```

Choose the current harness defaults or customize the eight configurable
worker roles. The current interactive session is `project-manager`; it keeps
approval and release authority and is never dispatched, for nine roles in total:

- `ba` — business analysis and requirement quality;
- `architect` — technical contracts, boundaries, and task risk routing;
- `detailed-designer` — the detailed design package, placeholder-body
  skeleton, and pre-implementation design audit;
- `planner` — tasks, dependencies, and acceptance instruments;
- `implement` — scoped changes plus unit/focused self-verification;
- `visual-engineering` — UI and visual work when needed;
- `tester` — risk-review and release-triggered integration-acceptance;
- `debugger` — user-authorized diagnosis or narrow fixes only.

Delivery Setup migrates a legacy five-, seven-, or eight-role managed block
into the nine-role set. It preserves every existing role's Truss, Model, and
Effort cell byte-for-byte, asks you to resolve the new `detailed-designer` tuple
when it is absent, and copies the retired review/`tester-debugger` tuple to both
`tester` and `debugger`. It refuses malformed, mixed, duplicate, or ambiguous
blocks without writing and leaves an ordinary nine-role rerun unchanged.

## Start a delivered change

Give Delivery an outcome, scope, and observable acceptance:

```text
$delivery implement <outcome> under <scope>. Preserve <public contract>.
The focused check must reject <plausible wrong result>, and an independent
session must accept the exact final revision.
```

Delivery prepares a design contract before changing the candidate. For
Architectural work, the sequence is architect → detailed-designer → planner:
after the architect completes, a fresh `detailed-designer` dispatch authors the
detailed design package — modules and files, API signatures, inputs/outputs,
data structures, enums, state machines, interfaces, dependency order and
sequence, error handling and error codes — plus a compile-ready placeholder-body
skeleton and a pre-implementation completeness and ambiguity audit. The audit's
readiness value is exactly `READY_FOR_PLANNING` or `NOT_READY`; only a
`READY_FOR_PLANNING` audit bound to the current package identity permits
planner decomposition. A failed or missing design dispatch never lets the
planner or implementer absorb the role.

After your approval, Orca runs isolated implementation and the repository's
own checks. The implementer performs unit/focused tests from approved acceptance
criteria. Architect requires a Tester risk-review only for selected risky tasks;
ordinary tasks do not dispatch Tester. A single Tester integration-acceptance
runs at the exact final HEAD only when you request merge into the default branch,
marking a pull request ready, a tag, a release, or a policy-defined publish.
Release actions remain limited to what you explicitly authorize.

## Example

Suppose an API must reject an invalid state already documented in the project:

```text
$delivery change the order API to reject the invalid state documented in
.truss/authority/product/orders.md. Preserve the public error contract. The
focused test must reject the old behavior. Architect must record its risk route;
Tester accepts the exact final revision if this delivery proceeds to merge or
release.
```

Delivery should produce an approved contract, separate analysis and planning,
a scoped implementation and repository proof. Risky tasks receive one Tester
risk-review; merge or release receives one independent exact-revision
integration-acceptance. It performs only the release action you approve.

## Testing and debugging policy

Architect records `risk.level: low | medium | high` and
`tester_task_gate: required | not_required` before planning. High-risk tasks
always require `risk-review`; low-risk tasks default to no Tester; medium-risk
tasks require an explicit decision and reason. Tester prepares its task test
manifest from the approved spec before seeing implementation, then performs the
risk-focused review without editing production code or existing tests.

A local completion without a release request is
`IMPLEMENTED_NOT_INTEGRATION_ACCEPTED`: implementation and focused checks are
complete, but the candidate is not represented as release-ready. Integration,
E2E, exploratory, and whole-change review run in one `integration-acceptance`
session only for the release-triggering actions above.

Debugger is outside the automatic pipeline. Truss may recommend it after one or
two failed Implementer repair attempts or for hard-to-reproduce, multi-module,
race, performance, production/staging, or historical regression investigation,
but every Debugger dispatch needs explicit user authorization. Debugger may be
`diagnose-only` or explicitly authorized to diagnose and fix; it never weakens
existing tests, expands beyond the investigated fault, or accepts its own fix.

## When to use it

Use Delivery for:

- architectural changes;
- public contract or compatibility changes;
- changes whose author should not accept their own work;
- higher-risk work that benefits from separate analysis, implementation, and acceptance.

Do not use it for routine read-only questions or a small, clear local edit.

## Recovery and authority

The repository remains the system of record. Orca Runs, Tasks, and Dispatches
are transient execution state. Delivery never silently gains authority to
merge, push, reset, clean, stash, force-push, or edit outside the approved
scope. If the execution plane is unavailable, Delivery stops rather than
falling back to an untracked worker.

### Where Delivery artifacts live

- Run coordination, under `.truss/delivery/runs/<run-key>/`: the transient plan
  `plan.md`, the approved envelope beside it, dispatch prompts, handoffs, gate
  output, and run state.
- The approval receipt, at `.truss/delivery/approvals/<run-key>.md`.
- Durable content, under `.truss/authority/`: business analysis under
  `product/`, decisions under `decisions/`, detailed design and its separate
  design audit under `design/<design-key>/`, other architecture and design
  under the repository authority map, durable plans under `plans/`, and the
  communication choice at `communication.md`.

Classify every artifact by purpose and lifecycle, not filename. Anything that
must outlive the run moves to its authority owner before the run closes, and the
transient copy is not kept as a second canonical artifact. Ambiguous
classification, persistence, or retrieval authority returns `NEEDS_INPUT`.

### Authority retrieval

- Repository-hosted: `git cat-file -e <approved-baseline-commit>:<authority-path>`
  and `git show <approved-baseline-commit>:<authority-path> | sha256sum`.
- Consumer-local: the authorized Orca handoff carries each absolute
  candidate-local path and its SHA-256; the accepting session runs
  `test -r <absolute-path>` and `sha256sum <absolute-path>`.

A missing path, unreadable bytes, digest mismatch, wrong root, or absent
authorization returns `NEEDS_INPUT` and blocks acceptance.

The complete protocol is in
[`distribution/payload/.agents/skills/delivery/SKILL.md`](../distribution/payload/.agents/skills/delivery/SKILL.md).
