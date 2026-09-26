# Execution Plan: truss migrate (stage 4 of decision 0008 item 7)

Date: 2026-09-26

## Status

Active

## Outcome

`truss migrate` migrates a legacy Truss installation (`.truss-core/`, and the
legacy delivery roots `.delivery/` and `.delivery-dispatch/`) to the single
`.truss/` root with `core/`, `authority/`, and `delivery/` namespaces. Preview is
the default and mutates nothing; `--apply` runs a verified, backed-up,
rollback-capable transaction and prints the exact retained backup path. `truss
status`, `truss doctor`, and `truss addon status` refuse a dual-root repository
and recommend the migration preview.

## Context

- `.truss/authority/decisions/0008-single-truss-root.md` — root layout, item 7
  staging, and the 2026-09-26 amendment accepting `truss migrate` with its
  migration-only ownership exception and operator-visible limits.
- `.truss/authority/decisions/0005-consumer-local-truss.md`,
  `0006-private-accepted-envelope-binding.md`,
  `0007-source-and-installed-separation.md` — local-only and source/installed
  boundaries the migration must not widen.
- `.truss/core/docs/WORKFLOW.md` — required durable memory, authority gates, and
  revision-bound proof.
- Private run artifacts: `.truss/delivery/runs/truss-migrate/`.
- Decision record for this run: `.truss/delivery/runs/truss-migrate/decision-record.md`.
- Business analysis: `.truss/delivery/runs/truss-migrate/business-analysis.md`
  (REQ-001 through REQ-038).

## Scope

In scope:

- Dual-root read refusal for `status`, `doctor`, and `addon status`.
- `truss migrate` command surface, preview, classification, transaction,
  verification, rollback, recovery, idempotence, and text/JSON presentation.
- The decision-0008 amendment authorizing migration-only writes.
- Focused tests, integration tests, and the repository gate.

Out of scope:

- Removing legacy read support (a later major with its own record).
- Windows `--apply` support.
- Deleting the retained backup.
- Installer, payload, layout-version, dependency, release, tag, or CI changes.
- Any repository-wide path rewrite or history rewrite.

## Approach

Four serialized waves, one writer per path. Private run artifacts are never
committed; durable authority and this plan are.

1. **Task 0 (Control, this commit)** — amend decision 0008 and bind this durable
   plan as the accepted contract baseline.
2. **Task 1** — read-side dual-root refusal at the application/port boundary.
3. **Task 2** — the complete preview and migration transaction as one coupled
   unit; no mutating subset is committed without rollback and recovery proof.
4. **Task 3** — contract checks and the exact-HEAD repository gate.

## Risks And Recovery

- **Data loss during publish.** Verified retained backup under
  `.truss-migration-backup/<UTC-timestamp>/`, per-phase journal, and reverse
  rollback that restores recorded originals and removes only transaction-created
  paths. `rollback_failed` never claims safety.
- **Interrupted apply.** A valid incomplete journal makes preview report
  `recovery_required`; `--apply` rolls back and re-preflights from the restored
  tree. Post-crash operator edits are fenced, never overwritten.
- **Ambiguous legacy delivery shape.** Refused, not guessed.
- **Running executable under the retired tree.** Linux-only rename/remove after
  verified backup; Windows apply refused.
- **Recovery for this run.** Resume from Git state on branch
  `delivery/truss-migrate`, this plan, and the private run directory; re-verify a
  live dispatch before re-engaging it.

## Progress

- [x] Task 0 — decision 0008 amendment and this durable plan (contract baseline).
- [x] Task 1 — refuse dual-root reads consistently (commit `e438c62`, independently accepted).
- [ ] Task 2 — complete preview and safe migration transaction.
- [ ] Task 3 — contract checks and exact-HEAD repository proof.
- [ ] Independent task acceptance and integration acceptance at final HEAD.

## Decisions

- 2026-09-26: Backup retained indefinitely at
  `.truss-migration-backup/<UTC-timestamp>/` (owner).
- 2026-09-26: Backup receives the fifth relative ignore rule (owner, D-01).
- 2026-09-26: Absent optional entrypoints skipped; present markerless blocks
  (owner, D-03).
- 2026-09-26: Preview prints the backup template; apply prints the exact path
  (owner, D-07).
- 2026-09-26: Apply is Linux-only in this delivery (architect, D-04).
- 2026-09-26: Legacy delivery run-key recognition is narrow and structural
  (architect, D-02).
- 2026-09-26: Migration uses a separate transaction; the core update transaction
  keeps its ownership (architect, D-06).

## Validation

- Focused proof: `cargo test -p truss --test dual_root_refusal --locked`;
  `cargo test -p truss --test migration_lifecycle --locked`;
  `cargo test -p truss --test clean_architecture --locked`.
- Integration or end-to-end proof: migration lifecycle tests covering preview
  no-mutation, the 0.1.16 fixture, backup equality, injected failure and
  rollback per phase, crash retry, collisions, unknown documents, corrupt
  markers, pending sessions, symlinks, text/JSON output, and post-migration
  `status`/`doctor`/`addon status`.
- Repository-required checks: `bash scripts/validate-premerge.sh`.

## Result

Complete after implementation. Record the verified outcome, limitations, and
follow-up before moving this plan to `.truss/authority/plans/completed/`.
