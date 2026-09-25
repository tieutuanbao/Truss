# Execution Plan: Add-On Update Path

Date: 2026-09-23

## Status

Active

## Outcome

A consumer that has an optional add-on installed can update it to an
immutable ref without `--force` clobbering core or consumer edits and without
hand copying files. Both installers reach the same result through one state
writer, every applied change is verifiable against a recorded digest, and
anything ambiguous stops before mutation.

## Context

- Decision: `.truss-core/docs/decisions/0003-add-on-state-ownership.md`
- Installers: `scripts/install-truss.sh`, `scripts/install-truss.ps1`
- Manifests (membership owner): `scripts/*-install-files.txt`
- Core state and planner: `.truss-core/manifest.json`,
  `.truss-core/base/`, `crates/truss/src/infrastructure/filesystem_state.rs`
- Merge owner: `crates/truss/src/infrastructure/git_merge.rs`
  (`git merge-file -p --diff3`)
- Embedded core payload: `crates/truss/src/infrastructure/embedded_distribution.rs`
- Release identity: `scripts/truss-release-tag`,
  `crates/truss/src/infrastructure/release_handoff.rs`
- Observed friction: the delivery add-on went from 7 to 17 manifest paths with
  3 pre-existing paths changed between `truss-v0.1.12` and the release commit;
  the only safe update was a hand-run manifest copy into two consumers.

## Scope

In scope:

- Add-on payload descriptor: name, immutable ref, source core version, ordered
  path set, per-file SHA-256.
- Installed add-on record in `.truss-core/addons.json` and baseline under
  `.truss-core/base-addons/<name>/`.
- CLI operations to install, update, and report status for a named add-on,
  reusing the core planner and transaction code.
- Add-on conflict sessions under `.truss-core/addon-update/<name>/`.
- Installer delegation on Bash and PowerShell.
- Documentation and release proof.

Out of scope:

- Add-on removal or uninstall.
- A third-party add-on registry, inter-add-on dependency resolution, or
  background auto-update.
- Any change to `$delivery-setup` ownership of the `AGENTS.md` Delivery block.
- A second merge implementation, or reaching into `.truss-core/update/`.
- Raising the core `manifest.json` schema version.
- Release tarball assets for add-ons (deferred; see the decision record).

## Approach

Six slices, each independently reviewable. The CLI comes first so that no
shell script ever needs to plan or merge.

### S1 — Payload descriptor and provenance model

Derive a descriptor from the existing manifest: `name`, immutable `source_ref`,
`source_core_version`, ordered path set, per-file SHA-256. The manifest stays
the membership owner; no second hand-maintained path list.

- Instrument: build the descriptor from a fixture manifest, enumerate both
  path sets with `comm`, hash every payload file and compare.
- Counterexample: a descriptor that lists every path but hashes the target
  workspace files instead of the payload bytes still passes path completeness,
  and must fail once one target file is locally modified.
- Also required: `cat scripts/*-install-files.txt | sed '/^#/d;/^$/d' | sort |
  uniq -d` prints nothing, and a descriptor whose paths overlap the core or
  another add-on is rejected.

### S2 — Installed add-on record and exact adoption

Write `.truss-core/addons.json` (its own `schema_version`) and baseline copies
under `.truss-core/base-addons/<name>/`. Record after a successful apply.

The approved rows, after the 2026-09-23 amendment:

Row 1
- Requirement: a successful install writes the record after the files exist,
  and the record matches the payload.
- Instrument: in a test workspace, run the install path, then reload the record
  and compare every baseline hash with the staged payload bytes, and confirm
  every managed path exists with the recorded digest.
- Counterexample: a writer that records hashes of the target workspace files
  instead of the payload bytes passes a "record exists and files present"
  check, and silently bakes a consumer edit into the baseline.

Row 2
- Requirement: a legacy install adopts only on exact match.
- Instrument: two fixtures — one whose files exactly equal the supplied ref
  (must adopt), one with a single edited byte (must stop).
- Counterexample: an implementation that adopts whenever the managed paths
  merely exist accepts a consumer-edited file as upstream.

Row 3
- Requirement: a refusal performs no CLI-owned mutation. An add-on operation
  requires a valid pre-existing core state root and shared lock; an absent or
  invalid core state is refused without creating or changing `.truss-core`.
  When preflight passes but the workspace changes before lock acquisition,
  the authoritative locked observation refuses and preserves the workspace
  exactly as changed by the competing writer, with no additional add-on state,
  baseline, or managed-file mutation.
- Instrument: use two deterministic fixtures. First, call add-on apply with no
  core state and compare the complete workspace path/type/content snapshot
  before and after; they must be byte-identical and `.truss-core` must remain
  absent. Second, begin with valid core state, force preflight to pass, then
  use a test barrier to change one managed path before existing-lock
  acquisition. Snapshot the complete workspace after that competing change,
  allow locked observation to continue, and require refusal. The final
  snapshot must equal the post-change snapshot; `addons.json`,
  `base-addons/<name>/`, and every other CLI-owned output must remain absent.
- Counterexample: an implementation that creates `.truss-core`, patches
  `.truss-core/.gitignore`, or creates `.truss-core/lock` before validating the
  core-state prerequisite changes the first fixture on refusal. An
  implementation that treats preflight as authoritative overwrites or blesses
  the competing edit in the second fixture.

This corrected Row 3 replaces the earlier instrument, which hashed the fixture
before and after a one-edited-byte attempt and therefore only ever exercised a
refusal during `preflight`. It never forced the preflight-pass followed by
locked-refusal interleaving that the S2 scoped re-review found.

### Slicing and sizing rules

Learned from S1 and S2, both of which needed a second review round for a
reason the contract could have prevented:

1. **Split on independent review, not on labels.** Create a separate task only
when the unit has its own test cycle and a reviewer could accept it while
rejecting its neighbour. Keep an inseparable state transition, its
synchronization, and its commit semantics together; separate independently
reviewable schema or model work from filesystem apply.
2. **No numeric diff ceiling.** Before dispatch, enumerate the expected files
and the one test cycle; split when the task cannot fit one worker session,
needs more than one independent acceptance cycle, or a reviewer could accept
one behaviour while rejecting another. Diff size is a warning, never a gate.
3. **Preconditions and ownership are part of the contract.** Name every
prerequisite artifact and its owner, and state whether the task may create,
validate, repair, or only consume it. Missing authority returns `NEEDS_REPLAN`.
4. **Every row names its plausible present-but-wrong counterexample.** When
correctness depends on ordering, concurrency, recovery, deletion, or a failure
boundary, the instrument must exercise that boundary deterministically.
5. **Every row supplies one exact command the reviewer can rerun** on the
candidate revision. When that command emits intermediate evidence a second
command consumes, the evidence must outlive the producer at a stable
documented locator.
6. **Name a nontrivial refactor in Direction.** Split it when the extraction has
independent acceptance and materially expands the regression surface; allow a
small inseparable extraction when the behaviour cannot be implemented cleanly
without it.
7. **Review the whole candidate.** The reviewer reproduces the declared rows and
the closure gate and also inspects the complete candidate for regressions, data
or security risk, and owned-scope violations. A finding outside the declared
rows is classified normally and never suppressed: if it exposes missing
authority or ambiguity, Control routes to replan; if it is an implementation
defect under existing invariants, it is remediated normally.

### S3a — Extract a pure reusable planner, behaviour-preserving

Extract the classification matrix behind the core path into a pure planner the
add-on adapter can consume. **Only do this once the adapter's concrete shape is
known**, so the extraction is not speculative.

Preconditions and ownership: may restructure the core planner; must not change
any classification result, any conflict rule, or the core `manifest.json`
schema.

- Instrument: for every existing core matrix tuple, the old and the extracted
  planner produce the same change kind, conflict reason, mutations, and frozen
  file set, and the existing core suites pass unchanged.
- Counterexample: a modified upstream removal becomes `Delete` instead of the
  modified-removal conflict; the equivalence table rejects it.

### S3b — Add-on planner adapter: the complete matrix, dry run only

One table over the whole matrix: create, preserve, update, clean merge,
overlapping conflict, missing managed file, existing unmanaged path, clean
deletion, modified deletion, and an already-missing removed path. No workspace
apply in this slice; the dry run reports the plan only.

Preconditions and ownership: uses the S2b state contract (existing core state,
existing lock) and the S1 descriptor; must not apply, must not persist a
conflict session, must not create core state.

- Instrument: the table matches the expected kind, conflict, and mutation for
  every tuple, and a workspace snapshot before and after the dry run is
  identical.
- Counterexample: a planner that deletes a consumer-modified removed file, or
  that adopts an existing unmanaged path; the table rejects both.

### S3c — Transactional apply of a conflict-free plan

Apply a complete conflict-free plan atomically: writes, deletions, baseline
update, and `addons.json` provenance last. Conflict sessions stay out of this
slice.

Preconditions and ownership: consumes the S3b plan; must not stage or resume a
conflict session, which S4 owns.

- Instrument: an injected failure after the first staged mutation leaves the
  workspace, the baseline, and `addons.json` byte-identical to before; a
  successful apply writes every change and provenance last.
- Counterexample: the first file is visible before a later write fails, so the
  update is partial; the fault-injection snapshot rejects it.

### S4 — Scoped conflict recovery

Persist a conflicted add-on plan under `.truss-core/addon-update/<name>/`,
freeze every observed file, and provide resume and abort. Add-on sessions are
add-on scoped and never touch `.truss-core/update/`, which stays core-only.

Preconditions and ownership: requires valid existing core state and the
existing shared lock and must not create or repair them; must not read, write,
or clear a core session; `AGENTS.md` stays byte-identical.

- Instrument: stage a plan with one conflict and one clean change; read back the
  session and require the workspace, baseline, and `addons.json` unchanged; then
  change an unrelated frozen managed path and resume, which must refuse with the
  competing bytes intact; then abort, which must remove only the owned session
  and be idempotent.
- Counterexample: a resume that validates only the conflicted path and applies
  after another planned path changed, or an abort that clears
  `.truss-core/update/` or deletes managed files.

### S4b — Superseded by S4b1-S4b3

`BLOCKED` before any mutation: the approved argument shape below assumed a
self-contained resume that S4 did not provide, and it assumed the interface
layer could reach the add-on use cases, which the clean-architecture boundary
forbids. The command shape is retained as the target for S4b3; the slice is
replanned into S4b1, S4b2, and S4b3 after decision 0003 clause 12.

Target shape, to be published by S4b3:
truss addon status   --name <n> [--directory <d>] [--json]
truss addon install  --name <n> --manifest <path> --source <dir> --source-ref <ref>
                     [--source-core-version <v>] [--directory <d>] [--dry-run] [--json]
truss addon update   --name <n> --manifest <path> --source <dir> --source-ref <ref>
                     [--source-core-version <v>] [--directory <d>] [--dry-run] [--json]
truss addon continue --name <n> [--directory <d>] [--json]
truss addon abort    --name <n> [--directory <d>] [--json]
```

`--manifest` stays the membership owner and `--source` holds the staged bytes;
the descriptor is built from both by the S1 builder, so no new descriptor file
format is invented. `--source-ref` must be an immutable ref: a tag or a commit
SHA, never a branch name. `continue` reads the resolutions the operator edited
under the session's `resolved/` directory, the same convention the core uses.

The CLI layer stays thin: argument mapping and presentation only. It calls the
existing application use cases and must not re-implement classification, apply,
or session behaviour.

Preconditions and ownership: valid existing core state and the existing shared
lock; no creation or repair of `.truss-core/`, its `.gitignore`, or the lock;
`.truss-core/update/` untouched; `AGENTS.md` byte-identical; dry run mutates
nothing and never applies a plan carrying a conflict; existing core commands
keep their behaviour.

- Instrument: a CLI integration test over a temporary workspace with valid core
  state that runs the full cycle — status with no record, install at one ref,
  `update --dry-run` to a second ref, `update`, status again, then abort a
  seeded conflict and continue a resolved one — asserting the evidence at each
  step and snapshot equality wherever a command must not mutate.
- Counterexample: a command that reports a plan but applies anyway under dry
  run; a command that ignores `addons.json` and re-derives the baseline from the
  payload, so every managed file looks consumer-modified; an abort that clears
  `.truss-core/update/`; an update that applies the clean subset of a conflicted
  plan.

### S4b1 — Self-contained frozen conflict session

Owned paths:

```
crates/truss/src/domain/update_plan.rs
crates/truss/src/application/addon_apply.rs
crates/truss/src/infrastructure/addon_session.rs
crates/truss/src/infrastructure/addon_apply.rs
crates/truss/tests/addon_session.rs
```

Outcome: session schema 2 stores the complete candidate, the descriptor
identity, the frozen plan, the conflict inputs, the resolutions, and the frozen
observations. Resume takes `root + AddOnName` only and never re-plans. A schema
1 session refuses `continue` safely and still permits `abort`.

Layout:

```
.truss-core/addon-update/<name>/
  session.json                 schema 2, identity plus path lists
  candidate/<managed-path>     every descriptor path, payload bytes
  base/ local/ incoming/ resolved/   per conflict path
  frozen/<managed-path>
  plan.json                    frozen changes, conflicts, clean mutations
```

- Instrument: one deterministic test stages a plan with one clean write, one
delete, one preserve, and one conflict; removes the external payload directory;
edits `resolved/`; resumes successfully; then verifies the workspace, the
complete baseline, provenance, and owned-session removal. A schema 1 fixture
must refuse `continue` and permit `abort`.
- Counterexample: a session that stores conflict paths only passes
conflict-resolution tests but loses the clean write, or classifies an omitted
path as a deletion. Removing the external payload before `continue` makes that
implementation fail.

### S4b2 — Application boundary and infrastructure implementations

Owned paths:

```
crates/truss/src/application/addon_application.rs
crates/truss/src/application/addon_apply.rs
crates/truss/src/application/addon_plan.rs
crates/truss/src/application/ports.rs
crates/truss/src/application/mod.rs
crates/truss/src/infrastructure/addon_apply.rs
crates/truss/src/infrastructure/addon_plan.rs
crates/truss/src/infrastructure/mod.rs
crates/truss/tests/addon_application.rs
crates/truss/tests/clean_architecture.rs
```

Outcome: `AddOnPlanPort` and `AddOnExecutionPort` (`apply`, `stage`, `resume`,
`abort`, `session_pending`) plus an `AddOnApplication` facade expose
status/install/update/continue/abort with no interface-to-infrastructure import.
`session_pending` is add-on scoped and must not overload core
`resolution_pending`.

- Instrument: application tests with fake ports verify orchestration, dry-run
  non-mutation, conflict staging, frozen-session continue, scoped abort, and
  status pending or absent. `cargo test --test clean_architecture` stays green.
- Counterexample: a facade that reads `.truss-core/addon-update/` directly, or
  that asks core `resolution_pending`, reports the wrong namespace; the
  fake-port call log and the architecture test reject both.

### S4b3 — CLI surface, presentation, and composition wiring

Owned paths:

```
crates/truss/src/interface/cli.rs
crates/truss/src/interface/presenter.rs
crates/truss/src/main.rs
crates/truss/tests/cli_lifecycle.rs
crates/truss/tests/clean_architecture.rs
```

Outcome: the target commands are published. `continue --name` stays payload free
because S4b1 made the session self-contained. The interface maps arguments and
presents application reports only. `main.rs` is a wiring-only edit: it parses,
constructs concrete adapters, calls the interface, and writes output. It may not
carry path rules, payload construction, session logic, classification, apply
decisions, or presentation formatting.

- Instrument: a CLI integration test performs absent status, install ref A,
  status, dry-run update to ref B with byte-identical snapshots, real update, a
  staged conflict, an edited `resolved/`, a payload-free continue, and a scoped
  idempotent abort. Existing core CLI tests stay unchanged and green.
- Counterexample: CLI code importing infrastructure or opening session paths
  directly fails `clean_architecture`; `continue` requiring source arguments
  fails the parser test; a dry run that mutates fails snapshot equality.

### S4b4 discovery record — real-payload smoke test

A real-payload smoke test found the defect that made this plan's own outcome
unreachable: installing the real delivery add-on into a workspace with the core
installed was refused, because `collect_extra_managed_paths` took `.agents` as a
scan root and counted every core skill directory as an undeclared add-on managed
path. Decision 0003 clause 13 fixed the semantics and the slice landed as
`f5dbbc2`. Control then reproduced the outcome by hand: with a real core install
and the real 17-path payload, `addon install` at the real location returns
"Add-on delivery truss-v0.1.13 installed (adopted=false)" at exit 0, and a
pre-created `.agents/skills/delivery/stray.md` still refuses at exit 1 naming
that file.

The slice review then returned `CHANGES_REQUESTED` on one Important finding:
`crates/truss/src/interface/cli.rs` passes `foreign_manifests: &[]` on both the
real install and update paths, so the descriptor-overlap check runs against an
empty ownership set and an exact cross-add-on collision could overwrite another
add-on's managed file. The remediation loads ownership from the workspace's
recorded state (`.truss-core/manifest.json` plus `.truss-core/addons.json`),
enforces it before planning or applying, adds no CLI flag, and adds a real
CLI fixture with two recorded add-ons sharing a parent plus an exact collision
that must be refused.

Observation for the final integration review: after a payload-free `continue`
the record reports payload digests while the workspace holds the operator's
resolution, which is consistent with the record naming payload identity and the
workspace being a consumer modification. The reviewer did not flag it; the
integration review should confirm it is intended.

Installing the real delivery add-on into a workspace with the core installed is
refused:

```
$ truss addon install --name delivery --manifest scripts/delivery-install-files.txt \
    --source /tmp/addon-stage --source-ref truss-v0.1.13 --directory /tmp/addon-smoke
Error: add-on managed path is not declared by the payload: .agents/skills/audit-onboarding-proposal
```

The smoke test installed the core first, staged the 17 real payload paths, and
then ran the command; it exits 1. `collect_extra_managed_paths`
(`crates/truss/src/infrastructure/addon_state.rs:371`) computes its scan roots
as the declared directories with no declared ancestor, which for the delivery
add-on is `.agents`, and then walks the whole `.agents` tree. Every core skill
directory under `.agents/skills/` therefore counts as an undeclared add-on
managed path. The add-on delivery fixture could not use its own real location
and used `docs/demo/` instead, which hid this.

That makes the plan's own outcome unreachable, so it is Blocking. The hazard the
check exists for is real and must survive: an add-on must not silently claim a
directory that already holds foreign content. Pending a ruling on the ownership
semantics and the slice.

### S4b4 — Add-on ownership boundary

A real-payload smoke test found the defect that makes this plan's own outcome
unreachable: installing the real delivery add-on into a workspace with the core
installed is refused, because `collect_extra_managed_paths`
(`crates/truss/src/infrastructure/addon_state.rs:371`) takes `.agents` as a scan
root and then counts every core skill directory under `.agents/skills/` as an
undeclared add-on managed path. Decision 0003 clause 13 fixes the semantics: a
declared directory is a scan root only when it is not an ancestor-or-equal of a
path owned by another owner, and a valid pre-existing core state includes
`.truss-core/manifest.json`.

Owned paths:

```
crates/truss/src/infrastructure/addon_state.rs
crates/truss/src/infrastructure/state_io.rs
crates/truss/tests/common/mod.rs
crates/truss/tests/addon_state_record.rs
crates/truss/tests/cli_lifecycle.rs
```

Conditional: if the observe request must carry the foreign set through a port,
`crates/truss/src/application/addon_state.rs` joins the set and the handoff must
say so. Forbidden otherwise: `interface/`, `main.rs`, `scripts/**`,
`.truss-core/**`, `crates/truss/assets/**`, Cargo dependencies, any
classification or apply rule, and any session schema change.

The fixture must also close the gap that hid the defect: `tests/common/mod.rs`
`seed_core_state` writes only `.truss-core/.gitignore` and `.truss-core/lock`,
so every add-on fixture ran against an incomplete core state and clause 8 was
never exercised in its real shape.

- Instrument: in a workspace built by a real `truss install`, stage the real 17
delivery payload paths from `scripts/delivery-install-files.txt` and run the
whole cycle at the real location — `addon install` at `truss-v0.1.13`;
`addon status` reporting the ref and all 17 payload digests; one changed file
staged as `truss-v0.1.14` making `addon update --dry-run` report exactly one
`Update` with byte-identical workspace, baseline, and `addons.json`; `addon
update` applying it; a seeded consumer-edit against upstream change staging a
conflict and never applying; the operator editing `resolved/`; a payload-free
`addon continue`; an idempotent `addon abort`; and a seeded core
`.truss-core/update/session.json` plus `AGENTS.md` byte-identical throughout.
- Counterexample: in a second workspace, pre-create
`.agents/skills/delivery/stray.md`, an undeclared file inside the add-on's own
subtree. `addon install` must refuse naming `stray.md`. An implementation that
fixes the first refusal by dropping or hollowing the extra-path check passes the
real cycle and fails this fixture; symmetrically, a root selection that demotes
the add-on's own subtree passes the stray fixture and fails the real cycle. The
two fixtures discriminate each other.

The smaller `docs/demo` fixture stays acceptable as a fast unit fixture, but it
may not satisfy any row claiming that install or update works at an add-on's
real location.

### S5 — Installer delegation and platform parity

Bash and PowerShell: acquire the exact payload, validate the descriptor, stage
the files, invoke the CLI operation, and never direct-copy an existing managed
add-on file.

- Instrument: run the fixture flow on both platforms where available and
  compare workspace tree hashes, `addons.json`, `base-addons` tree hashes, and
  the reported plan. A fresh install must end equal to updating an older
  fixture to the same ref. Repository validation verifies both scripts
  delegate and that no add-on path still reaches `copy_file`/`Copy-TrussFile`
  for updates.
- Counterexample: Bash delegates while PowerShell keeps "merge keeps existing
  file"; a fixture with one upstream-changed existing add-on file must end
  identically on both platforms.

### S6 — Documentation and release proof

Update `README.md`, `.truss-core/docs/product/installation-profiles.md`, the
installer help text, and the release steps. Document the exact ref, dry run,
adoption, conflict recovery, abort, the `AGENTS.md` non-ownership, and platform
parity.

- Instrument: a fresh-source rehearsal and a released-source rehearsal reach
  identical add-on trees and provenance at the same ref.
- Counterexample: documentation claims the source is pinned while
  `--source-git` still clones the default branch. Run against a repository
  whose default branch advanced after the tag; installed hashes must remain
  those of the tag.

## Risks And Recovery

- **A dispatch receipt's file list is not evidence.** For `e196105` the
  orchestration payload reported one modified file while `git show --stat`
  reported the five owned paths. Evidence for scope comes from git, never from
  the receipt; the receipt's `reportPath`, `outcome`, and ids remain useful.
- **Frozen session risks.** Session schema 2 stores code bytes, so its size
  grows with the payload: add a test with the largest shipped add-on and report
  the bytes. Candidate files must reject symlinks and path escapes on both stage
  and load. The plan DTO must validate path-set equality across the descriptor,
  the candidate, the mutations, the conflicts, and the frozen observations. A
  binary upgrade between stage and continue must not alter classification,
  which holds only because continue no longer plans. An abandoned schema 1
  session needs an explicit operator message and a working abort path.
- **A concurrent session in the same working tree.** On 2026-09-24 another
session checked out `delivery/delivery-prerequisites` from this same checkout
at 10:16 and fast-forwarded `main` to `d640c9f` at 10:33 while this delivery
assumed `delivery/addon-update` at `68e5c4c`. Nothing was lost, because the
branch tip was committed and the tree was clean. The protocol now asserts the
branch and HEAD immediately before each dispatch and again before accepting a
handoff. Git refuses a branch switch while tracked files carry conflicting
modifications, so the mutation window is protected; the exposure is a silent
context switch between operations.
- **Pinned effort may not take effect.** `AGENTS.md` pins effort `medium` for
the implement role, but a dispatch handoff reported Pi running at effort
`off`, and the model catalog reports `thinking: no` for
`tao-router/code-writer`, so a `--thinking medium` argv cannot take effect.
Record requested and observed effort separately and never claim the pin took
effect. Recovery: pin a model that supports effort, or record the effective
value as unavailable.
- **Missing or damaged core state.** Add-on operations never bootstrap or
  repair core state. They refuse without mutation and direct the operator to
  core install or update recovery. This closes the case the S2 scoped
  re-review found, where `apply` created `.truss-core/`, patched its
  `.gitignore`, and created `lock` before an authoritative locked refusal.
- **Two writers of `.truss-core/`.** Mitigated by making the CLI the only
  writer and having the installers delegate; the installer never writes an
  existing managed add-on file. Recovery: the record and baseline are written
  after the files; a failed run leaves the workspace unchanged.
- **Removed upstream files.** The core planner already models deletion and
  modified-deletion conflict; S3 covers the deletion matrix. Recovery: an
  unchanged removed file is deleted transactionally; a modified removed file
  becomes a conflict.
- **Wrong or moving ref.** Everything becomes "consumer modified". Mitigated
  by recording `source_ref`, printing it in `--dry-run`, refusing branches as
  refs, and refusing to update when no ref can be resolved. Recovery:
  re-run with the correct immutable ref.
- **`git` unavailable.** The core update path already requires it. The add-on
  path must stop with a clear message and must never fall back to overwriting.
- **Path escape or symlink payload.** Reuse `RelativePath`,
  `validate_managed_path`, regular-file checks, and symlink rejection.
- **Partial payload acquisition.** Validate the complete descriptor before
  planning any workspace mutation. Recovery: delete the owned staging
  directory; workspace and provenance are untouched.
- **Executable-bit drift.** SHA-256 covers content, not mode. Constrain the
  first release to regular non-executable add-on files and fail descriptor
  generation on an unsupported mode.
- **Older client meeting new state.** The core schema stays 1, `addons.json`
  is ignored by older binaries, and the add-on conflict namespace keeps
  `.truss-core/update/` core-only. `.truss-core/lock` serializes mutations.

## Progress

- [x] S1 Payload descriptor and provenance model — `bef85bb`, remediated by
      `3d6e3f5`, independent review and scoped re-review both `ACCEPT`
- [x] S2 Installed add-on record and exact adoption — **accepted**. Chain:
      `ae7eb64` implemented, `2968c03` remediated the observation-before-lock
      finding, the scoped re-review then found the state-root residue, the
      contract was amended (`12985f8`) and the slice replanned as S2b,
      `ec2bfaf` implemented that, and the final review of the complete S2 range
      returned `ACCEPT`.
- [x] S2b Require an existing core state and keep refusal atomic — `ec2bfaf`,
      covered by the same final `ACCEPT`.
- [x] S3a Extract a pure reusable planner, behaviour-preserving — `dbe9fdb`,
      independent review `ACCEPT` with no findings, in two dispatches. The
      equivalence table transcribed the matrix rather than the new output, and
      S3b's consumer shape was named before the extraction.
- [x] S3b Add-on planner adapter: the complete matrix, dry run only — `68e5c4c`,
      independent review `ACCEPT` with no findings, in two dispatches. The
      adapter translates stored state into the S3a planner input and reuses the
      planner; it applies nothing.
- [x] S3c Transactional apply of a conflict-free plan — `6c5e76d`, remediated
      by `4cc0ec4`, scoped re-review `ACCEPT`. Four dispatches because the
      review found a real Blocking defect rather than a contract gap: the
      planner released `.truss-core/lock` before the applier reacquired it, so a
      stale plan could overwrite a competing write. The fix re-checks every
      frozen observation under the apply lock immediately before mutation.
- [x] S4 Scoped conflict recovery — `72cdb80`, independent review `ACCEPT` with
      no findings, in two dispatches. Sessions live under
      `.truss-core/addon-update/<name>/`; `.truss-core/update/` stays core-only;
      resume re-verifies every frozen observation under the shared lock.
- [~] S4b CLI surface — **BLOCKED before any mutation** at baseline `72cdb80`,
      candidate untouched, baseline green, and **superseded by replanning**.
      The approved argument shape assumed a self-contained resume that S4 did
      not provide, no public add-on session-pending query existed, and the
      plan/apply/session operations are infrastructure inherent methods the
      interface layer is forbidden to import. Ruling: session shape C, decision
      clause 12, and three slices below.
- [x] S4b1 Self-contained frozen conflict session (schema 2) — `e196105`,
      independent review `ACCEPT` with zero blocking findings, in two
      dispatches. Session schema 2 stores the candidate for every descriptor
      path, the descriptor identity, and the materialised plan with its digest;
      resume takes `root + AddOnName`, never re-plans, and schema 1 or an
      unsupported schema fails closed. Two minor notes recorded: session-size
      duplication (214,720 bytes for a 55,901-byte payload) and unix-only
      symlink test coverage.
- [x] S4b2 Application boundary and infrastructure implementations — `029a75f`,
      independent review `ACCEPT` with no findings, in two dispatches. Ports
      `AddOnPlanPort` and `AddOnExecutionPort` plus the `AddOnApplication`
      facade; `continue` never plans, a dry run never applies, a conflict stages,
      and the core `resolution_pending` is never consulted for add-on state.
      `clean_architecture.rs` was not modified.
- [x] S4b3 CLI surface, presentation, and composition wiring — `6bc2b46`,
      independent review `ACCEPT` with no findings, in two dispatches. The
      reviewer classified the real-location install defect as known, recorded,
      and routed to S4b4, and accepted no row as proof of a real-location
      install. `clean_architecture.rs` grew by additions only (+4/-0).
- [x] S4b4 Add-on ownership boundary — `f5dbbc2`, remediated by `d9357b9`,
      scoped re-review `ACCEPT`. Control reproduced the plan's core outcome by
      hand: with a real core install and the real 17-path payload, `addon
      install` at `.agents/skills/delivery` returns "installed (adopted=false)"
      at exit 0, and a pre-created `stray.md` refuses at exit 1 naming it. The
      review's Important finding — the real CLI passing an empty ownership set —
      was fixed by a **required** `AddOnStatePort::recorded_owners` consulted
      before planning or applying, with a real two-add-on collision fixture that
      refuses and names the other owner.
- [x] S5 Installer delegation and platform parity — `2fb8ea7`, remediated by
      `d32c488`, scoped re-review `ACCEPT`. Both installers stage the manifest
      payload and call one CLI operation; the direct-copy helpers are deleted;
      the ref is the release tag only when HEAD is exactly that release and
      otherwise the exact commit SHA. The review's two Important findings were
      proof gaps, not code defects: the rehearsal is now committed at
      `tests/s5-rehearse.sh` and run by the closure gate, and it proves all
      three add-ons against their own manifests, 56 ok / 0 failed. The
      installers are byte-identical to the reviewed revision. PowerShell stays
      static verification because `pwsh` is absent on this host, with the
      execution gap declared.
- [x] S6 Documentation and release proof — `0318944`, independent review
      `ACCEPT` with no findings, in two dispatches. Documents the lifecycle, the
      immutable ref rules, the post-tag release gate, and the two declared
      limits; Control reproduced the branch-ref refusal itself (`--source-ref
      main` exits 1 with the immutable-ref error).

### Integration review

The plan's slices are all accepted. Architectural work still needs one
integration review over the complete candidate by a fresh reviewer: task
interactions, complete-contract coverage of decision 0003's clauses, the
deferred findings, candidate identity, and release readiness. That review is
release-binding, and its verdict is bound to the exact HEAD it reviews.


### Open finding on S2 (Blocking, unresolved)

`fs::create_dir_all(&state_root)` and `ensure_state_ignore(&state_root)` run
before the shared lock and before the authoritative refusal. The authoritative
state load, observation, and commit do hold the lock, so a stale pre-lock
`preflight` can no longer bless or overwrite local bytes. But when the pre-lock
`preflight` passes and the locked observation then refuses, `.truss-core/`,
`.truss-core/.gitignore`, and normally `.truss-core/lock` remain behind. That
violates acceptance Row 3 read literally ("a stop performs no mutation",
whole-tree hashes equal), while Row 3's current test only covers a refusal
during `preflight`, so it misses the interleaving. The reviewer's required
remediation is either to restore an initially absent state root byte-for-byte
on refusal, or to redesign lock placement and state-root creation so refusal
leaves no residue, plus a deterministic proof forcing preflight-pass followed
by locked-observation refusal.

## Decisions

- 2026-09-23: The Rust CLI owns add-on state and updates; installers acquire
  and delegate. Promoted to
  `.truss-core/docs/decisions/0003-add-on-state-ownership.md`.
- 2026-09-23: `.truss-core/addons.json` is separate from the core manifest,
  which stays at schema version 1.
- 2026-09-23: Add-on conflict sessions get their own namespace; the merge
  owner stays `git merge-file`.
- 2026-09-23: The updater does not judge the `AGENTS.md` Delivery block.
- 2026-09-23: Row 1 evidence is written to the git-ignored
  `target/s1-evidence/` and replayed with `LC_ALL=C` on both `comm` inputs.
  The first review rejected the earlier `CARGO_TARGET_TMPDIR` route because
  the evidence did not survive the test run, so the approved instrument could
  not be replayed.
- 2026-09-23: The original claim that seven refusal tests were observed red
  before the checks existed is verified interactively only and is not
  reproducible from repository history. It is recorded, not carried as
  evidence.
- 2026-09-23: S2's scoped re-review did not accept, so Control routed to
  `REPLAN_OR_SPLIT` instead of starting a second repair pass. Whether the
  residue counts as a violation is a contract question, not a code-review
  question, and is pending a design ruling before any further task is
  dispatched.
- 2026-09-23: Creating a Run binds the coordinator terminal. A delivery that
  creates a second Run is fenced out of its own earlier Run
  (`consumer_fenced`); one delivery keeps one Run.
- 2026-09-23: Ruling on the S2 residue question. Add-on operations require a
  valid pre-existing core state and must not create or repair `.truss-core/`,
  `.truss-core/.gitignore`, or `.truss-core/lock`; core install and update own
  those artifacts. Rolling back a created state root and permitting the
  residue were both rejected, the first because cleanup around a lock file
  inside the directory being removed differs across platforms and turns a
  refusal into a recovery state, the second because it weakens
  validation-before-write and whole-tree equality to make an implementation
  pass. Promoted to decision 0003 clause 8.
- 2026-09-23: Row 3's original instrument was itself inadequate: hashing the
  fixture around a one-edited-byte attempt only ever exercised a refusal
  during `preflight`. Row 3 now requires the preflight-pass followed by
  locked-refusal interleaving with a competing-writer snapshot.
- 2026-09-23: Post-mortem on S2's cost, corrected after an independent audit.
  Ten worker dispatches covered two slices: S1 used four (implement, review,
  remediation, re-review) and S2 used six (implement, review, remediation,
  re-review, replanned task, final review). S2's cumulative commit churn is
  +2107/-343; its final range against S1's accepted tip is +2005/-218 across
  ten files.
- 2026-09-23: Ownership of that cost is shared, not Control-only. Control wrote
  the incomplete prerequisite and the non-discriminating Row 3; the implementer
  chose an unauthorized state-root bootstrap instead of returning
  `NEEDS_REPLAN`, and missed both the observation-before-lock window and the
  literal Row 3 violation; the reviewer found both defects correctly; the
  protocol added the expected remediation and re-review cost once the defects
  existed. Causes the first diagnosis omitted: the required two-review cost,
  blocking waits that inflate wall-clock without adding work, one stray Run
  that added coordination delay, the plan and plan-review role pins not being
  dispatched, Pi transcript and pin evidence friction, and a pin mismatch where
  `AGENTS.md` specifies effort medium while the dispatch reported effort off.
- 2026-09-23: The first diagnosis also over-corrected future process. An
  arbitrary 400-line and five-file ceiling, an abstract one-concern rule, an
  absolute refactor ban, and a review scope limited to declared rows are all
  replaced by the corrected rules in the Approach section. The cheapest
  preventive change was one explicit core-state precondition plus one
  deterministic Row 3 fixture; that alone would have avoided the remediation,
  the failed re-review, the replan, the contract amendment, and S2b. S3 is
  split into S3a-S3c.
- 2026-09-24: S5 dependency: installer delegation needed no Rust change. Both
  installers stage the manifest payload and call one `truss addon
  install|update`; the ref is the release tag only when the checkout is exactly
  that release and otherwise the exact commit SHA; a branch name is never
  passed. The direct-copy helpers (`copy_file`, `write_source_file`,
  `copy_manifest_files`, `Copy-TrussFile`, `Write-SourceFile`) were deleted from
  both scripts, so the core path is affected by that deletion too and the review
  must confirm core install, update, `--merge`, `--force`, and `--override`
  semantics are unchanged.
- 2026-09-24: S5's rehearsal script lives under `target/s5-evidence/`, which is
  gitignored, so the instrument is not part of the candidate the reviewer
  reviews. The review must decide whether that satisfies the contract's
  "reviewer can rerun one exact command on the candidate revision" rule or
  whether the rehearsal must be committed, for example under `tests/`, where the
  closure gate already syntax-checks shell files.
- 2026-09-24: S4b4 remediation needed a new `AddOnStatePort` method to deliver the
  recorded ownership set. Control chose a **required** trait method over a
  defaulted one: a default that answers an empty set is the same silent
  empty-ownership hole at the port boundary that the remediation exists to
  close. The one test fake gained a minimal stub, and the fake-port rows must
  record the call so the guard's ordering before planning and applying stays
  observable; changed expectations are listable additions, never removals.
- 2026-09-24: S4b4's clause-13 prerequisite invalidated two in-crate `#[cfg(test)]`
  fixtures in `crates/truss/src/infrastructure/addon_apply.rs`, which sat outside
  the slice's owned scope. Control authorized a narrow extension: that one file,
  test module only, zero production lines, fixtures seeding a real
  `.truss-core/manifest.json` and the real core skill files. Weakening clause 13
  or making `validate_core_state` lenient was rejected. Lesson: a slice that adds
  a prerequisite will invalidate fixtures outside its scope; the remedy is a
  test-only fixture update under Control authorization, visible to the reviewer.
- 2026-09-24: S4b blocked before mutation: the approved command shape assumed
  a self-contained resume, no public add-on session-pending query existed, and
  the plan/apply/session operations were infrastructure inherent methods the
  interface layer may not import. Ruling: the session persists the frozen
  decision (schema 2) instead of re-planning from an external payload, and the
  interface reaches add-on use cases through application ports and a facade,
  with `main.rs` as a wiring-only composition root. Promoted to decision 0003
  clause 12. Plan drift: the argument shape was fixed too concretely before the
  session's self-containment was proven; S4 should have exposed either a
  payload-free resume contract or an explicit statement that resume still needs
  the payload, plus the session-pending port.
- 2026-09-24: S3c's review found the same defect class as S2 in a new place:
  the plan was built under one lock section and applied under another, so a
  competing write between them could be overwritten. The rule that follows is
  general: a two-step add-on operation either holds one lock across both steps
  or re-verifies its frozen observation under the lock that commits. The
  reviewer also required the row's evidence to be emitted, not only asserted,
  which is the same standard S1 was rejected for missing.
- 2026-09-24: A concurrent session moved this checkout to `main` at `d640c9f`
  mid-delivery. `delivery/addon-update` and every slice tip were intact, so no
  work was lost and no slice was invalidated. The guard is a branch and HEAD
  assertion before each dispatch and before accepting a handoff, not a
  repository state change. `main` now carries `d640c9f`, a delivery-skill
  prerequisite-ownership change that this branch does not include, so the
  release step must reconcile it.

## Validation

- Focused proof: the S1-S5 instruments above, each with its counterexample
  observed red.
- Integration or end-to-end proof: install an add-on at a released tag from
  source and from the binary, edit one managed file, update to a newer tag,
  and confirm preserve, update, conflict, and abort paths; assert
  `AGENTS.md` is byte-identical throughout.
- Repository-required checks: `scripts/validate-premerge.sh`.

## Result

Delivered on branch `delivery/addon-update` at `0318944`, 23 commits from
`cdbb1b7`, local only.

Every slice was implemented, independently reviewed, and accepted: S1 descriptor,
S2 and S2b record and adoption, S3a planner extraction, S3b adapter, S3c
transactional apply, S4 conflict sessions, S4b1 self-contained session schema 2,
S4b2 ports and facade, S4b3 CLI surface, S4b4 ownership boundary, S5 installer
delegation, S6 documentation. The release-binding integration review returned
`ACCEPT` for this exact HEAD.

The plan's outcome is reachable and was reproduced by Control by hand: with a
real core install and the real 17-path payload, `truss addon install` succeeds at
`.agents/skills/delivery`, a pre-created `stray.md` refuses at exit 1 naming that
file, and `--source-ref main` refuses at exit 1 as a non-immutable ref. The
closure gate prints `pre-merge validation passed` at exit 0 on this exact HEAD,
and the committed installer rehearsal reports `56 ok, 0 failed`.

Limitations recorded rather than hidden:

- Windows and PowerShell parity is static verification only on this host because
  `pwsh` is absent; no byte-level cross-platform claim is supported.
- The released-source rehearsal is a post-tag release gate: it cannot run before
  a new `truss-vX.Y.Z` tag and its binaries exist.
- A dirty local add-on payload is refused rather than recorded, which diverges
  from clause 6; resolving it needs a Rust slice or an amendment.
- Session schema 2 duplicates payload bytes (214,720 bytes for a 55,901-byte
  payload).
- Symlink rejection is tested on unix only.
- `main` carries `d640c9f`, a delivery-skill documentation change this branch does
  not contain; the integration reviewer states reconciliation needs a fresh
  review because a verdict binds to one HEAD.

Also recorded as friction: an orchestration receipt's file list is unreliable
(one file reported where git showed five), and the delivery created one empty
stray Run plus untracked `.delivery/` artifacts.

Release, recorded 2026-09-24:

- The owner accepted the candidate at the second human gate and authorised the
  merge and release.
- Merged into `main` as `0cf3b72`. The merge result differs from the reviewed
  branch tip by exactly `main`'s own delivery-skill documentation commit, two
  files and eleven lines; every candidate path is byte-identical, which is the
  named affected subset for the review verdict. The closure gate passed on the
  merge result.
- Released as `truss-v0.1.14`: bumped from `5b777ab`, assets
  `truss-linux-x64` (2,617,544 bytes) and its `.sha256`, published and reachable.
- The released-source rehearsal ran while HEAD equalled the tag and closed that
  release gate: tag-pinned raw install exits 0, the recorded ref is the resolved
  release tag, and a floating base URL refuses. `56 ok, 0 failed`.
- `scripts/truss-release-tag` now names `truss-v0.1.14`, pushed last, and the raw
  URL serves it.
- Consumer path verified end to end: a 0.1.13 install reports
  `update_available`, and `truss update --dry-run` against the published release
  previews `0.1.13 -> 0.1.14` after verifying the checksum.
- Still open: Windows and PowerShell runtime parity, which the building host
  cannot exercise because `pwsh` is absent.

This plan moves to `../completed/` now that the candidate is accepted and
released.

