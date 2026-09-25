# Single Truss Root Implementation Plan (4A)

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development
> (recommended) or executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A new installation writes every Truss path under `.truss/`, the CLI
reads an existing `.truss-core/` install and refuses a conflicting pair of roots,
and the shipped prose names the new paths.

**Architecture:** The rename is one contract change with five surfaces: the
manifest destinations, the embedded payload paths, the state root, the installer
route, and the shipped prose. The Rust side is concentrated in one helper —
`state_root(root)` in `crates/truss/src/infrastructure/state_io.rs:19-21` — which
every one of its call sites already goes through. `state_root` becomes
presence-based so an existing legacy install still resolves, and an explicit
conflict check refuses a repository where both roots exist. `truss migrate` and
the compatibility matrix's refusal messages beyond the conflict case are **not**
in this plan; they are plan 4B.

**Tech Stack:** Rust 2021 crate `truss`, Bash bootstrap + PowerShell twin, the
repository's Bash/Python contract checks, the S5 rehearsal.

**Spec:** `.truss-core/docs/decisions/0008-single-truss-root.md` (items 1-4 and
7). The plan argues from that record; the executor reads it.

## Global Constraints

- This plan lands on a branch that already carries decision 0007's distribution
  tree and decision 0006's private-envelope contract. Do not re-litigate those.
- `.truss-core/X` becomes `.truss/core/X` for all 13 manifest destinations and
  for every state path: `bin`, `manifest.json`, `addons.json`, `lock`,
  `transaction.json`, `base/`, `base-addons/`, `update/`,
  `update-candidate/`, `addon-update/`, `docs/`.
- `distribution/layout-version` becomes `3`.
- The payload counts do not change: 27 core destinations, 4 engineering-wisdom,
  18 delivery, 2 planning. `crates/truss/tests/cli_lifecycle.rs` keeps its
  18-path delivery expectation.
- No history rewrite, no untracking of tracked files, no deletion of a
  `.truss-core/` tree in a consumer. Legacy trees are read, never migrated in this
  plan.
- The shipped payload text has 54 lines naming `.truss-core` under
  `distribution/payload/**` and 4 in
  `distribution/entrypoints/agent-truss-block.md`. Every one of them changes.
- `~/.truss/delivery-log` is machine-local and unrelated to `<repo>/.truss/`.
  `references/maintenance-log.md` is not edited.
- Every task ends with its own commit on the working branch.

---

## File Structure

| Surface | Files | Change |
| --- | --- | --- |
| Declared destinations | `scripts/truss-install-files.txt` | 13 lines `.truss-core/...` → `.truss/core/...` |
| Payload mirror | `distribution/payload/.truss-core/**` | moved to `distribution/payload/.truss/core/**` |
| Pre-refactor counterparts | `crates/truss/assets/.truss-core/**` and the repository root `.truss-core/**` | **not renamed in this plan** — they are the duplicate window's comparison side, and the later duplicate-removal plan deletes them |
| Payload mirror entrypoint set | `distribution/payload/AGENTS.md` is generated; no file to move | — |
| Layout marker | `distribution/layout-version` | `2` → `3` |
| Embedded payload | `crates/truss/src/infrastructure/embedded_distribution.rs` | every logical path and include path naming `.truss-core/` becomes `.truss/core/` |
| State root | `crates/truss/src/infrastructure/state_io.rs` | presence-based `state_root`, plus `legacy_state_root` and a conflict check |
| Root refusal | `crates/truss/src/application/service.rs` and `addon_application.rs` | refuse a conflicting pair of roots before any mutation |
| Path test | `crates/truss/src/domain/model.rs:355-366` | the parse fixture uses the new path |
| Installer (Bash) | `scripts/install-truss.sh` | sentinel, layout preflight, staging source, provenance paths, binary ignore rules |
| Installer (PowerShell) | `scripts/install-truss.ps1` | the same changes |
| Contract checks | `tests/payload-layout-contract.sh`, `tests/delivery-role-contract.sh` | resolve through `distribution/payload/**` and the new destination strings |
| Shipped prose | `distribution/payload/**` (54 lines) and `distribution/entrypoints/agent-truss-block.md` (4 lines) | `.truss-core/...` → `.truss/core/...` |
| Docs | `README.md`, `.truss-core/docs/product/installation-profiles.md` | the installed tree diagram and the maintenance commands |

---

## Task 1: The CLI resolves and writes the new root

**Files:**
- Modify: `crates/truss/src/infrastructure/state_io.rs`
- Modify: `crates/truss/src/application/service.rs`, `crates/truss/src/application/addon_application.rs`
- Modify: `crates/truss/src/domain/model.rs:355-366`
- Modify: `crates/truss/src/infrastructure/embedded_distribution.rs`
- Modify: `scripts/truss-install-files.txt`
- Move: `distribution/payload/.truss-core/**` → `distribution/payload/.truss/core/**`
- Modify: `distribution/layout-version`

**Interfaces:**
- Consumes: `state_root(root) -> PathBuf` and its ~30 call sites; the manifest read at `embedded_distribution.rs`'s test module; the payload mirror and marker created by the layout plan.
- Produces: `resolve_state_root(root) -> Result<PathBuf, PortError>` (new, in `state_io.rs`) returning the new root when either root exists there, the new root for a fresh tree, and an error naming both paths when they both exist; `legacy_state_root(root) -> PathBuf`; the constant `NEW_STATE_DIR: &str = ".truss/core"` and `LEGACY_STATE_DIR: &str = ".truss-core"`. `state_root(root)` keeps its signature and delegates to the new helper's precedence without the conflict error, so existing call sites compile unchanged.

- [ ] **Step 1: Write the failing test for root resolution**

In `crates/truss/src/infrastructure/state_io.rs`'s test module, add:

```rust
#[test]
fn state_root_prefers_the_new_root_and_refuses_a_conflicting_pair() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // A fresh repository resolves to the new root.
    assert_eq!(state_root(root), root.join(".truss").join("core"));
    assert_eq!(resolve_state_root(root).unwrap(), root.join(".truss").join("core"));

    // A legacy installation resolves to the legacy root.
    fs::create_dir_all(root.join(".truss-core")).unwrap();
    fs::write(root.join(".truss-core/manifest.json"), b"{}").unwrap();
    assert_eq!(state_root(root), root.join(".truss-core"));
    assert_eq!(resolve_state_root(root).unwrap(), root.join(".truss-core"));

    // A new-root installation resolves to the new root.
    fs::create_dir_all(root.join(".truss/core")).unwrap();
    fs::write(root.join(".truss/core/manifest.json"), b"{}").unwrap();
    fs::remove_dir_all(root.join(".truss-core")).unwrap();
    assert_eq!(resolve_state_root(root).unwrap(), root.join(".truss").join("core"));

    // Both trees present is a refusal naming both paths.
    fs::create_dir_all(root.join(".truss-core")).unwrap();
    fs::write(root.join(".truss-core/manifest.json"), b"{}").unwrap();
    let error = resolve_state_root(root).unwrap_err().to_string();
    assert!(error.contains(".truss/core"), "error names the new root: {error}");
    assert!(error.contains(".truss-core"), "error names the legacy root: {error}");
}
```

- [ ] **Step 2: Run it and confirm it fails**

Run: `cargo test --locked -p truss state_root_prefers` — expected: the test fails to compile because `resolve_state_root` does not exist. Record the compiler error verbatim.

- [ ] **Step 3: Implement the resolution**

In `state_io.rs`, replace the current body:

```rust
pub(crate) fn state_root(root: &Path) -> PathBuf {
    root.join(".truss-core")
}
```

with:

```rust
/// The installed tree's root directory name for a new installation.
pub(crate) const NEW_STATE_DIR: &str = ".truss/core";

/// The installed tree's root directory name before decision 0008.
pub(crate) const LEGACY_STATE_DIR: &str = ".truss-core";

/// The legacy root path, for a read of an installation that predates 0008.
pub(crate) fn legacy_state_root(root: &Path) -> PathBuf {
    root.join(LEGACY_STATE_DIR)
}

fn root_is_installed(path: &Path) -> bool {
    path.join("manifest.json").exists() || path.join("base").exists()
}

/// The root a command should operate on, resolving presence rather than
/// configuration so a read and a write in one command agree.
///
/// A repository holding both trees is a refusal, not a precedence: two trees
/// mean two locks, two baselines, and a stale one of each.
pub(crate) fn resolve_state_root(root: &Path) -> Result<PathBuf, PortError> {
    let new = root.join(NEW_STATE_DIR);
    let legacy = legacy_state_root(root);
    if root_is_installed(&new) && root_is_installed(&legacy) {
        return Err(PortError::new(format!(
            "both {} and {} hold a Truss installation; decide which tree this repository \
             keeps before running Truss (plan 4B adds `truss migrate`)",
            NEW_STATE_DIR, LEGACY_STATE_DIR
        )));
    }
    if root_is_installed(&legacy) {
        return Ok(legacy);
    }
    Ok(new)
}

/// The resolved root without the conflict error, for call sites that only need
/// a path. Precedence matches `resolve_state_root`: a legacy installation wins
/// while it is the only installed tree.
pub(crate) fn state_root(root: &Path) -> PathBuf {
    let new = root.join(NEW_STATE_DIR);
    if root_is_installed(&new) || !root_is_installed(&legacy_state_root(root)) {
        return new;
    }
    legacy_state_root(root)
}
```

- [ ] **Step 4: Run the test and confirm it passes**

Run: `cargo test --locked -p truss state_root_prefers` — expected PASS.

- [ ] **Step 5: Refuse a conflicting pair before any mutation**

In each application entry point that starts a mutation — `install`, `update`,
`continue_update`, `abort_update`, and the add-on install/update/continue paths —
call `resolve_state_root(root)?` instead of relying on `state_root(root)`, so a
conflicting pair stops before a write. `crates/truss/src/application/service.rs`
and `addon_application.rs` own those entry points; the port that returns
`PortError` is already in scope in both. Add a test in `service.rs`'s test module
that an `install` against a repository holding both trees fails and writes
nothing, asserting the repository has an unchanged file list afterwards.

- [ ] **Step 6: Move the declared destinations, mirror, and marker**

```bash
python3 - <<'PY'
import pathlib
p = pathlib.Path("scripts/truss-install-files.txt")
lines = p.read_text(encoding="utf-8").splitlines(True)
out = [line.replace(".truss-core/", ".truss/core/", 1) if line.startswith(".truss-core/") else line
       for line in lines]
p.write_text("".join(out), encoding="utf-8")
PY
mkdir -p distribution/payload/.truss
git mv distribution/payload/.truss-core distribution/payload/.truss/core
printf '3\n' > distribution/layout-version
grep -c "^\.truss/core/" scripts/truss-install-files.txt   # must print 13
```

Do not rename the repository root `.truss-core/**` tree or
`crates/truss/assets/.truss-core/**` in this step or any later step of this plan:
they are the duplicate window's comparison side, and the duplicate-removal plan
owns their deletion.

- [ ] **Step 7: Switch the embedded payload to the new paths**

In `crates/truss/src/infrastructure/embedded_distribution.rs`, apply one rule to
every site: the logical path string passed to `add(...)` and the
`include_bytes!` path must each rename `.truss-core/` to `.truss/core/`. The
`include_bytes!` prefix `../../../../distribution/payload/` is unchanged, so a
site reads `include_bytes!("../../../../distribution/payload/.truss/core/docs/WORKFLOW.md")`.
The generated `AGENTS.md` logical path is unchanged. The manifest `include_str!`
is unchanged. Then:

```bash
grep -c "\.truss-core" crates/truss/src/infrastructure/embedded_distribution.rs   # must print 0
grep -c "include_bytes!\|include_str!" crates/truss/src/infrastructure/embedded_distribution.rs  # must print 28
```

- [ ] **Step 8: Update the path fixture**

In `crates/truss/src/domain/model.rs:355-366`, the parse fixture uses
`.truss-core/docs/WORKFLOW.md`; change both occurrences to
`.truss/core/docs/WORKFLOW.md`.

- [ ] **Step 9: Run the workspace tests**

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: exit 0 for all three. Tests that staged a payload into a temporary root
now stage it under `.truss/core`; a failure naming a missing `.truss-core` path
means a site was missed in Step 7 — fix the site, do not restore the old name.

- [ ] **Step 10: Commit**

```bash
git add -A crates distribution scripts/truss-install-files.txt
git commit -m "feat(truss): install under the single .truss root"
```

---

## Task 2: Both installers stage into the new tree

**Files:**
- Modify: `scripts/install-truss.sh`
- Modify: `scripts/install-truss.ps1`

**Interfaces:**
- Consumes: `distribution/payload/**` and `distribution/layout-version` from the layout plan; the new destination strings from Task 1.
- Produces: a bootstrap whose local mode stages every add-on destination from `distribution/payload/<destination>` into `<target>/<destination>`, whose sentinel recognises both roots, whose layout preflight runs before the core install, and whose binary ignore rules name `.truss/core/bin/truss`.

- [ ] **Step 1: Add the layout preflight**

In `scripts/install-truss.sh`, add a function and call it in the main sequence
immediately before `install_truss_core` (the sequence is
`preflight_addons`, `install_truss_core`, then the add-ons):

```bash
require_supported_layout() {
  local marker="distribution/layout-version"
  local value=""
  value="$(read_source_text "$marker" 2>/dev/null | head -n 1)" || true
  [ -n "$value" ] || fail "the Truss source at $SOURCE_ROOT declares no $marker; this bootstrap requires layout 3 and refuses to guess"
  [ "$value" = "3" ] || fail "the Truss source declares layout $value; this bootstrap requires layout 3. For a pre-0008 tag use --source-git <tag>, which ships its own bootstrap"
}
```

Mirror it in `scripts/install-truss.ps1` as `Require-SupportedLayout`, using its
`Read-SourceText`, and call it at the same point in the sequence.

- [ ] **Step 2: Teach the sentinel and staging the new tree**

In `scripts/install-truss.sh`:

- the local-mode sentinel currently requires `.truss-core/docs/TRUSS.md`; accept
  either `.truss/core/docs/TRUSS.md` (new) or `.truss-core/docs/TRUSS.md`
  (legacy), and keep requiring `AGENTS.md` beside it;
- `stage_addon_payload` currently sets the local payload source to the checkout
  itself; point it at `$SOURCE_ROOT/distribution/payload` so each destination
  resolves to `distribution/payload/<destination>`, and keep the remote branch
  building `$SOURCE_BASE_URL/distribution/payload/<destination>`;
- `assert_local_addon_payload_is_committed` must verify the source paths it
  actually staged rather than the destination strings: for every destination,
  check `distribution/payload/<destination>` against the recorded ref, which also
  closes the `--exclude-standard` hole decision 0007 §5 records.

Mirror all three changes in `scripts/install-truss.ps1`.

- [ ] **Step 3: Rename the binary ignore rules**

`merge_core_gitignore` in `scripts/install-truss.sh` and its PowerShell twin append
`.truss-core/bin/truss` and `.truss-core/bin/truss.exe`. Change both rules to
`.truss/core/bin/truss` and `.truss/core/bin/truss.exe`, and keep the existing
skip condition keyed on the new rules. Reconcile the known divergence while you
are there: the Bash skip inspects only the two rules, the PowerShell skip also
counts the marker line.

- [ ] **Step 4: Verify the rehearsal**

```bash
bash tests/s5-rehearse.sh
```

Expected: `rehearsal summary: 56 ok, 0 failed`. The lanes read destinations from
the manifests, so they follow the rename; a lane that hardcodes `.truss-core` and
fails is a file the plan missed — fix it in this task and say so in the report.

- [ ] **Step 5: Commit**

```bash
git add scripts/install-truss.sh scripts/install-truss.ps1
git commit -m "feat(installer): stage the payload into the .truss root"
```

---

## Task 3: The shipped prose and the contract checks name the new tree

**Files:**
- Modify: `distribution/payload/**` (54 lines)
- Modify: `distribution/entrypoints/agent-truss-block.md` (4 lines)
- Modify: `tests/payload-layout-contract.sh`, `tests/delivery-role-contract.sh`
- Modify: `README.md`, `.truss-core/docs/product/installation-profiles.md`

**Interfaces:**
- Consumes: the new destinations from Task 1 and the installer route from Task 2.
- Produces: shipped text that names `.truss/core/...`, and checks whose resolved
  paths are the distribution mirror and the new destinations rather than the
  repository root and the old ones.

- [ ] **Step 1: Rename the shipped references**

```bash
grep -rl "\.truss-core" distribution/payload distribution/entrypoints | \
  xargs python3 -c '
import sys, pathlib
for name in sys.argv[1:]:
    p = pathlib.Path(name)
    p.write_text(p.read_text(encoding="utf-8").replace(".truss-core", ".truss/core"), encoding="utf-8")
'
grep -rc "\.truss-core" distribution/payload distribution/entrypoints | grep -v ":0" || echo "no legacy reference left in shipped text"
```

`distribution/payload/.agents/skills/delivery/SKILL.md` carries
`.truss/delivery-runs/...` and `.truss/authority/approvals/...` from decision 0006.
Decision 0008 item 3 moves them to `.truss/delivery/runs/<run-key>/` and
`.truss/delivery/approvals/<run-key>.md`. Apply that rename in the same pass, in
`SKILL.md`, `templates/plan.md`, `templates/business-analysis.md`,
`templates/decision-record.md`, both plans README copies, and `WORKFLOW.md`, and
assert afterwards:

```bash
grep -rn "delivery-runs\|authority/approvals" distribution/payload || echo "old delivery paths gone"
```

- [ ] **Step 2: Retarget the layout check**

`tests/payload-layout-contract.sh` derives destinations from the manifests, so its
mapping rules follow the rename; two things need explicit attention: the
`ASSET_SOURCED` exception list names `.truss-core/docs/...` destinations and must
name `.truss/core/docs/...`, and For the new destinations this is the legacy root path, related by one rule that every
comparison and fixture must use: `.truss/core/<rest>` corresponds to
`.truss-core/<rest>`, so `.truss/core/docs/WORKFLOW.md` is compared against the
repository root's `.truss-core/docs/WORKFLOW.md`, and the four `ASSET_SOURCED`
destinations against `crates/truss/assets/.truss-core/docs/<name>`. Neither the
repository root tree nor the `crates/truss/assets/` tree is renamed by this plan;
they stay as the comparison side until the duplicate-removal plan deletes them.

Add one rule that the payload mirror contains no `.truss-core` directory:

```text
payload-layout-contract L7: distribution/payload must not contain a .truss-core directory
```

with a negative fixture that recreates one.

- [ ] **Step 3: Retarget the delivery contract check**

`tests/delivery-role-contract.sh` reads root `.agents/skills/delivery/**`,
`.truss-core/docs/WORKFLOW.md`, and `.truss-core/docs/plans/README.md`. Point those
defaults at the distribution mirror — `distribution/payload/.agents/skills/delivery/**`
and `distribution/payload/.truss/core/docs/**` — per decision 0007 §3 and the note
that this check must read the canonical distribution skill, not an installed copy.
Update the negative fixtures that mutate those paths accordingly.

- [ ] **Step 4: Update the two documents**

`README.md` shows the installed tree and the maintenance commands
(`.truss-core/bin/truss status`, and so on); `installation-profiles.md` describes
the tree, the state root, and the update contract. Change the paths and the tree
diagram to `.truss/core/**`, and name the three namespaces of decision 0008 item
1. Do not document `truss migrate` as available: it is plan 4B.

- [ ] **Step 5: Run the checks and the gate**

```bash
bash tests/payload-layout-contract.sh
bash tests/delivery-role-contract.sh
bash scripts/validate-premerge.sh
```

Expected: both checks `0 failed`, then `rehearsal summary: 56 ok, 0 failed` and
`pre-merge validation passed`. Record each verbatim summary line.

- [ ] **Step 6: Commit**

```bash
git add -A distribution tests README.md .truss-core/docs/product/installation-profiles.md
git commit -m "docs(truss): name the single .truss root in shipped prose"
```

---

## Acceptance

| Requirement | Instrument | Counterexample | Observed red |
| --- | --- | --- | --- |
| A fresh repository resolves to `.truss/core`, a legacy one to `.truss-core` | `state_root_prefers_the_new_root_and_refuses_a_conflicting_pair` | A resolution that always returns `.truss/core` would fail the legacy assertion | |
| A repository holding both roots is refused before any write | the `service.rs` install-refusal test plus its unchanged-file-list assertion | A resolution with precedence instead of an error would write into the new tree | |
| Every declared destination and embedded logical path is renamed | `grep -c "\.truss-core" embedded_distribution.rs` = 0 and `grep -c "^\.truss/core/" truss-install-files.txt` = 13 | One site left behind: the embedding/manifest equality test fails | |
| The payload mirror holds no legacy directory | `payload-layout-contract L7` | A recreated `.truss-core` directory under the mirror | |
| Both installers stage from the mirror into the new tree and refuse an unsupported layout | `tests/s5-rehearse.sh` lanes plus `require_supported_layout` | Staging from the repository root, or accepting a layout-2 marker | |
| Shipped text names the new tree and the new delivery paths | `grep -rn "\.truss-core\|delivery-runs\|authority/approvals" distribution/payload distribution/entrypoints` returns nothing | One prose line left naming the old root | |
| The full repository gate is green | `bash scripts/validate-premerge.sh` | Any check failing | |

**Cannot be observed:** whether an operator with an existing `.truss-core/`
installation ends up on the new tree; that is `truss migrate` in plan 4B, and this
plan only guarantees the legacy tree is read and never written under a new name.
Nothing here proves agent behavior.

---

## Stop conditions

- A consumer-facing path outside the `.truss-core` prefix changes: return
  `NEEDS_REPLAN`. Only the root prefix is in scope.
- A rehearsal lane hardcodes `.truss-core` in a way this task's file list does not
  cover: fix it in Task 2 and record it; if it needs a file outside the task's
  owned paths, stop.
- `cargo test` shows a manifest count moving off 27, 4, 18, or 2: a destination was
  added or dropped. Stop.
- The rename would require deleting or rewriting history: stop. Legacy trees are
  read, not rewritten.

## Closure gates

```bash
# from the repository root
bash tests/payload-layout-contract.sh
bash tests/delivery-role-contract.sh
cargo test --workspace --locked
bash scripts/validate-premerge.sh
git diff --check
```

## Self-review

- Spec coverage: 0008 item 1 (root and namespaces) is Task 1 Step 6 and item 3's
  delivery paths are Task 3 Step 1; item 2's ownership boundaries are already
  enforced by the CLI never reading `delivery/` or `authority/`; item 4's ignore
  contract is Task 2 Step 3 for the binary rules and Task 3 Step 4 for the
  documented rules; item 6's compatibility matrix lands partially — the conflict
  refusal is Task 1, the rest is plan 4B; item 7's phases 2 and 3 are these three
  tasks, and phase 4 is a later major.
- Known gap stated rather than hidden: `truss migrate`, the layout-2 refusal
  message for the raw route beyond `require_supported_layout`, and the
  old-binary-plus-new-root refusal are **not** in this plan.
- Type consistency: `NEW_STATE_DIR`, `LEGACY_STATE_DIR`, `legacy_state_root`,
  `resolve_state_root`, `state_root`, `require_supported_layout` /
  `Require-SupportedLayout`, and rule id `L7` are used with the same names in
  every step.
- Placeholders: none. Mechanical steps carry the exact command or the exact rule
  plus the count that proves it was applied.

## Result

Completed on branch `refactor/single-truss-root`, which also carries decision
0007's distribution tree and decision 0006's private-envelope contract. The CLI
resolves and writes `.truss/core`, refuses a repository holding both roots before
any mutation, both installers stage from `distribution/payload/**` into the new
destinations and refuse an unsupported layout before mutating, and the shipped
prose plus the two contract checks name the new tree.

Known limitation handed to plan 4B — **read-side refusal is not implemented**.
`truss status`, `truss doctor`, and `truss addon status` do not call the root
resolution, so in a repository holding both `.truss/core/**` and `.truss-core/**`
they take new-root precedence and report from one tree instead of refusing. Every
mutating entry point does refuse: the application and add-on install, update,
continue, abort, and self-update paths resolve the root first. Decision 0008's
compatibility table states a dual-root repository is refused without limiting that
to mutations, so plan 4B must either add read-side refusal to those three surfaces
or narrow the compatibility wording to mutations explicitly. Until then, a
dual-root repository is refused by every write and only diagnosed by a read.

Also handed forward: both installers still read `scripts/agent-truss-block.md` and
`scripts/claude-truss-block.md` at their legacy paths, which are now duplicate
sources for one block that the duplicate-removal plan must retire.

## Next plan

**4B — migration and compatibility:** the `truss migrate` command with its
refusal conditions and proven rollback, the old-binary-plus-new-root refusal
message, the raw-route unsupported-layout gate, the two-root adoption path, and
the rehearsal lanes that prove each of them.
