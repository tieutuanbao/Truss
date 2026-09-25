# Distribution Layout Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development
> (recommended) or executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A tracked `distribution/` tree becomes the single declarative source
mapping for every installed destination, guarded by an executable layout
contract, and the Rust CLI's embedded distribution reads from it.

**Architecture:** Two tasks. Task 1 adds `distribution/layout-version`,
`distribution/generated.txt`, and a mirrored `distribution/payload/**` copy of
every non-generated destination, plus `tests/payload-layout-contract.sh`, which
proves the mapping in both directions and proves the interim duplicate window
cannot diverge. Task 2 switches `embedded_distribution.rs` and the three Rust
tests that currently read payload from the repository root, so the CLI and its
tests stop depending on the root tree. The shell installers keep reading the root
copies until the next plan, so every gate stays green.

**Tech Stack:** Bash + Python 3 checker (repository pattern), Rust 2021 crate
`truss`, Cargo test.

**Spec:** `.truss-core/docs/decisions/0007-source-and-installed-separation.md`
(§1 distribution tree, §2 manifest contract, §3 build inputs, §5 provenance, §7
fresh clone). The plan argues from that record; the executor reads it.

## Global Constraints

- Installed destinations do not change. The four manifests stay at
  `scripts/*-install-files.txt` so their URLs do not move, and they keep listing
  destination strings. Counts today: `truss-install-files.txt` 27,
  `engineering-wisdom-install-files.txt` 4, `delivery-install-files.txt` 18,
  `plan-install-files.txt` 2 — 51 destinations, of which 46 are root files, 4 come
  from `crates/truss/assets/.truss-core/docs/**`, and 1 (`AGENTS.md`) is
  generated.
- `crates/truss/tests/cli_lifecycle.rs` expects 18 delivery manifest paths;
  nothing in this plan adds or removes a payload path.
- No install-state schema change, no `.truss-core/` rename, no change to
  `scripts/truss-release-tag`, which is metadata and not payload.
- **Interim duplicate window.** Both `distribution/payload/**` and the existing
  root copies exist during this plan and the next one. Nothing may make one an
  editable second copy: Task 1's check proves byte identity for every duplicate.
  The 4 asset-sourced destinations are the exception — for them the canonical
  bytes are the `crates/truss/assets/` copies, so the check compares
  `distribution/payload/<dest>` against the asset copy, not the root copy. The
  root copy of `.truss-core/docs/communication.md` is this repository's
  configured instance and legitimately differs.
- The structural checks prove file and byte relationships, not agent behavior.
- Every task ends with its own commit on the working branch.

---

## File Structure

| File | Responsibility |
| --- | --- |
| `distribution/layout-version` | Declares payload layout `2`. One non-comment line. |
| `distribution/generated.txt` | Declares each generated destination and its generator input. |
| `distribution/payload/**` | Mirrors installed destinations one to one for every non-generated destination. |
| `distribution/entrypoints/agent-truss-block.md` | Canonical entrypoint block; `AGENTS.md` is generated from it. |
| `tests/payload-layout-contract.sh` | Proves the mapping, the marker, orphan-freedom, generator resolution, and the duplicate window's byte identity. New file, wired into `scripts/validate-premerge.sh`. |
| `crates/truss/src/infrastructure/embedded_distribution.rs` | Switches 23 include sites to the distribution tree. |
| `crates/truss/tests/addon_payload_descriptor.rs`, `addon_session.rs`, `cli_lifecycle.rs` | Resolve the payload root from `distribution/payload` instead of the repository root. |

### Mapping rules the check enforces

A destination resolves to exactly one source:

1. a plain file at `distribution/payload/<destination>`, or
2. a declaration in `distribution/generated.txt`.

Declaration format: one record per line, tab-separated, no comments inside a
record:

```text
<destination>\t<generator-input>\t<prefix>
```

`<generator-input>` is a path under `distribution/entrypoints/`. `<prefix>` is a
literal prefix written before the generator bytes, with `\n` meaning a newline
and an empty field meaning no prefix. The single record today is:

```text
AGENTS.md	distribution/entrypoints/agent-truss-block.md	# Agent Instructions\n\n
```

---

## Task 1: The distribution tree and its layout contract

**Files:**
- Create: `distribution/layout-version`
- Create: `distribution/generated.txt`
- Create: `distribution/payload/**` (51 manifest destinations minus the generated one, mirrored)
- Create: `distribution/entrypoints/agent-truss-block.md`
- Create: `distribution/entrypoints/claude-truss-block.md`
- Create: `tests/payload-layout-contract.sh`
- Modify: `scripts/validate-premerge.sh`

**Interfaces:**
- Consumes: the four manifests, `crates/truss/assets/.truss-core/docs/**` (4 files), `scripts/agent-truss-block.md`, `scripts/claude-truss-block.md`.
- Produces: the env-var entry point `PAYLOAD_LAYOUT_ROOT` (default: the repository root) that all rules resolve against, so negative fixtures can run on isolated copies; rule ids `payload-layout-contract L1`..`L5`.

- [ ] **Step 1: Create the tree and the marker**

Mirror every non-generated destination. For a destination whose bytes currently
come from the repository root:

```bash
mkdir -p distribution/payload
while IFS= read -r manifest; do
  while IFS= read -r dest; do
    case "$dest" in ""|\#*) continue ;; esac
    [ "$dest" = "AGENTS.md" ] && continue
    mkdir -p "distribution/payload/$(dirname "$dest")"
    cp -p "$dest" "distribution/payload/$dest"
  done < "$manifest"
done < <(find scripts -name '*-install-files.txt' | sort)
```

For the 4 destinations whose embedded bytes come from `crates/truss/assets/`,
overwrite the copies with the asset bytes, because those are the canonical ones:

```bash
for dest in .truss-core/docs/communication.md \
            .truss-core/docs/plans/README.md \
            .truss-core/docs/plans/completed/README.md \
            .truss-core/docs/decisions/README.md; do
  cp -p "crates/truss/assets/$dest" "distribution/payload/$dest"
done
```

Then the marker, the entrypoints, and the declaration:

```bash
printf '2\n' > distribution/layout-version
mkdir -p distribution/entrypoints
cp -p scripts/agent-truss-block.md distribution/entrypoints/agent-truss-block.md
cp -p scripts/claude-truss-block.md distribution/entrypoints/claude-truss-block.md
printf 'AGENTS.md\tdistribution/entrypoints/agent-truss-block.md\t# Agent Instructions\\n\\n\n' > distribution/generated.txt
```

Verify the copy count before continuing: `find distribution/payload -type f | wc -l`
must print `50` (51 destinations minus the generated `AGENTS.md`), and
`find distribution/entrypoints -type f | wc -l` must print `2`.

- [ ] **Step 2: Write the layout contract check**

Create `tests/payload-layout-contract.sh`:

```bash
#!/usr/bin/env bash
# Payload layout contract — structural guard for the distribution tree.
#
# Authority: .truss-core/docs/decisions/0007-source-and-installed-separation.md
# (§1 distribution tree, §2 manifest contract, §3 build inputs).
#
# Proves the source mapping and the interim duplicate window, not agent
# behaviour. Negative proof mutates isolated copies under a temp root selected
# with PAYLOAD_LAYOUT_ROOT; the candidate working tree is never modified.
#
# Requirements: bash, python3, find. No new dependency.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${PAYLOAD_LAYOUT_ROOT:-$REPO}"

for command in python3 find; do
  command -v "$command" >/dev/null 2>&1 || {
    echo "payload-layout-contract requires: $command" >&2
    exit 1
  }
done

WORK="$(mktemp -d "${TMPDIR:-/tmp}/payload-layout-contract.XXXXXX")"
OK=0
BAD=0
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

check() {
  local label="$1" cond="$2"
  if [ "$cond" = "0" ]; then
    printf 'ok   %s\n' "$label"
    OK=$((OK + 1))
  else
    printf 'FAIL %s\n' "$label"
    BAD=$((BAD + 1))
  fi
}

pos() {
  local label="$1"
  shift
  local out st
  out="$("$@" 2>&1)"
  st=$?
  check "$label" "$st"
  [ "$st" = 0 ] || printf '%s\n' "$out" | sed 's/^/       /'
}

neg() {
  local label="$1" rule="$2"
  shift 2
  local out st
  out="$("$@" 2>&1)"
  st=$?
  if [ "$st" != 0 ] && printf '%s\n' "$out" | grep -q "$rule"; then
    check "$label" 0
  else
    check "$label" 1
    printf '%s\n' "$out" | sed 's/^/       /'
  fi
}

CHECKER="$WORK/layout.py"
cat > "$CHECKER" <<'PY'
import os
import sys

ROOT = os.environ.get("PAYLOAD_LAYOUT_ROOT", os.getcwd())
PAYLOAD = os.path.join(ROOT, "distribution", "payload")
ENTRY = os.path.join(ROOT, "distribution", "entrypoints")
MARKER = os.path.join(ROOT, "distribution", "layout-version")
GENERATED = os.path.join(ROOT, "distribution", "generated.txt")

# Destinations whose canonical bytes live in crates/truss/assets/, not at the
# repository root. Removed when the duplicate window closes.
ASSET_SOURCED = {
    ".truss-core/docs/communication.md",
    ".truss-core/docs/plans/README.md",
    ".truss-core/docs/plans/completed/README.md",
    ".truss-core/docs/decisions/README.md",
}

PROBLEMS = []


def fail(rule, message):
    PROBLEMS.append(rule)
    print("%s: %s" % (rule, message))


def read_manifest(path):
    values = []
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line and not line.startswith("#"):
                values.append(line)
    return values


def manifests():
    scripts = os.path.join(ROOT, "scripts")
    found = sorted(
        name for name in os.listdir(scripts) if name.endswith("-install-files.txt")
    )
    return [os.path.join(scripts, name) for name in found]


def generated():
    records = []
    if not os.path.isfile(GENERATED):
        fail("payload-layout-contract L2", "distribution/generated.txt is missing")
        return records
    with open(GENERATED, encoding="utf-8") as handle:
        for number, line in enumerate(handle, 1):
            line = line.rstrip("\n")
            if not line:
                continue
            fields = line.split("\t")
            if len(fields) != 3:
                fail("payload-layout-contract L4",
                     "generated.txt:%d has %d tab-separated fields, expected 3 "
                     "(destination, generator input, prefix)" % (number, len(fields)))
                continue
            records.append(tuple(fields))
    return records


def payload_files():
    found = []
    for base, _, names in os.walk(PAYLOAD):
        for name in names:
            full = os.path.join(base, name)
            found.append(os.path.relpath(full, PAYLOAD))
    return sorted(found)


def check_marker():
    if not os.path.isfile(MARKER):
        fail("payload-layout-contract L1", "distribution/layout-version is missing")
        return
    with open(MARKER, encoding="utf-8") as handle:
        lines = [line.strip() for line in handle if line.strip()]
    if lines != ["2"]:
        fail("payload-layout-contract L1",
             "distribution/layout-version declares %r; layout 2 is required and an "
             "unknown or missing declaration must fail before any mutation" % (lines,))


def check_mapping():
    declarations = generated()
    generated_destinations = {record[0] for record in declarations}
    destinations = []
    for manifest in manifests():
        destinations.extend(read_manifest(manifest))
    if not destinations:
        fail("payload-layout-contract L2", "no manifest destination was read")
        return
    for dest in destinations:
        mirrored = os.path.join(PAYLOAD, dest)
        has_file = os.path.isfile(mirrored)
        has_declaration = dest in generated_destinations
        if has_file and has_declaration:
            fail("payload-layout-contract L2",
                 "%s resolves to both a payload file and a generated declaration; "
                 "a destination must resolve to exactly one source" % dest)
        elif not has_file and not has_declaration:
            fail("payload-layout-contract L2",
                 "%s resolves to no source: expected distribution/payload/%s or a "
                 "generated declaration" % (dest, dest))
    for path in payload_files():
        if path not in destinations:
            fail("payload-layout-contract L3",
                 "distribution/payload/%s matches no manifest destination; the "
                 "manifest is the membership owner" % path)


def check_generators():
    for dest, generator, _ in generated():
        if not os.path.isfile(os.path.join(ROOT, generator)):
            fail("payload-layout-contract L4",
                 "generated destination %s names generator input %s, which does not "
                 "exist" % (dest, generator))
        if not generator.startswith("distribution/entrypoints/"):
            fail("payload-layout-contract L4",
                 "generated destination %s names generator input %s outside "
                 "distribution/entrypoints/" % (dest, generator))


def check_duplicate_window():
    for manifest in manifests():
        for dest in read_manifest(manifest):
            mirrored = os.path.join(PAYLOAD, dest)
            if not os.path.isfile(mirrored):
                continue
            if dest in ASSET_SOURCED:
                legacy = os.path.join(ROOT, "crates", "truss", "assets", dest)
            else:
                legacy = os.path.join(ROOT, dest)
            if not os.path.isfile(legacy):
                fail("payload-layout-contract L5",
                     "%s has no pre-refactor counterpart at %s, so the duplicate "
                     "window cannot be proven identical"
                     % (dest, os.path.relpath(legacy, ROOT)))
                continue
            with open(mirrored, "rb") as left, open(legacy, "rb") as right:
                if left.read() != right.read():
                    fail("payload-layout-contract L5",
                         "distribution/payload/%s differs from its pre-refactor "
                         "counterpart %s; during the duplicate window they must be "
                         "byte-identical" % (dest, os.path.relpath(legacy, ROOT)))
    for name in ("agent-truss-block.md", "claude-truss-block.md"):
        moved = os.path.join(ENTRY, name)
        legacy = os.path.join(ROOT, "scripts", name)
        if os.path.isfile(moved) and os.path.isfile(legacy):
            with open(moved, "rb") as left, open(legacy, "rb") as right:
                if left.read() != right.read():
                    fail("payload-layout-contract L5",
                         "distribution/entrypoints/%s differs from scripts/%s during "
                         "the duplicate window" % (name, name))


def main():
    check_marker()
    check_mapping()
    check_generators()
    check_duplicate_window()
    return 1 if PROBLEMS else 0


if __name__ == "__main__":
    sys.exit(main())
PY

echo "payload layout contract: $(git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo 'no git HEAD')"
echo

run() { env PAYLOAD_LAYOUT_ROOT="$ROOT" python3 "$CHECKER" "$@"; }

pos "the repository distribution tree satisfies the layout contract" run

# Negative fixtures: copy the tree into an isolated root and mutate one thing.
fixture() { # name
  local name="$1"
  rm -rf "$WORK/$name"
  mkdir -p "$WORK/$name"
  cp -a "$ROOT/distribution" "$WORK/$name/distribution"
  cp -a "$ROOT/scripts" "$WORK/$name/scripts"
  cp -a "$ROOT/crates" "$WORK/$name/crates"
  printf '%s\n' "$WORK/$name"
}

rm -f "$(fixture l1)/distribution/layout-version"
neg "a missing layout marker is rejected" "L1" run_root "$WORK/l1"

printf '3\n' > "$(fixture l1b)/distribution/layout-version"
neg "an unknown layout marker is rejected" "L1" run_root "$WORK/l1b"

rm -f "$(fixture l2)/distribution/payload/.truss-core/docs/WORKFLOW.md"
neg "a destination with no source is rejected" "L2" run_root "$WORK/l2"

printf 'AGENTS.md\tdistribution/entrypoints/agent-truss-block.md\t# Agent Instructions\\n\\n\n' >> "$(fixture l2b)/distribution/generated.txt"
neg "a destination with two sources is rejected" "L2" run_root "$WORK/l2b"

printf 'orphan\n' > "$(fixture l3)/distribution/payload/.truss-core/docs/orphan.md"
neg "a payload file matching no destination is rejected" "L3" run_root "$WORK/l3"

python3 - "$(fixture l4)/distribution/generated.txt" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
open(path, "w", encoding="utf-8").write(text.replace("distribution/entrypoints/", "scripts/"))
PY
neg "a generator input outside distribution/entrypoints is rejected" "L4" run_root "$WORK/l4"

printf 'diverged\n' >> "$(fixture l5)/distribution/payload/.truss-core/docs/README.md"
neg "a payload copy diverging from its pre-refactor counterpart is rejected" "L5" run_root "$WORK/l5"

echo
echo "== payload-layout-contract summary: $OK ok, $BAD failed =="
[ "$BAD" = 0 ]
```

Add the `run_root` helper the fixtures use, next to `run`:

```bash
run_root() { env PAYLOAD_LAYOUT_ROOT="$1" python3 "$CHECKER"; }
```

- [ ] **Step 3: Run the check and confirm it discriminates**

Run: `bash tests/payload-layout-contract.sh`
Expected: the positive row is `ok` and every negative fixture is `ok`. A negative
fixture reporting `FAIL` means the rule does not discriminate: fix the rule, do
not weaken the fixture. Capture the summary line.

- [ ] **Step 4: Wire the check into the repository gate**

In `scripts/validate-premerge.sh`, after `bash tests/delivery-role-contract.sh`,
add:

```bash
bash tests/payload-layout-contract.sh
```

Then replace the manifest existence loop:

```bash
while IFS= read -r manifest; do
  while IFS= read -r path; do
    [[ -z "$path" || "$path" == \#* ]] || [[ -f "$path" ]] || { echo "manifest path missing: $path ($manifest)" >&2; exit 1; }
  done < "$manifest"
done < <(find scripts -name '*-install-files.txt' | sort)
```

with a comment pointing at the new check:

```bash
# Destination-to-source resolution is proven by tests/payload-layout-contract.sh,
# which checks both directions of the manifest contract and the layout marker.
# The former root-existence loop is gone: a destination no longer has to exist at
# the repository root, and requiring it there would keep the root tree alive.
```

- [ ] **Step 5: Run the repository gate**

```bash
bash tests/payload-layout-contract.sh
bash tests/delivery-role-contract.sh
bash scripts/validate-premerge.sh
```

Expected: `payload-layout-contract summary: N ok, 0 failed`,
`delivery-role-contract summary: 18 ok, 0 failed`, then
`pre-merge validation passed`. The rehearsal must still pass because the shell
installers still read the root copies, which this task does not remove.

- [ ] **Step 6: Commit**

```bash
git add distribution tests/payload-layout-contract.sh scripts/validate-premerge.sh
git commit -m "feat(distribution): add the payload layout tree and its contract"
```

---

## Task 2: The Rust CLI embeds the distribution tree

**Files:**
- Modify: `crates/truss/src/infrastructure/embedded_distribution.rs`
- Modify: `crates/truss/tests/addon_payload_descriptor.rs:84-145`
- Modify: `crates/truss/tests/addon_session.rs:963-1052`
- Modify: `crates/truss/tests/cli_lifecycle.rs:794-825`

**Interfaces:**
- Consumes: the tree and mapping from Task 1.
- Produces: an embedded distribution whose 23 include inputs resolve under
  `distribution/`, and Rust tests whose payload root is
  `repository/distribution/payload`.

- [ ] **Step 1: Switch the include sites**

In `crates/truss/src/infrastructure/embedded_distribution.rs`, apply exactly this
transformation inside `current()`, changing nothing else:

- `include_bytes!("../../../../scripts/agent-truss-block.md")` becomes
  `include_bytes!("../../../../distribution/entrypoints/agent-truss-block.md")`.
- Every other `include_bytes!("../../../../<path>")` and
  `include_str!("../../../../<path>")` becomes
  `include_bytes!("../../../../distribution/payload/<path>")` accordingly, except
  the manifest read in the test module: `include_str!("../../../../scripts/truss-install-files.txt")`
  stays exactly as it is.
- Every `include_bytes!("../../assets/<path>")` becomes
  `include_bytes!("../../../../distribution/payload/<path>")`.

The logical destination strings passed to `add(...)` must not change. After the
edit, `grep -c 'include_bytes!\|include_str!'` still reports 28 sites, and no
site may mention `assets/` or a bare `../../../../.agents/` path:

```bash
grep -n 'include_bytes!\|include_str!' crates/truss/src/infrastructure/embedded_distribution.rs
```

- [ ] **Step 2: Run the embedding tests and confirm they fail loudly if you missed a site**

Run: `cargo test --locked -p truss embedded_distribution`
Expected: PASS. A missing file is a compile error naming the path, which is the
point of the change: the CLI can no longer compile against the root tree alone.

- [ ] **Step 3: Point the Rust test fixtures at the distribution payload root**

Three tests read payload bytes from the repository root. Change only the payload
root they pass, never the destination strings in a manifest, record, baseline, or
evidence file:

- `crates/truss/tests/addon_payload_descriptor.rs:86-93`: keep
  `repository.join("scripts/...")` for the manifests and pass
  `repository.join("distribution").join("payload")` as the `root` argument to
  `spec(...)` at line 93. Evidence paths stay under `repository/target/s1-evidence`.
- `crates/truss/tests/addon_session.rs:963-1008`: the real-delivery fixture keeps
  its manifest location and reads payload bytes from
  `repo.join("distribution").join("payload").join(file.path)` at lines 1000-1008
  and 1041-1045. The manifest path and the recorded destination strings do not
  change.
- `crates/truss/tests/cli_lifecycle.rs:794-825`: `RealDeliveryFixture` keeps
  `repository_root()` for the manifest and reads source bytes from
  `repository_root().join("distribution").join("payload").join(&path)`. The
  18-path hard-coded expectation at lines 953-970 does not change.

- [ ] **Step 4: Run the workspace tests**

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Expected: all three exit 0. Evidence files under `target/s1-evidence`,
`target/s4b1-evidence`, and `target/s4b4-evidence` are regenerated by these runs;
they are ignored scratch and revision-bound, so stale copies are not evidence.

- [ ] **Step 5: Run the repository gate**

Run: `bash scripts/validate-premerge.sh`
Expected: `payload-layout-contract` and `delivery-role-contract` both report
`0 failed`, the rehearsal reports `56 ok, 0 failed`, and the gate prints
`pre-merge validation passed`. The rehearsal still passes because the shell
installers still read the root copies.

- [ ] **Step 6: Commit**

```bash
git add crates/truss/src/infrastructure/embedded_distribution.rs \
  crates/truss/tests/addon_payload_descriptor.rs \
  crates/truss/tests/addon_session.rs \
  crates/truss/tests/cli_lifecycle.rs
git commit -m "feat(truss): embed the distribution payload tree"
```

---

## Acceptance

| Requirement | Instrument | Counterexample | Observed red |
| --- | --- | --- | --- |
| The layout marker declares layout 2 and an unknown or missing declaration fails | `payload-layout-contract L1` | A missing marker (`l1`) and a marker declaring `3` (`l1b`) | Task 1: both fixtures `ok` in the `8 ok, 0 failed` run |
| Every destination resolves to exactly one source, and every source resolves to exactly one destination | `payload-layout-contract L2` | A destination with no source (`l2`); a destination with both a file and a declaration (`l2b`) | Task 1: `l2` and `l2b` `ok`. Final fix wave added `l2d` (duplicate declaration), `l2u` (declaration no manifest owns), and `l2m` (destination repeated across manifests), all `ok` in the `16 ok, 0 failed` run |
| No payload file escapes the manifest | `payload-layout-contract L3` | An orphan file under `distribution/payload/` (`l3`) | Task 1: `l3` `ok` |
| Every generated destination names a generator that resolves inside `distribution/entrypoints/` | `payload-layout-contract L4` | A generator input pointing at `scripts/` (`l4`); a traversal path (`l4t`); an escaping symlink (`l4s`) | Task 1: `l4` `ok`. Final fix wave: `l4t` and `l4s` `ok` against a `realpath` containment check |
| The duplicate window cannot diverge | `payload-layout-contract L5` | A payload copy diverging from its pre-refactor counterpart (`l5`); a missing Claude entrypoint copy (`l5e`) | Task 1: `l5` `ok`, and a full `cmp` sweep over all 50 copies plus both entrypoints reported 0 mismatches. Final fix wave: `l5e` `ok`, with both entrypoint copies now required present and compared |
| The generated declaration is bound to what the CLI produces | `payload-layout-contract L6` and the Rust test `generated_agents_md_matches_declaration` | A declaration prefix changed to `# Other Instructions\n\n`; a declaration generator pointing at the Claude block | Final fix wave: both fixtures `ok`; the Rust library run reports 24 passed, 0 failed |
| The CLI cannot compile from the root tree alone | `cargo test --workspace --locked` and `cargo test --locked --lib` | An include site left pointing at the root tree: the build fails to find the file | Task 2: 28 macro sites with 27 under `distribution/payload` or `distribution/entrypoints` and the manifest `include_str!` untouched; `cargo test --locked --lib` 24 passed, 0 failed |
| Installed destinations are unchanged | `cargo test --workspace --locked` (`cli_lifecycle` 18-path expectation, `embedded_distribution` manifest-equality assertion, `addon_payload_descriptor`) | A manifest line added or dropped; a descriptor path prefixed with `distribution/` | Task 2: destination literals set-identical between base and head (27 = 27); manifest counts 27, 4, 18, 2 unchanged; `cli_lifecycle` keeps its 18-path expectation |
| The shippable rehearsal still passes on the interim tree | `bash scripts/validate-premerge.sh` | The shell route changing without the root copies being present | Closure run on `267a8c4`: `payload-layout-contract 16 ok, 0 failed`, `delivery-role-contract 18 ok, 0 failed`, `rehearsal 56 ok, 0 failed`, `pre-merge validation passed`, exit 0 |

**Cannot be observed:** that a future editor keeps the two trees in step once
this plan's duplicate window closes; that is the next plan's job, and until then
L5 is the only guard. Nothing here proves agent behavior.

---

## Stop conditions

- `find distribution/payload -type f | wc -l` is not 50, or an entrypoint copy is
  missing: stop and reconcile the manifest inventory before writing the checker.
- A destination resolves neither way and the correct fix would change an
  installed destination: return `NEEDS_REPLAN`. Destination strings are contract.
- `cargo test` shows a manifest count moving off 18 for the delivery manifest: a
  payload path was added or removed. Stop.
- The rehearsal fails because a root copy was removed: this plan does not remove
  root copies; restore it and report.

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

- Spec coverage: 0007 §1 (tree, marker, entrypoints, generated declaration) and
  §3 (build inputs read canonical sources only) land in Tasks 1 and 2. §2's
  manifest contract lands as the L2/L3/L4 rules. §5's provenance replacement is
  **not** here: it belongs with the installer route change, because the check
  runs in the installer. §4, §6, §7, §8, §9 are later plans.
- Task-2 scope check: the three Rust tests are batched into one task on purpose —
  they are the same one-line payload-root change and they must move together with
  the embedding switch, or they would keep reading the root tree and pass falsely.
- Type consistency: `PAYLOAD_LAYOUT_ROOT`, `run`/`run_root`, `fixture`, rule ids
  L1-L5, and the `distribution/payload` root are used with the same names in every
  step.
- Placeholders: none. Task 1 carries the full checker source and the exact copy
  commands; Task 2 carries the exact transformation and the exact lines to change.

## Result

Completed 2026-09-25 on branch `refactor/distribution-layout-foundation`, commits
`05f0a4c`, `51f36ad`, `565e3b0`, and `267a8c4`.

`distribution/` is now the declarative source mapping: `layout-version` declares
layout 2, `payload/` mirrors all 50 non-generated installed destinations, and
`generated.txt` declares that `AGENTS.md` is composed from
`distribution/entrypoints/agent-truss-block.md` with the prefix
`# Agent Instructions\n\n`. `tests/payload-layout-contract.sh` proves the
mapping in both directions, rejects duplicate declarations and duplicate manifest
ownership, resolves generator containment through `realpath`, binds the
declaration to the CLI's own generator and prefix literals, requires both
entrypoint copies during the duplicate window, and proves the copies cannot
diverge. The Rust CLI's 27 payload includes and its three payload-reading tests
now read the distribution tree.

Validation on the final head: `payload-layout-contract` 16 ok, 0 failed;
`delivery-role-contract` 18 ok, 0 failed; `cargo test --locked --lib` 24 passed;
`scripts/validate-premerge.sh` reported `rehearsal 56 ok, 0 failed` and
`pre-merge validation passed`, exit 0.

Limitations: the shell installers still read the root copies, so this plan's
evidence says nothing about the installer route; rule L5 must be retired by the
plan that deletes the root copies; and no runtime guard exists that would detect
a production payload reader returning to the root tree once L5 is gone.

Follow-up: the installer route switch, then duplicate removal with the repository
migration.

## Next plans

1. **Installer route switch** — Bash and PowerShell read the payload from
   `distribution/payload`, entrypoints from `distribution/entrypoints`, check the
   layout marker before core mutation, verify provenance against the recorded ref
   blob (covering manifests, marker, and generator inputs), split the S5
   `PAYLOAD_PROBE`, and revise the historical old-installer counterexample.
2. **Duplicate removal and repository migration** — delete the root payload
   copies and the `crates/truss/assets/` duplicates, retarget the delivery
   contract defaults at the canonical distribution paths, add the installer
   local-only exclude mode, self-install in local-only mode, clean `.gitignore`,
   and untrack the authority documents.
