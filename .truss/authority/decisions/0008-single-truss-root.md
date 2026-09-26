# 0008 Single Truss Root

Date: 2026-09-25

## Status

Accepted

## Context

Truss currently occupies two repository-root paths in a consumer: `.truss-core/`
for the installed payload and the CLI-owned installation state, and `.truss/` for
the local-only delivery working memory and product authority introduced by
decisions 0006 and 0007. The owner's goal is one Truss-owned folder, so a project
does not accumulate parallel Truss directories at its root.

The change is not a directory rename. `.truss-core/` is both an installed
destination prefix and the installation state root, so renaming it changes the
installed contract:

- **13 manifest destinations** begin with `.truss-core/`
  (`scripts/truss-install-files.txt`).
- **780 occurrences across 86 files** name `.truss-core`, including 342 in
  `crates/`, 170 inside the installed tree itself, 116 in `scripts/`, 54 in the
  shipped payload text, 39 in `tests/`, 32 in the skills, and 4 in the entrypoint
  block that generates `AGENTS.md`.
- `state_root()` hardcodes `root.join(".truss-core")`
  (`crates/truss/src/infrastructure/state_io.rs:19-21`), and the installer's
  local-source sentinel requires `.truss-core/docs/TRUSS.md`
  (`scripts/install-truss.sh:1064`).
- Consumers that already installed Truss have the payload and state at the old
  paths, so a new layout creates a second tree beside the first.

Verified while deciding: the CLI tolerates unknown entries under the state root.
No code enumerates the state root and rejects unexpected entries; `read_dir` is
used only on specific directories (`crates/truss/src/infrastructure/addon_apply.rs:530`,
`addon_session.rs:532`, `addon_state.rs:524`, `state_io.rs:195`), the core
rollback path touches only `manifest.json` and `base/`
(`crates/truss/src/infrastructure/transaction.rs:78-92`), and
`ensure_state_ignore` appends missing rules without rejecting extra ones
(`state_io.rs:36-60`). That fact is what makes a staged migration possible.

## Decision

### 1. One root, three namespaces

```text
.truss/
├── core/                     # installed payload and CLI-owned state
│   ├── bin/truss
│   ├── manifest.json  addons.json  lock  transaction.json
│   ├── base/  base-addons/  update/  update-candidate/  addon-update/
│   └── docs/                 # WORKFLOW, communication, templates, READMEs
├── delivery/                 # per-run working memory
│   ├── runs/<run-key>/{plan.md,approved-envelope.md,prompt-*.md,report-*.md}
│   └── approvals/<run-key>.md
└── authority/                # product authority, local-only
    ├── architecture/  decisions/
```

`.truss-core/` is replaced by `.truss/core/`. The installed destination for every
existing `.truss-core/...` path becomes `.truss/core/...`, for all 13 manifest
destinations and for every state path: `bin`, `manifest.json`, `addons.json`,
`lock`, `transaction.json`, `base/`, `base-addons/`, `update/`,
`update-candidate/`, `addon-update/`, and `docs/`.

Keeping a `core/` level, rather than placing `docs/` directly under `.truss/`, is
deliberate: it separates the CLI-owned installed tree from the two
human-and-agent-owned namespaces at one visible level instead of mixing payload
with `bin/`, `base/`, and the lock.

### 2. Ownership boundaries

- The CLI writes only under `.truss/core/**`. It never reads or writes
  `.truss/delivery/**` or `.truss/authority/**`.
- The delivery skill writes only under `.truss/delivery/**`. Control owns those
  artifacts; delivery does not delete them on its own (0006 item 6).
- Humans and agents write `.truss/authority/**` and the consumer-owned parts of
  `.truss/core/docs/**` that are outside the managed manifest, exactly as they
  write `.truss-core/docs/plans/active/**` and `decisions/**` today.

### 3. Delivery artifact paths

- Run artifacts: `.truss/delivery/runs/<run-key>/`, holding the mutable `plan.md`,
  the immutable `approved-envelope.md`, and the dispatch prompts and reports.
- Approval receipt: `.truss/delivery/approvals/<run-key>.md`.

These supersede the `.truss/delivery-runs/<run-key>/` and
`.truss/authority/approvals/<run-key>.md` paths of decision 0006 items 3 and 4,
the `.truss/delivery-runs/<run-key>/plan.md` path of 0005 item 4 as amended, and
the path recorded in 0007 §9. The receipt stays under `delivery/` rather than
`authority/` because it belongs to one run, not to the durable product authority.

### 4. Ignore contract

A consumer that keeps Truss in version control commits `.truss/core/` and ignores
the two local-only namespaces:

```gitignore
.truss/core/
.truss/delivery/
.truss/authority/
```

A consumer running Truss local-only uses a single rule:

```gitignore
/.truss/
```

The pattern belongs in the repository-root `.gitignore` or in the resolved
`.git/info/exclude`, per 0005. Patterns placed in `.truss/core/.gitignore` cannot
reach a root path, because patterns there resolve relative to `.truss/core/`.

### 5. Machine-local log

`~/.truss/delivery-log` keeps its path and its meaning: a machine-local, opt-in
line log that is never committed and never read for routing, recovery, or runtime
decisions. `references/maintenance-log.md` is not edited for this decision.

The two `.truss/` locations are therefore distinguished only by scope — a home
directory path is machine local, a checkout path belongs to one repository — and
this record is where that distinction is written down. No code or guidance may
treat `~/.truss/` and `<repo>/.truss/` as related.

### 6. Layout version and compatibility

`distribution/layout-version` becomes `3`. Nothing resolves a layout-2 payload at
runtime.

| Combination | Contract |
| --- | --- |
| old binary + old root | unchanged; old tags stay immutable |
| new binary + old root | reads the old root, keeps writing the old root, and reports that a migration is available |
| new binary + new root | normal operation |
| old binary + new root | refused with an explicit message that the installation uses a newer layout |
| old root and new root present together | refused; the operator runs `truss migrate` or explicitly adopts one tree |
| raw base URL at a layout-2 tag | refused with an unsupported-layout message; `--source-git` at that tag still works, because that ref ships its own bootstrap |
| `truss migrate` | transactional, backed up, rollback on failure, and refused while an unresolved conflict session exists |

### 7. Staging

1. This record; no code.
2. Read both roots: the new code resolves the new root first and falls back to the
   old one, and writes stay at the old root.
3. Write the new root: new installs use `.truss/core/**`, the shipped payload text
   carries the new paths, and the installer preflight checks `layout-version`
   before any mutation.
4. `truss migrate`: move state transactionally, with backup, refusal conditions,
   and rollback.
5. Remove legacy read support in a later major, with its own record.

Shipped prose is part of the change: 54 lines in `distribution/payload/**` and 4
in `distribution/entrypoints/agent-truss-block.md` name `.truss-core` and must
name `.truss/core` instead. Each release tag is self-consistent; older tags are
never retro-fitted.

## Alternatives Considered

1. **Keep `.truss-core/` and place the delivery artifacts inside it**
   (`.truss-core/runs/`, `.truss-core/approvals/`). Rejected by the owner: it
   reaches one folder without changing the installed contract, but it keeps the
   `.truss-core` name and mixes agent-written artifacts into a CLI-owned tree
   without a namespace boundary that a reader can see.
2. **`.truss/docs/**` instead of `.truss/core/docs/**`.** Rejected: it puts the
   installed payload at the same level as `bin/`, `base/`, and the lock, so one
   directory level carries both payload and state and neither is identifiable by
   name.
3. **Keep the two roots of decision 0007.** Rejected: it is the arrangement the
   owner asked to replace, and it requires two ignore decisions instead of one.
4. **A symlink `.truss/core -> ../.truss-core`.** Rejected: it changes no
   contract while pretending to, and the code already rejects symlinks on
   managed state files (`reject_symlink` on the shared `.truss-core/.gitignore`),
   so the tree would not behave like a real installation.
5. **Rename without a migration path.** Rejected: every existing consumer would
   get a second tree, an independent lock, and a stale baseline.

## Consequences

Positive:

- One Truss-owned folder per project, with one visible boundary per owner: CLI,
  delivery, authority.
- The consumer ignore contract needs three rules, or one for a local-only
  installation, instead of two roots with separate rules.
- Naming becomes consistent with the namespace the guidance already uses for
  local-only artifacts.

Tradeoffs:

- The installed contract changes for every consumer, and existing installs need
  `truss migrate` or an explicit adoption decision. Earlier commits keep their
  old paths; history is not rewritten.
- The change touches 780 references across 86 files, 13 destinations, the state
  root, the installer sentinel, self-update, the release rehearsal, and the
  shipped prose.
- A compatibility window with two readable roots adds code that later has to be
  removed, and it makes "which root am I operating on" a question every command
  must answer.
- `~/.truss/` and `<repo>/.truss/` share a name. The scope difference is recorded
  here, not in the delivered log reference, so a reader who only reads
  `maintenance-log.md` will not learn it.
- Layout 2 becomes unreachable through the raw base URL, as 0007 already accepted
  for layout 1.

## Follow-Up

- Amend 0005 item 4, 0006 items 3 and 4, and 0007 §4, §8, and §9 to the paths in
  items 1 and 3 above.
- Re-scope the distribution layout work: the mirror under
  `distribution/payload/**` must use the new destination strings, which is the
  branch keeping this decision's plan unmerged.
- Write the implementation plan: dual-root resolution, new-root writes, shipped
  prose, `truss migrate`, both installers, the rehearsal, and the gates.
- Record `truss migrate`'s refusal conditions and rollback proof before it ships;
  a migration without a proven rollback is not accepted.

## Amendment

Date: 2026-09-25. Amended by owner decision after the first consumer migration
(MeBe). Items 2 and 4 are corrected; the rest stands.

Corrected ownership boundaries:

- `.truss/core/**` is the Truss-managed payload: templates, workflow,
  constraints, and rules. Truss installs and replaces it on update. A project
  never edits it as authority.
- `.truss/authority/**` in a consumer is the authority of **that project** —
  its decisions, plans, and product documents — written by that project's own
  humans and agents, and **never committed**. Truss ships no content into it,
  creates no file inside it, and never writes it.
- `.truss/delivery/**` stays local-only per 0006, written by delivery Control.

Corrected ignore contract — two rules, both relative, neither root-anchored:

```gitignore
.truss/delivery/
.truss/authority/
```

A consumer running Truss local-only ignores `.truss/` with a single rule.
Root-anchored spellings of any Truss ignore rule are withdrawn: every Truss
namespace is written relative to the project root, and a rule anchored with a
leading slash describes a different location than the one Truss uses.

Truss product authority (its own `TRUSS.md`, `ARCHITECTURE.md`, decisions)
stays in the Truss source repository and is never part of the installed
payload.

## Amendment

Date: 2026-09-26. Amended by owner decision to accept `truss migrate` (stage 4
of item 7) and to record its operator-visible limits.

### Migration-only ownership exception

The `truss migrate` command may write `.truss/authority/**` and
`.truss/delivery/**` **only** as operations enumerated in an immutable
migration plan derived from recognized legacy roots. This exception does not
extend install, update, status, doctor, self-update, or add-on ownership.

- Migration is preview-only unless `--apply` is supplied. Preview performs no
  filesystem mutation: no create, no write, no lock, no backup directory, no
  timestamp.
- Unknown inputs, ambiguous delivery run identity, unsafe paths, pending
  sessions, byte-differing collisions, and missing or broken entrypoint markers
  refuse before mutation.
- Apply uses a separate journal and a verified retained backup under
  `.truss-migration-backup/<UTC-timestamp>/`. It merges without replacing
  pre-existing namespace contents and retires legacy roots only after
  destination and integration verification.
- Rollback restores only recorded originals, removes only transaction-created
  paths, fences post-crash edits, and must not claim safety when restoration is
  incomplete.
- The migration transaction is separate from the core update transaction,
  which keeps its current ownership unchanged.

Ordinary CLI ownership stated earlier in this decision is otherwise unchanged.

### Integration ignore rules

The managed entrypoint integration rules are the relative paths
`.truss/authority/`, `.truss/delivery/`, `.truss/core/bin/truss`, and
`.truss/core/bin/truss.exe`. `.truss-migration-backup/` is **additionally
ignored** as retained local recovery material; it is not a managed namespace
and is never auto-deleted. Root-anchored spellings of any Truss ignore rule
remain withdrawn.

### Operator-visible limits of this delivery

- An absent `AGENTS.md` or `CLAUDE.md` is skipped as `absent_optional`; a
  present file with zero, duplicate, broken, or out-of-order
  `TRUSS:BEGIN`/`TRUSS:END` markers refuses before mutation. Migration never
  creates an optional mixed file.
- Apply is Linux-only in this delivery. A Windows host preview reports
  `blocked` with reason `unsupported_apply_platform`, and `--apply` exits
  non-zero before lock or mutation. Windows apply requires a later approved
  design and a real proof lane.
- Preview prints the deterministic backup template
  `<repo>/.truss-migration-backup/<UTC-timestamp>/` with no concrete
  timestamp; `--apply` freezes one UTC timestamp in the journal and prints the
  exact created path.

### Recognized legacy delivery shape

Run-key recognition is intentionally narrow: `.delivery/<run-key>/**` and
`.delivery-dispatch/<run-key>/**` with exactly one direct child directory per
non-empty root, and byte-identical child names when both roots are non-empty.
Files directly under a legacy root, multiple direct children, differing child
names, symlinked children, and nested shapes are refused rather than guessed.
