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
run_root() { env PAYLOAD_LAYOUT_ROOT="$1" python3 "$CHECKER"; }

pos "the repository distribution tree satisfies the layout contract" run

# Negative fixtures: copy the tree into an isolated root and mutate one thing.
# A fixture root carries the distribution tree, the scripts the checker reads for
# its manifests, crates/truss/assets for the asset-sourced counterparts, and the
# pre-refactor destination copies, so that a mutation makes exactly one rule
# fire instead of also tripping the duplicate-window rule on missing files.
fixture() { # name
  local name="$1"
  local manifest dest
  rm -rf "$WORK/$name"
  mkdir -p "$WORK/$name"
  cp -a "$ROOT/distribution" "$WORK/$name/distribution"
  cp -a "$ROOT/scripts" "$WORK/$name/scripts"
  cp -a "$ROOT/crates" "$WORK/$name/crates"
  for manifest in "$ROOT"/scripts/*-install-files.txt; do
    while IFS= read -r dest || [ -n "$dest" ]; do
      case "$dest" in ""|\#*) continue ;; esac
      mkdir -p "$WORK/$name/$(dirname "$dest")"
      cp -p "$ROOT/$dest" "$WORK/$name/$dest"
    done < "$manifest"
  done
  printf '%s\n' "$WORK/$name"
}

rm -f "$(fixture l1)/distribution/layout-version"
neg "a missing layout marker is rejected" "L1" run_root "$WORK/l1"

printf '3\n' > "$(fixture l1b)/distribution/layout-version"
neg "an unknown layout marker is rejected" "L1" run_root "$WORK/l1b"

rm -f "$(fixture l2)/distribution/payload/.truss-core/docs/WORKFLOW.md"
neg "a destination with no source is rejected" "L2" run_root "$WORK/l2"

# A generated destination that also exists as a payload file resolves twice. The
# copy carries the root bytes so that only the two-sources rule fires.
cp -p "$ROOT/AGENTS.md" "$(fixture l2b)/distribution/payload/AGENTS.md"
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
