# 0003 Add-On State Ownership, Provenance, And Conflict Recovery

Date: 2026-09-23

## Status

Accepted

## Context

The optional add-ons (`delivery`, `delivery-setup`, `engineering-wisdom`,
`plan-product`) are installed by `scripts/install-truss.sh` and
`scripts/install-truss.ps1` from a manifest list. Nothing else knows about
them.

That leaves four gaps, each with evidence:

- `copy_file` in `scripts/install-truss.sh` skips an existing target unless
  `FORCE=1`, and `--force` backs up and overwrites every selected manifest
  path, including `.truss-core/docs/*` a consumer edited. There is no
  per-file classification, so `--merge` never updates an add-on and
  `--force` clobbers unrelated consumer work.
- `truss update` is core-only. `.truss-core/manifest.json` records 27 core
  paths with `upstream_sha256`, and no add-on path. `grep -RniE
  'delivery|addon|add-on|profile' crates/truss/src --include='*.rs'` returns
  nothing.
- No baseline exists for add-on files, so no tool can separate a consumer
  edit from an upstream change. The core has `.truss-core/base/` plus
  `manifest.json`; add-ons have neither.
- The add-on payload ref is neither recorded nor verifiable. Raw mode fetches
  `$TRUSS_SOURCE_BASE_URL/$relative`, and `--source-git` shallow-clones the
  default branch. Neither pins an immutable identity, and no digest is
  recorded.

On 2026-09-23 the delivery add-on grew from 7 to 17 manifest paths with 3
pre-existing paths changed between `truss-v0.1.12` and the release commit.
The only safe update available was a hand-run manifest-driven copy, applied
into two consumer repositories by an agent. That is an unsupported operation
with no record, no dry run, and no conflict handling.

## Decision

The Rust `truss` CLI owns add-on installed state and add-on updates. The
installers acquire payload bytes and delegate application to the CLI.

Specifically:

1. **One state writer.** All state below `.truss-core/` is written by the
   CLI. Bash and PowerShell acquire an exact payload, validate its
   descriptor, stage it, and invoke one CLI operation that validates, plans,
   merges, applies, records the baseline, and reports. An installer must
   never overwrite an existing managed add-on file directly.
2. **Separate state file.** Installed add-on provenance lives in
   `.truss-core/addons.json` with its own `schema_version`. The core
   `.truss-core/manifest.json` stays at `schema_version: 1`, because
   `InstallationState` rejects any schema version it does not know, so
   raising it would break `truss status` and `truss update` for every
   already-installed consumer before it could self-update.
3. **Separate baseline.** Add-on baselines live under
   `.truss-core/base-addons/<addon-name>/`, not inside the core baseline.
4. **Independently recorded ref.** Each add-on record carries `source_ref`
   and `source_core_version`. The default `source_ref` is the core release
   tag, but equality is not an invariant: an add-on may be installed later
   than its core, a development install records a commit SHA, and
   reinstalling the core must not silently advance an optional add-on. A
   released add-on newer than the installed core is refused unless
   repository authority states compatibility.
5. **Adoption is exact.** A legacy install is adopted only when every
   managed local path already equals the payload bytes from an
   operator-supplied immutable ref. Any mismatch stops. Local bytes are
   never blessed as an upstream baseline.
6. **Scope of refs.** A released source records the exact `truss-vX.Y.Z`
   tag. A git source records the exact commit SHA, never a branch name. A
   dirty local checkout is an explicit development mode that records the
   commit plus a dirty marker and makes no reproducible-provenance claim.
7. **Separate conflict namespace.** Add-on conflict sessions live under
   `.truss-core/addon-update/<addon-name>/`. `.truss-core/update/` stays core
   only, because an older binary reads a session there as a core session.
   `.truss-core/lock` serializes every core and add-on mutation, and one
   active update transaction per repository is allowed.
8. **Core-state prerequisite.** Add-on state operations require a valid
   pre-existing core installation state. Core install and core update
   exclusively own creation and repair of `.truss-core/`,
   `.truss-core/.gitignore`, and `.truss-core/lock`. An add-on operation
   validates those artifacts and refuses without mutation when any is absent,
   invalid, or unsafe. It never creates or repairs them. Authoritative add-on
   state loading, workspace observation, and commit all execute while holding
   the existing shared lock.
9. **The merge owner does not change.** Three-way merge stays with
   `git merge-file -p --diff3` through `GitThreeWayMerge`. No second merge
   implementation is written, in Rust or in shell.
10. **`AGENTS.md` stays outside add-on scope.** No add-on file list includes
    it, and the updater does not inspect, classify, or diagnose the Delivery
    activation wording. That wording is owned by `$delivery-setup` under
    decision 0001. An add-on update must leave `AGENTS.md` byte-identical and
    may report only that the block was not touched.
11. **Cross-platform before publishing.** Both installers advertise the same
    add-on flags, so the update path ships on Bash and PowerShell together,
    or on neither. PowerShell must never fall back to the old skip or force
    behaviour once a shared update option exists.

## Alternatives Considered

1. **Installer-first transactional update (Bash and PowerShell own the
   update).** Rejected. Sharing `git merge-file` shares merge semantics only.
   Planning, state mutation, path validation, transaction ordering,
   conflict-session creation, stale-workspace detection, recovery, and
   provenance-last behaviour would each get three implementations instead of
   one.
2. **Raise `manifest.json` to `schema_version: 2` with an `addons` map.**
   Rejected. Older binaries reject an unknown core schema, which would break
   status and update for existing installs.
3. **Publish add-on payloads as release assets and fetch them like the core.**
   Deferred, not rejected. The installers already obtain payload bytes
   through existing local, raw, and git source modes, so tarball assets are
   not required for a first implementation.
4. **Document the manual manifest copy as the supported procedure.**
   Rejected as a resolution. A manual copy cannot distinguish a consumer
   edit, cannot delete files removed upstream, and records no provenance. It
   remains acceptable only as an emergency runbook.
5. **Force the add-on ref to equal the core version.** Rejected. It makes an
   optional payload's provenance depend on an unrelated core reinstall, and
   it cannot express an add-on installed after its core or a commit-SHA
   development install.
6. **A pinned copy plus baseline with no three-way merge.** Rejected as the
   complete behaviour, because add-on files include instructions and
   templates a consumer may legitimately edit and upstream changed existing
   paths in an observed release. Acceptable only as an internal milestone
   that stops on any local/upstream overlap.
7. **Roll back a state root that the add-on path created.** Rejected. It adds
   cleanup logic around a lock file inside the directory being removed, it
   differs across platforms (notably a Windows lock handle), it must tell
   CLI-created content from core-owned content, and a cleanup failure turns a
   refusal into a recovery state. The root cause is add-on code performing
   core bootstrap it does not own.
8. **Permit the state-root residue on refusal.** Rejected as invariant
   weakening rather than clarification. It would relax both the
   whole-tree-equality requirement and validation-before-write to make an
   implementation pass.

## Consequences

Positive:

- One owner for add-on state, planning, merge, apply, and recovery.
- An add-on can be updated without `--force` clobbering consumer changes and
  without hand copying files.
- Provenance is reproducible and verifiable: an immutable ref plus per-file
  digests.
- Consumers on an older binary keep working: unknown state files are ignored
  and the core schema does not move.

Tradeoffs:

- `truss update` grows a second managed distribution and a second persisted
  namespace.
- Adoption of existing add-on installs requires an operator-supplied ref; it
  cannot be inferred from local bytes.
- Windows parity is a release gate, so the feature cannot ship on one
  platform first.

## Follow-Up

- Implement through `$delivery` as Architectural work, following the approved
  slices in `.truss-core/docs/plans/active/addon-update.md`.
- Amended 2026-09-23: clause 8 was added after the S2 scoped re-review found
  that `FileSystemAddOnState::apply` created `.truss-core/`, patched
  `.truss-core/.gitignore`, and created `.truss-core/lock` before its
  authoritative locked refusal. The add-on path was acting as a second core
  bootstrap owner.
- The payload descriptor publication mechanism is selected during the design
  phase: staged bytes from the existing source modes, with release assets
  deferred.
- Add-on removal is deliberately out of scope. A missing add-on is not
  authorization to delete anything.
- `.truss-core/docs/decisions/README.md` does not index decision documents,
  and neither 0001 nor 0002 is listed there. That remains unchanged rather
  than indexing only 0003.
