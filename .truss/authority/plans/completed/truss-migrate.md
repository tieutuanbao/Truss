# Execution Plan: truss migrate (stage 4 of decision 0008 item 7)

Date: 2026-09-26

## Status

Completed

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
- [x] Task 2 — complete preview and safe migration transaction (commit `644b805`, independently accepted with two Minor findings).
- [x] Task 3 — contract checks and exact-HEAD repository proof (commit `8361a11`).
- [x] Independent task acceptance for Tasks 1 and 2.
- [ ] Final integration acceptance at the exact final HEAD.

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

Completed 2026-09-26 at candidate HEAD `8361a11271c900d3b28dc78388d6bf23071f3c73`
on branch `delivery/truss-migrate`. Local delivery only: nothing was pushed,
merged, tagged, or released.

Verified outcome:

- `truss migrate --directory <repo> [--json] [--apply]` is present, preview-first
  and mutation-free; apply is a verified, backed-up, rollback-capable
  transaction that prints the exact retained backup path.
- `truss status`, `truss doctor`, and `truss addon status` refuse a dual-root
  repository and name the migration preview.
- Decision 0008 carries the accepted migration-only ownership exception, the
  five relative integration ignore rules including `.truss-migration-backup/`,
  the absent-optional entrypoint rule, the Linux-only apply limit, the preview
  backup template, and the narrow legacy delivery run-key shape.
- Evidence: `bash scripts/validate-premerge.sh` exited 0 printing
  `pre-merge validation passed`; focused suites `migration_lifecycle` (14),
  `clean_architecture` (3), and `dual_root_refusal` (4) passed; delivery-role
  contract 22 ok / 0 failed; payload-layout contract 26 ok / 0 failed.
- Independent acceptances: Task 1 `ACCEPT`, Task 2 `ACCEPT`, both by fresh
  read-only sessions; the Task 2 session observed all four rows the implementer
  had left un-red on a scratch copy and drove the real binary end to end.

Limitations recorded rather than hidden:

- Windows `--apply` is unsupported in this delivery (D-04). Preview reports
  `blocked` with reason `unsupported_apply_platform`; no Windows lane exists and
  no prose claims one.
- `rollback_failed` at the process boundary is proven by a unit test and by the
  Task 2 acceptance session, not by the shipped binary in the lifecycle suite,
  because no portable fixture provokes the restore I/O error.
- Minor (accepted, no remediation required): the two REQ-010 unit fixtures skip
  the backup root in their snapshot helper, so residue limited to that root is
  discriminated only by the process-level collision test.
- Minor (accepted): the unknown-run-key token rewrite falls back to
  `.truss/delivery/runs/evidence/<rest>`, which can leave wrong-shape
  *references* (never misfile files) when no delivery root exists.
- Applying to a dual root whose new tree carries a different manifest is the
  contract's honest byte-different collision refusal, not a fast-forward; a real
  consumer in that state needs an operator decision.

Follow-up (not authorized here):

- Publishing, release, and tag are out of scope. Window apply support and
  legacy-read removal require their own approved records.
- The retained backup under `.truss-migration-backup/` is never auto-deleted.

### Post-acceptance remediation (2026-09-26)

The first integration acceptance returned ACCEPT at `a0d36a2`, but Control's own
post-acceptance demonstration found a Blocking defect the acceptance had missed:
`--apply` published every file through a byte copy that created destinations with
default permissions, so `.truss/core/bin/truss` became non-executable (755 → 664)
and the retained backup lost the bit too, meaning a rollback would have restored
bytes without the mode. A legacy install would migrate into a tree whose own CLI
could not run.

- Remediation commit `40e76b5` makes the filesystem adapter copy bytes and Unix
  permission bits together at every migration byte boundary: retained backup,
  stage, publication, integration writes, and every rollback restore.
- Discriminating instruments added and observed red at `a0d36a2`:
  `filesystem_migration::tests::migration_preserves_executable_bits_and_restores_them_on_rollback`
  and `migration_lifecycle::migrated_executables_keep_their_permission_bits`.
- Control's independent reproduction after the fix: one executable before
  apply, `.truss/core/bin/truss` at `755` after, the backup copy at `755`, and
  the migrated binary running and reporting `current`.

The earlier Task 2 and integration verdicts are invalidated by this mutation and
must be re-earned by a fresh acceptance at the new exact HEAD.

### Findings recorded, not absorbed

- **Out of scope for this delivery — installer payload-to-install mode loss.**
  `distribution/payload/.agents/skills/onboard-repository/scripts/*.py` are
  `100755` in Git, but an installed tree carries them as `664`, so the installer's
  byte copy loses the bit on a fresh install. Migration is now faithful to the
  legacy tree it reads; the installer path is forbidden scope here and needs its
  own bounded change.
- **Minor — directory modes are not preserved.** File modes are preserved; a
  legacy directory with a non-default mode is recreated with defaults.
- **Minor — the transaction `stage/` directory is retained after a committed
  apply,** so the backup directory holds a second full copy of the published
  tree. Correct but costly; a later change may drop the stage on commit.
- **Minor — `absent_optional` is not surfaced** in the preview output
  (integration acceptance note).
- **Minor — a stale `(plan 4B …)` parenthetical** remains in an infrastructure
  diagnostic outside this delivery's scope.
- **Minor — REQ-010 unit fixtures skip the backup root** in their snapshot
  helper, so residue limited to that root is discriminated only by the
  process-level collision test.
- **Minor — unknown-run-key token rewrite fallback** to
  `.truss/delivery/runs/evidence/<rest>` can leave wrong-shape references (never
  misfile files).

### Post-acceptance remediation 2 — flaky running-binary test (2026-09-26)

The post-remediation acceptance returned ACCEPT at `1753e32`, but Control then
observed that `crates/truss/tests/migration_lifecycle.rs::a_copied_running_binary_applies_and_retires_its_own_tree`
failed intermittently under the default parallel test threads with
`ExecutableFileBusy` ("Text file busy") at the exec of a freshly copied binary:
1 failure in 6 full-suite runs, 0 in 13 isolated or single-threaded runs. That
made `bash scripts/validate-premerge.sh` intermittently red with no code change,
so the repository gate was not reproducibly green.

- Remediation commit `571bf82` routes both execs of a freshly written binary in
  that test through one bounded retry on `ErrorKind::ExecutableFileBusy`. It
  touches only the test file, changes no product code, and weakens no assertion.
- Evidence: before the change, 1 of 20 default-parallelism runs failed with
  `Text file busy`; with the retry instrumented, 45 runs absorbed 5 real busy
  execs with 0 failures; Control re-ran the full file suite 12 times at
  `571bf82` with 0 failures.
- This is test-fixture interference on Linux, not a product race: the busy state
  comes from the test's own concurrent copy/exec of the fixture binary, and a
  real consumer runs the migrated binary after the migration process has exited.

The earlier verdicts at `1753e32` are invalidated by this mutation and must be
re-earned by a fresh acceptance at the new exact HEAD.
