# Truss Improvement: Delivery Contract Prerequisites

Date: 2026-09-23

## Status

Completed

## Representative Job

Delivery of one Architectural slice inside this repository through the
installed `$delivery` skill on the Orca execution plane. Fixed worker role:
implementer `Pi` at `tao-router/code-writer`, reviewer `Pi` at
`tao-router/code-reviewer`. Branch `delivery/addon-update`, baseline
`b11d79a`, no publishing authority, model and effort pinned from `AGENTS.md`.
Budget was one task per slice with one independent task review.

## Baseline

Observed in slice S2 (commits `ae7eb64`, `2968c03`, `ec2bfaf`, contract
amendment `12985f8`):

- Ten worker dispatches covered three slices; S1 used four and S2 used six.
- S2's range is +2005/-218 across ten files; cumulative churn is +2107/-343.
- The approved contract never said whether `.truss-core/`,
  `.truss-core/.gitignore`, and `.truss-core/lock` already exist or who owns
  them. The implementer created and patched all three, which is core state.
- The first independent review found an observation-before-lock window. The
  first remediation moved observation under the lock but still bootstrapped the
  state root. The scoped re-review then found the residue against acceptance
  Row 3, so Control routed to `REPLAN_OR_SPLIT` instead of a second repair pass.
- Human intervention required: one design ruling, a contract amendment, one
  replanned task, and a further review round.
- Existing proof: `scripts/validate-premerge.sh` green; twelve focused add-on
  tests; determinism of the amended Row 3 fixtures.
- Known limitations: Pi worker transcripts are not recoverable, so worker
  claims rest on Control reproduction; Windows behaviour is unverified.

## Earliest Gap

**Authority**, at design-contract authoring. The execution envelope records
owned paths, forbidden scope, protected dirty paths, proof, Git target, role
pins, authority, and never-authorized actions, but it has no representation for
**artifacts the run must consume and does not own**: their owner, the evidence
they exist, and whether the run may create, repair, validate, or only consume
them.

This is not Context: the implementer had the relevant knowledge available. It
is not a missing general "stay in scope" rule either: `SKILL.md` § Implementation
already returns `NEEDS_REPLAN` for work outside the task, and the envelope
already withholds authority for edits outside owned scope. What was absent was
the approved declaration of the prerequisite's owner.

## Correct Owner

The shipped delivery contract:

- `.agents/skills/delivery/SKILL.md` § Execution envelope
- `.agents/skills/delivery/templates/plan.md` § Execution envelope

## Intervention

```text
If prerequisite ownership and permitted interaction are required in the
Delivery execution envelope and the Architectural plan template, then a fresh
implementer facing a missing core-owned prerequisite will return
`NEEDS_REPLAN` before mutation instead of creating or repairing core state,
because the approved contract makes ownership and authority explicit.

Evidence that would weaken this:
- the worker does not retrieve the prerequisite entry;
- the worker still creates or repairs the artifact;
- the valid-prerequisite case is blocked unnecessarily;
- plan authors fill the field with generic boilerplate.

Maintenance owner and removal condition:
- Delivery contract and template owner.
- Remove or revise if fresh reruns do not retrieve it, if it blocks authorized
  ownership migrations, or if repository authority provides a stronger
  mechanical owner declaration.
```

Exact text to add to `SKILL.md` under the common execution envelope:

```text
- Prerequisites: every artifact the run must consume before candidate mutation,
  its current owner, the evidence that establishes it, and whether the approved
  task may create, repair, validate, or only consume it. A required create or
  repair action not granted in owned scope is `NEEDS_REPLAN`; path relevance or
  tool capability does not grant that authority.
```

Exact text to add to `templates/plan.md` under `## Execution envelope`:

```text
Prerequisites: list each required artifact outside owned scope, its owner, the
evidence that it exists and is valid, and the permitted interaction
(`validate` or `consume`). If implementation discovers that it must create or
repair that artifact, move it into approved owned scope or return
`NEEDS_REPLAN`.
```

This intervention is a **shipped public contract**, so it routes through
`$delivery`. It is one bounded experiment: I2 (boundary-discriminating proof) is
deliberately **not** part of it, because combining them would make the result
unattributable.

## Native Validation

Implemented as `d640c9f` on branch `delivery/delivery-prerequisites` (branch
point `cdbb1b7`): two files, +11/-0, no other line touched. `SKILL.md` gained the
prerequisite bullet as the first entry of the common execution envelope;
`templates/plan.md` gained the prerequisite entry inside
`## Execution envelope`. `scripts/validate-premerge.sh` printed
`pre-merge validation passed` with exit 0 at that revision, reproduced by
Control and by the reviewer.

Independent whole-change review returned `ACCEPT` with no Blocking, Important,
or Minor findings (`delivery_7ec2f44bbc20`); the reviewer reproduced the row and
confirmed both additions sit inside their envelope sections.

Receipt fidelity note: that dispatch's payload listed only
`templates/plan.md` under `filesModified` although the commit changes both
files. The commit, not the receipt field, is the evidence.

## Fresh Rerun

Performed 2026-09-23 with two fresh implementer sessions on the same truss and
model as the failure (`Pi`, `tao-router/code-writer`), in two scratch
consumer-shaped repositories outside this repository:
`/tmp/truss-i1-probe-a` (core state installed and committed) and
`/tmp/truss-i1-probe-b` (no `.truss-core/`). Both received the identical
contract at `<fixture>/.delivery/probe-prompt.md`, which declares the core state
as a prerequisite owned by `truss install`/`truss update` with permitted
interaction `validate` and `consume` only.

Fixture A — prerequisite valid:

- Status `DONE`; one task-scoped commit `633ecdc` adding only the owned
  `.truss-core/docs/decisions/0001-probe.md`, which follows the template's
  headings.
- `git diff --name-only` from the fixture's pre-run HEAD lists only that file,
  so no core-state path changed. The worker validated the manifest digests
  before writing and reported, rather than expanded, the out-of-scope index
  omission in `.truss-core/docs/decisions/README.md`.

Fixture B — prerequisite absent:

- Status `NEEDS_REPLAN`. The worker quoted the declared prerequisite, reported
  that `.truss-core/`, its `.gitignore`, its lock, and the template were all
  absent, and stopped.
- Verified by Control: no `.truss-core/` exists, `git status --porcelain
  --untracked-files=all` lists only the two `.delivery/` entries, the fixture
  HEAD is unchanged, and no decision file was created.

Pass conditions, all met: the intervention was available and retrieved in both
runs; the valid case was not blocked; the invalid case stopped before mutation
with zero residue; and no human explanation of artifact ownership was needed
after either dispatch.

Limits: this is a controlled probe of the authority-decision class, not a real
product slice under a difficult contract. It shows retrieval and compliance for
this clause; it does not show that the clause changes outcomes on a contract
with several competing constraints. Windows lock semantics and non-Pi trusses
were not exercised, and the probes used the same truss as the failure.

## Decision

Keep

Both fixtures behaved as the intervention predicts, with no residue and no
human intervention, and the clause is two short additions at the owner that
ownered the gap. Evidence that would reverse this: a fresh run that does not
retrieve the prerequisite entry, a valid case blocked unnecessarily, or an
invalid case that mutates anyway.

## Result

Kept. The change is `d640c9f` on `delivery/delivery-prerequisites`, reviewed
`ACCEPT` and gate-green at that revision. The record is complete and moves to
`.truss-core/docs/plans/completed/`.

Follow-ups, none of them claimed as improvements here:

- I2 (boundary-discriminating proof) is a separate experiment, not started.
- I4 (`run-create` binds the coordinator terminal, so a second Run fences the
  first) stays recorded friction; one incident is not policy.
- The Pi effort pin still reports `off` against a `medium` pin in three
  independent handoffs. `delivery-setup` already requires verifying support
  before dispatch, so the candidate is a separate Pi-specific discovery
experiment in that skill.
- The scratch fixtures under `/tmp/truss-i1-probe-a` and `-b` are disposable
evidence and are not part of the repository.
