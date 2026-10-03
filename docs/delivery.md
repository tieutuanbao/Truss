# Delivery with Orca

Use Delivery when a change needs design approval, separate implementation, and
independent acceptance. Ordinary questions, reviews, and small changes use the
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

Choose the current harness defaults or customize the dispatched roles:

- `ba` — business analysis and requirement quality;
- `architect` — technical contracts and boundaries;
- `planner` — tasks, dependencies, and acceptance instruments;
- `implement` — scoped changes and proof;
- `visual-engineering` — UI and visual work when needed;
- `tester-debugger` — independent review, diagnosis, and acceptance.

The current interactive session is `project-manager`; it keeps approval and
release authority and is never dispatched.

## Start a delivered change

Give Delivery an outcome, scope, and observable acceptance:

```text
$delivery implement <outcome> under <scope>. Preserve <public contract>.
The focused check must reject <plausible wrong result>, and an independent
session must accept the exact final revision.
```

Delivery prepares a design contract before changing the candidate. After your
approval, Orca runs isolated roles, the repository's own checks, and independent
acceptance. Release actions are limited to what you explicitly authorize.

## Example

Suppose an API must reject an invalid state already documented in the project:

```text
$delivery change the order API to reject the invalid state documented in
.truss/authority/product/orders.md. Preserve the public error contract. The
focused test must reject the old behavior, and an independent tester-debugger
must accept the exact final revision.
```

Delivery should produce an approved contract, separate analysis and planning,
a scoped implementation, repository proof, and independent exact-revision
acceptance. It performs only the release action you approve.

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

The complete protocol is in
[`distribution/payload/.agents/skills/delivery/SKILL.md`](../distribution/payload/.agents/skills/delivery/SKILL.md).
