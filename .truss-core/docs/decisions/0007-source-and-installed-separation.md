# 0007 Source And Installed Separation

Date: 2026-09-25

## Status

Accepted

## Context

Decision 0005 (amended) accepts a consumer-local Truss in which the installed
payload, the delivery plans, and the delivery-process artifacts are never
published. Applying that to the Truss source checkout exposed the real defect:
**one path is simultaneously the published source and the installed
destination.**

- The embedded distribution reads payload bytes from the same paths that a
  consumer installs into: `crates/truss/src/infrastructure/embedded_distribution.rs`
  registers 28 `include_bytes!`/`include_str!` inputs, including
  `.truss-core/docs/templates/exec-plan.md`.
- `scripts/validate-premerge.sh:11-15` requires every manifest path to exist as
  a file at the repository root, so a destination path is used as a source path
  by the gate.
- The add-on payload is staged from the checkout itself
  (`scripts/install-truss.sh:788-795`), so source path and destination path are
  the same string.
- `.truss-core/docs/` mixes product authority (`ARCHITECTURE.md`, `TRUSS.md`,
  `decisions/**` — none of which is a payload path) with shipped payload
  (`WORKFLOW.md`, `templates/**`, `plans/README.md`).

Because of that conflation, "ignore the installed copy" means "hide the
published source", and the owner's goal is unreachable without separating them.

## Decision

Four layers are made explicit. Only the first is published.

| Layer | Location | Visibility |
| --- | --- | --- |
| Distribution source | `distribution/`, `crates/`, `scripts/`, `tests/`, root `README.md` | tracked |
| Installed output | `.truss-core/**`, `.agents/skills/<truss skill>/**`, `AGENTS.md`, `CLAUDE.md` | local-only in a source checkout |
| Product authority | `.truss/authority/**` | local-only |
| Delivery working memory | `.truss/delivery-runs/**` | local-only |

### 1. Distribution tree

```text
distribution/
├── layout-version                 # declares payload layout 2
├── entrypoints/
│   ├── agent-truss-block.md
│   └── claude-truss-block.md
├── generated.txt                  # destination <- generator declaration
└── payload/                       # mirrors installed destinations exactly
    ├── AGENTS.md                  # absent: generated, see generated.txt
    ├── .agents/skills/...
    └── .truss-core/docs/...
```

`distribution/payload/` mirrors destination paths one to one. A destination that
is synthesized rather than copied is declared in `distribution/generated.txt`
with its generator input, and has no file under `payload/`.

`distribution/layout-version` currently declares `2`. A checkout that declares
an unknown version, or that declares none while its payload is not a valid
pre-refactor layout, fails before any target mutation. A network error while
reading the marker is never treated as evidence of a legacy layout.

### 2. Manifest contract

The manifests stay at `scripts/*-install-files.txt` so published URLs do not
move, and they continue to list **installed destinations**. This is now a
contract, not an accident:

- Every manifest line resolves to exactly one source: a mirrored file under
  `distribution/payload/`, or a generator declared in
  `distribution/generated.txt`.
- Every file under `distribution/payload/` and every generator declaration
  resolves to exactly one manifest line. Orphans in either direction fail the
  gate.
- The resolver never falls back from one layout to another.

### 3. Build inputs

A fresh clone must build, test, and run every gate using tracked files only.
Embedding reads `distribution/payload/**` and `distribution/entrypoints/**`
exclusively. Embedding must never read an installed path, and there is no
fallback to an installed copy.

`AGENTS.md` stays a distribution destination, synthesized as today as a header
plus `distribution/entrypoints/agent-truss-block.md`, but the generator input
moves with the entrypoint.

### 4. Local-only installed outputs in a source checkout

After migration, the source checkout installs Truss into itself through the
ordinary installer in local-only mode, so an agent working in this repository
sees the same installed tree as any consumer. The installed tree is then
excluded:

```gitignore
/.truss-core/
/.truss/
/AGENTS.md
/CLAUDE.md
/.agents/skills/delivery/
/.agents/skills/delivery-setup/
/.agents/skills/encode-invariant/
/.agents/skills/improve-truss/
/.agents/skills/onboard-repository/
/.agents/skills/audit-onboarding-proposal/
```

Skills are excluded individually from the manifest, never as the whole
`.agents/` tree, per 0005.

This repository uses `.gitignore` because its own structure is published by
design; a consumer uses `.git/info/exclude` per 0005. The installer in
local-only mode writes its maintenance-binary rules to the resolved exclude
file, not to a committed root `.gitignore`. Where it does write a committed root
`.gitignore`, the existing Bash and PowerShell skip logic must be reconciled
first: `scripts/install-truss.sh:431` inspects only the two binary rules, while
`scripts/install-truss.ps1:194-197` also counts the marker.

`.truss/` becomes the agent-and-human local area. The CLI never reads or writes
it, and it holds no installation state. Installation state remains under
`.truss-core/` with an unchanged schema, so no state migration, lock change, or
conflict-session change is involved.

### 5. Provenance invariant

> The bytes installed for another user must belong to the recorded
> `source_ref`.

The current check is insufficient. `scripts/install-truss.sh:837-845` combines
`git ls-files --others --exclude-standard`, which **skips ignored files**, with
`git diff --quiet HEAD`, which **cannot see untracked files**. A payload path
that is ignored or untracked therefore stops being verified without any error.

The replacement check resolves each required source path against the recorded
ref itself, for example with `git cat-file -e <ref>:<path>` plus a blob-hash
comparison against the staged bytes, and covers the manifests, the layout
marker, and the generator inputs. A source that is ignored, untracked, missing,
dirty, or different from the ref fails the install before mutation.

### 6. Compatibility

One payload layout is supported: the post-refactor layout. Nothing resolves a
pre-refactor layout.

- The raw base URL mode requires the layout declared by the bootstrap's own
  version. Installing from a pre-refactor tag through the raw base URL fails
  explicitly with a message naming the unsupported layout; it never installs a
  partial payload.
- `--source-git <ref>` remains valid for any ref, including a pre-refactor tag,
  because that ref ships its own bootstrap and its own layout.
- The release gate "released source at tag equals fresh source at the same ref"
  applies to tags created after this refactor. Pre-refactor tags cannot be
  retro-fitted.
- The binary side is unaffected: staged payload trees and manifest destinations
  do not change, so an older binary consumes a newer staged add-on tree.

### 7. Fresh clone

A fresh clone carries no `AGENTS.md`, no `CLAUDE.md`, and no installed skill
until the installer runs. `README.md` is the only tracked entrypoint and must
state the bootstrap order:

```text
clone -> build -> run the installer in local-only mode -> agent guidance and
skills exist
```

No hook, wrapper, or automatic bootstrap is installed. This is an accepted
tradeoff, not an oversight.

### 8. Product authority becomes local-only

`ARCHITECTURE.md`, `TRUSS.md`, `product/installation-profiles.md`,
`decisions/0001-0006`, and `plans/completed/seven-role-delivery.md` move to
`.truss/authority/` and are untracked with `git rm --cached` plus an exclude
rule. Earlier commits keep their content; history is not rewritten.

Payload files that currently sit beside authority files (for example
`.truss-core/docs/product/README.md`) stay payload and move under
`distribution/payload/`.

### 9. Delivery run artifacts

Delivery working memory lives under `.truss/delivery-runs/<run-key>/`, and the
approval receipt under `.truss/authority/approvals/<run-key>.md`, per 0006. This
replaces the `.truss-delivery/runs/<run-key>/plan.md` path declared in 0005 item
4. Both are local-only and neither is ever committed.

## Gate changes

- `scripts/validate-premerge.sh:11-15` verifies source resolution and manifest
  coverage in both directions, instead of requiring each destination to exist
  at the repository root.
- `crates/truss/tests/addon_payload_descriptor.rs:85-120` stops treating the
  repository root as the payload root and reads the resolved or staged payload.
- `tests/s5-rehearse.sh:57,102-110` compares source paths per layout and keeps
  its installed-destination assertions.
- `tests/cli_lifecycle.rs` consumer assertions stay: a consumer install still
  produces `AGENTS.md` and `.truss-core/base/AGENTS.md`.
- `tests/delivery-role-contract.sh` reads the canonical distribution skill
  rather than an installed copy.

## Validation

| Invariant | Positive proof | Counterexample that must be rejected |
| --- | --- | --- |
| Build is independent of the installed layer | Fresh clone with no installed tree builds and passes gates | Embedding or a gate reads an installed path |
| Manifest and source agree | Every destination resolves; no orphans either way | Unmapped destination, orphan payload file, missing generator |
| Source bytes belong to the ref | Staged bytes match the ref blob | Source ignored-untracked, missing, dirty, or manifest edited outside the ref |
| Installed destinations unchanged | Consumer install produces the same tree and provenance | `distribution/` path leaks into a consumer manifest |
| Local-only mode touches no project file | Root `.gitignore` byte-identical; exclude rules idempotent | A tracked or shared entrypoint is silently hidden or edited |
| Installation state is not migrated | Existing `status`, `update`, and conflict continuation still work | Root rename, second lock, lost pending session |
| Layout compatibility | `--source-git` at an old tag installs | Raw mode at a pre-refactor tag installs a partial payload |
| Private approval binding | A correct snapshot and receipt are accepted | Missing or mismatched snapshot, receipt, or handoff is accepted |

PowerShell parity remains static verification only on a host without `pwsh`; no
executable parity is claimed.

## Migration

1. Land 0006 and this record; update the affected guidance and templates.
2. Add the layout marker, `distribution/`, and `generated.txt`; move the payload
   sources; leave destinations untouched.
3. Update embedding, the resolver, the provenance check, and every gate above;
   prove a fresh clone with no installed layer builds and passes.
4. Add the installer local-only mode and reconcile both installers' skip logic.
5. Self-install this repository in local-only mode; adjust root `.gitignore`;
   drop the now-obsolete `.truss-core/docs/plans/*` and
   `.truss-core/docs/decisions/*` rules from `.gitignore:22-27`.
6. Move authority to `.truss/authority/` and untrack it in an owner-approved
   commit.
7. Run the native gate, independent acceptance, and the release rehearsal.

No step may leave the repository unable to build from its tracked files alone.

## Risks

- **Pre-refactor tags stop working through the raw base URL.** Accepted; the
  `--source-git` escape is documented.
- **A fresh clone has no agent guidance until bootstrap.** Accepted; README
  documents it.
- **The installed tree in the source checkout can drift from the payload.**
  Mitigated by self-install through the ordinary installer and by the gate
  reading canonical sources, not installed copies.
- **Two installers must stay in sync.** Mitigated by the rehearsal; PowerShell
  execution remains unverified without `pwsh`.
- **Ignored artifacts are unprotected.** `git clean -fdx` destroys authority and
  run artifacts; 0006 makes retention the owner's explicit responsibility.

## Follow-Up

- Decide the exact `distribution/generated.txt` syntax and the layout-marker
  content during implementation planning.
- Decide whether root `README.md` also documents the local-only exclusion for
  contributors.
- Confirm that no CLI command needs to read `.truss/`.
