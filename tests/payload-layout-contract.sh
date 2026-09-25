#!/usr/bin/env bash
# Payload layout contract — structural guard for the distribution tree.
#
# Authority: .truss-core/docs/decisions/0007-source-and-installed-separation.md
# (§1 distribution tree, §2 manifest contract, §3 build inputs).
#
# Proves the source mapping, the generated-destination binding, and the interim
# duplicate window, not agent behaviour. Negative proof mutates isolated copies
# under a temp root selected with PAYLOAD_LAYOUT_ROOT; the candidate working tree
# is never modified.
#
# Rules:
#   L1  distribution/layout-version declares layout 3.
#   L2  every destination resolves to exactly one source; a destination has
#       exactly one manifest line and at most one generated declaration; a
#       declaration never introduces a destination no manifest lists.
#   L3  no payload file escapes the manifest membership.
#   L4  a generator input exists and resolves strictly beneath
#       distribution/entrypoints/.
#   L5  during the duplicate window every payload copy and both entrypoint
#       copies equal their pre-refactor counterpart.
#   L6  the generated declaration agrees with the literals the CLI composes
#       AGENTS.md from.
#   L7  the payload mirror carries no .truss-core directory; the installed
#       destination prefix is .truss/core since decision 0008.
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
import re
import sys

ROOT = os.environ.get("PAYLOAD_LAYOUT_ROOT", os.getcwd())
PAYLOAD = os.path.join(ROOT, "distribution", "payload")
ENTRY = os.path.join(ROOT, "distribution", "entrypoints")
MARKER = os.path.join(ROOT, "distribution", "layout-version")
GENERATED = os.path.join(ROOT, "distribution", "generated.txt")
GENERATOR_SOURCE = os.path.join(
    ROOT, "crates", "truss", "src", "infrastructure", "embedded_distribution.rs"
)

# Destinations whose canonical bytes live in crates/truss/assets/, not at the
# repository root. Removed when the duplicate window closes.
ASSET_SOURCED = {
    ".truss/core/docs/communication.md",
    ".truss/core/docs/plans/README.md",
    ".truss/core/docs/plans/completed/README.md",
    ".truss/core/docs/decisions/README.md",
}

# The installed destination prefix decision 0008 introduced, and the prefix it
# replaced. The rename is a prefix change only, so one rule relates the two:
# `.truss/core/<rest>` corresponds to `.truss-core/<rest>`. Neither the
# repository root tree nor crates/truss/assets is renamed by this plan; they are
# the comparison side until the duplicate-removal plan deletes them.
NEW_PREFIX = ".truss/core/"
LEGACY_PREFIX = ".truss-core/"
LEGACY_DIRECTORY = ".truss-core"


def legacy_counterpart(dest):
    """The pre-refactor path holding the bytes for a destination.

    Only the installed root was renamed, so a destination that never carried the
    .truss/core prefix is still its own pre-refactor counterpart at the repository
    root. Returning None for those was the bug: it reported the whole .agents tree
    as undefined.
    """
    if not dest.startswith(NEW_PREFIX):
        return os.path.join(ROOT, dest)
    relative = LEGACY_PREFIX + dest[len(NEW_PREFIX):]
    if dest in ASSET_SOURCED:
        return os.path.join(ROOT, "crates", "truss", "assets", relative)
    return os.path.join(ROOT, relative)

# The two entrypoint blocks the duplicate window must keep byte-identical.
ENTRYPOINT_BLOCKS = ("agent-truss-block.md", "claude-truss-block.md")

GENERATOR_LITERAL = re.compile(
    r'let\s+agent_block\s*=\s*include_bytes!\("([^"]+)"\)', re.DOTALL
)
PREFIX_LITERAL = re.compile(r'let\s+mut\s+agents\s*=\s*b"((?:[^"\\]|\\.)*)"')

PROBLEMS = []


def fail(rule, message):
    PROBLEMS.append(rule)
    print("%s: %s" % (rule, message))


def decode_escapes(value):
    """Decode the backslash escapes the declaration and the Rust literal share."""
    out = []
    index = 0
    while index < len(value):
        char = value[index]
        if char == "\\" and index + 1 < len(value):
            following = value[index + 1]
            mapped = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", "\\": "\\"}
            out.append(mapped.get(following, following))
            index += 2
            continue
        out.append(char)
        index += 1
    return "".join(out)


def repo_relative(path):
    """Drop the leading parent hops an include path carries."""
    prefix = "../"
    while path.startswith(prefix):
        path = path[len(prefix):]
    return path


def manifests():
    scripts = os.path.join(ROOT, "scripts")
    found = sorted(
        name for name in os.listdir(scripts) if name.endswith("-install-files.txt")
    )
    return [os.path.join(scripts, name) for name in found]


def manifest_entries():
    """Every destination with the manifest and line it came from."""
    entries = []
    for path in manifests():
        with open(path, encoding="utf-8") as handle:
            for number, line in enumerate(handle, 1):
                line = line.strip()
                if line and not line.startswith("#"):
                    entries.append((os.path.relpath(path, ROOT), number, line))
    return entries


def generated():
    """Declaration records as (line number, destination, generator, prefix)."""
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
            records.append((number, fields[0], fields[1], fields[2]))
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
    if lines != ["3"]:
        fail("payload-layout-contract L1",
             "distribution/layout-version declares %r; layout 3 is required and an "
             "unknown or missing declaration must fail before any mutation" % (lines,))


def check_declarations(declarations):
    """A destination is declared at most once."""
    first = {}
    for number, dest, _, _ in declarations:
        if dest in first:
            fail("payload-layout-contract L2",
                 "generated.txt declares %s twice (lines %d and %d); a destination "
                 "must have exactly one declaration" % (dest, first[dest], number))
        else:
            first[dest] = number
    return first


def check_manifest_cardinality(entries):
    """A destination appears exactly once across the manifests."""
    first = {}
    for manifest, number, dest in entries:
        if dest in first:
            previous, previous_line = first[dest]
            fail("payload-layout-contract L2",
                 "%s is listed more than once in the manifests (%s:%d and %s:%d); "
                 "each destination must have exactly one manifest line"
                 % (dest, previous, previous_line, manifest, number))
        else:
            first[dest] = (manifest, number)
    return first


def check_mapping(declarations, entries):
    declared = check_declarations(declarations)
    listed = check_manifest_cardinality(entries)
    if not entries:
        fail("payload-layout-contract L2", "no manifest destination was read")
        return
    for dest in sorted(listed):
        mirrored = os.path.join(PAYLOAD, dest)
        has_file = os.path.isfile(mirrored)
        has_declaration = dest in declared
        if has_file and has_declaration:
            fail("payload-layout-contract L2",
                 "%s resolves to both a payload file and a generated declaration; "
                 "a destination must resolve to exactly one source" % dest)
        elif not has_file and not has_declaration:
            fail("payload-layout-contract L2",
                 "%s resolves to no source: expected distribution/payload/%s or a "
                 "generated declaration" % (dest, dest))
    for dest, number in sorted(declared.items()):
        if dest not in listed:
            fail("payload-layout-contract L2",
                 "generated.txt:%d declares %s, which no manifest lists; the "
                 "manifest is the membership owner and a declaration must not "
                 "introduce a destination" % (number, dest))
    for path in payload_files():
        if path not in listed:
            fail("payload-layout-contract L3",
                 "distribution/payload/%s matches no manifest destination; the "
                 "manifest is the membership owner" % path)


def check_generators(declarations):
    entry_root = os.path.realpath(ENTRY)
    for _, dest, generator, _ in declarations:
        if not generator.startswith("distribution/entrypoints/"):
            fail("payload-layout-contract L4",
                 "generated destination %s names generator input %s outside "
                 "distribution/entrypoints/" % (dest, generator))
            continue
        path = os.path.join(ROOT, generator)
        if not os.path.isfile(path):
            fail("payload-layout-contract L4",
                 "generated destination %s names generator input %s, which does not "
                 "exist" % (dest, generator))
            continue
        resolved = os.path.realpath(path)
        if resolved == entry_root or not resolved.startswith(entry_root + os.sep):
            fail("payload-layout-contract L4",
                 "generated destination %s names generator input %s, which resolves "
                 "to %s outside distribution/entrypoints/; a spelling prefix is not "
                 "containment" % (dest, generator, resolved))


def check_generated_binding(declarations):
    """The declaration must agree with the literals the CLI composes from."""
    if not os.path.isfile(GENERATOR_SOURCE):
        fail("payload-layout-contract L6",
             "%s is missing, so the generated declaration cannot be bound to the "
             "embedded distribution"
             % os.path.relpath(GENERATOR_SOURCE, ROOT))
        return
    with open(GENERATOR_SOURCE, encoding="utf-8") as handle:
        source = handle.read()
    generator_match = GENERATOR_LITERAL.search(source)
    prefix_match = PREFIX_LITERAL.search(source)
    if not generator_match or not prefix_match:
        fail("payload-layout-contract L6",
             "cannot extract the AGENTS.md generator and prefix literals from "
             "%s; the generated declaration is unbound"
             % os.path.relpath(GENERATOR_SOURCE, ROOT))
        return
    source_generator = repo_relative(generator_match.group(1))
    source_prefix = decode_escapes(prefix_match.group(1))
    for _, dest, generator, prefix in declarations:
        if repo_relative(generator) != source_generator:
            fail("payload-layout-contract L6",
                 "generated destination %s declares generator input %s while the CLI "
                 "composes AGENTS.md from %s; the declaration must name the generator "
                 "the CLI uses" % (dest, generator, source_generator))
        if decode_escapes(prefix) != source_prefix:
            fail("payload-layout-contract L6",
                 "generated destination %s declares prefix %r while the CLI composes "
                 "%r; the declaration must state the prefix the CLI writes"
                 % (dest, decode_escapes(prefix), source_prefix))


def check_duplicate_window(entries):
    for _, _, dest in entries:
        mirrored = os.path.join(PAYLOAD, dest)
        if not os.path.isfile(mirrored):
            continue
        legacy = legacy_counterpart(dest)
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
    for name in ENTRYPOINT_BLOCKS:
        moved = os.path.join(ENTRY, name)
        legacy = os.path.join(ROOT, "scripts", name)
        if not os.path.isfile(moved):
            fail("payload-layout-contract L5",
                 "distribution/entrypoints/%s is missing; every entrypoint block must "
                 "exist there during the duplicate window" % name)
            continue
        if not os.path.isfile(legacy):
            fail("payload-layout-contract L5",
                 "scripts/%s has no distribution counterpart to compare; every "
                 "entrypoint block must still exist there during the duplicate "
                 "window" % name)
            continue
        with open(moved, "rb") as left, open(legacy, "rb") as right:
            if left.read() != right.read():
                fail("payload-layout-contract L5",
                     "distribution/entrypoints/%s differs from scripts/%s during "
                     "the duplicate window" % (name, name))


def check_legacy_directory():
    """The mirror carries no legacy directory name."""
    if os.path.isdir(os.path.join(PAYLOAD, LEGACY_DIRECTORY)):
        fail("payload-layout-contract L7",
             "distribution/payload/%s exists; the installed destination prefix is "
             "%s since decision 0008, and a legacy directory in the mirror means a "
             "source was not renamed with its destination"
             % (LEGACY_DIRECTORY, NEW_PREFIX.rstrip("/")) )


def main():
    declarations = generated()
    entries = manifest_entries()
    check_marker()
    check_legacy_directory()
    check_mapping(declarations, entries)
    check_generators(declarations)
    check_generated_binding(declarations)
    check_duplicate_window(entries)
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
# its manifests, crates/truss/assets for the asset-sourced counterparts, the Rust
# source the generated-binding rule reads, and the pre-refactor destination
# copies, so that a mutation makes exactly one rule fire instead of also tripping
# the duplicate-window rule on missing files.
fixture() { # name
  local name="$1"
  local manifest dest rest legacy
  rm -rf "$WORK/$name"
  mkdir -p "$WORK/$name"
  cp -a "$ROOT/distribution" "$WORK/$name/distribution"
  cp -a "$ROOT/scripts" "$WORK/$name/scripts"
  cp -a "$ROOT/crates" "$WORK/$name/crates"
  # L5 compares the mirror against the pre-0008 tree, so a fixture root has to carry
  # that tree as well, or every fixture reports a missing counterpart.
  [ -d "$ROOT/.truss-core" ] && cp -a "$ROOT/.truss-core" "$WORK/$name/.truss-core"
  # The comparison side of the duplicate window is the pre-0008 tree, related to a
  # renamed destination by one rule: .truss/core/<rest> <- .truss-core/<rest>. The
  # four asset-sourced destinations come from crates/truss/assets instead, which the
  # copy above already provides.
  for manifest in "$ROOT"/scripts/*-install-files.txt; do
    while IFS= read -r dest || [ -n "$dest" ]; do
      case "$dest" in ""|\#*) continue ;; esac
      rest="${dest#.truss/core/}"
      case "$dest" in
        .truss/core/docs/communication.md|.truss/core/docs/plans/README.md|.truss/core/docs/plans/completed/README.md|.truss/core/docs/decisions/README.md)
          legacy="crates/truss/assets/.truss-core/docs/${rest#docs/}"
          ;;
        .truss/core/*)
          legacy=".truss-core/${rest}"
          ;;
        *)
          legacy="$dest"
          ;;
      esac
      [ -e "$ROOT/$legacy" ] || continue
      mkdir -p "$WORK/$name/$(dirname "$dest")"
      cp -p "$ROOT/$legacy" "$WORK/$name/$dest"
    done < "$manifest"
  done
  printf '%s\n' "$WORK/$name"
}

# --- L1: the layout marker ---------------------------------------------------
rm -f "$(fixture l1)/distribution/layout-version"
neg "a missing layout marker is rejected" "L1" run_root "$WORK/l1"

printf '9\n' > "$(fixture l1b)/distribution/layout-version"
neg "an unknown layout marker is rejected" "L1" run_root "$WORK/l1b"

# --- L2: resolution, cardinality, and reverse membership --------------------
rm -f "$(fixture l2)/distribution/payload/.truss/core/docs/WORKFLOW.md"
neg "a destination with no source is rejected" "L2" run_root "$WORK/l2"

# A generated destination that also exists as a payload file resolves twice. The
# copy carries the root bytes so that only the two-sources rule fires.
cp -p "$ROOT/AGENTS.md" "$(fixture l2b)/distribution/payload/AGENTS.md"
neg "a destination with two sources is rejected" "L2" run_root "$WORK/l2b"

# The second declaration repeats a generator and prefix the CLI actually uses, so
# the duplicate-declaration rule fires alone rather than together with L6.
printf 'AGENTS.md\tdistribution/entrypoints/agent-truss-block.md\t# Agent Instructions\\n\\n\n' \
  >> "$(fixture l2d)/distribution/generated.txt"
neg "a destination declared twice is rejected" "L2" run_root "$WORK/l2d"

printf 'unowned.md\tdistribution/entrypoints/agent-truss-block.md\t# Agent Instructions\\n\\n\n' \
  >> "$(fixture l2u)/distribution/generated.txt"
neg "a declaration no manifest lists is rejected" "L2" run_root "$WORK/l2u"

tail -n 1 "$ROOT/scripts/plan-install-files.txt" \
  >> "$(fixture l2m)/scripts/plan-install-files.txt"
neg "a destination listed twice in the manifests is rejected" "L2" run_root "$WORK/l2m"

# --- L3: payload membership -------------------------------------------------
printf 'orphan\n' > "$(fixture l3)/distribution/payload/.truss/core/docs/orphan.md"
neg "a payload file matching no destination is rejected" "L3" run_root "$WORK/l3"

# --- L4: generator containment ---------------------------------------------
python3 - "$(fixture l4)/distribution/generated.txt" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
open(path, "w", encoding="utf-8").write(text.replace("distribution/entrypoints/", "scripts/"))
PY
neg "a generator input outside distribution/entrypoints is rejected" "L4" run_root "$WORK/l4"

printf 'AGENTS.md\tdistribution/entrypoints/../../scripts/agent-truss-block.md\t# Agent Instructions\\n\\n\n' \
  > "$(fixture l4t)/distribution/generated.txt"
neg "a generator input escaping by traversal is rejected" "L4" run_root "$WORK/l4t"

ln -s ../../scripts/agent-truss-block.md \
  "$(fixture l4s)/distribution/entrypoints/leaving.md"
printf 'AGENTS.md\tdistribution/entrypoints/leaving.md\t# Agent Instructions\\n\\n\n' \
  > "$WORK/l4s/distribution/generated.txt"
neg "a generator input escaping by symlink is rejected" "L4" run_root "$WORK/l4s"

# --- L5: the duplicate window ----------------------------------------------
printf 'diverged\n' >> "$(fixture l5)/distribution/payload/.truss/core/docs/README.md"
neg "a payload copy diverging from its pre-refactor counterpart is rejected" "L5" run_root "$WORK/l5"

rm -f "$(fixture l5e)/distribution/entrypoints/claude-truss-block.md"
neg "a missing entrypoint copy is rejected" "L5" run_root "$WORK/l5e"

# --- L6: the generated declaration binds the CLI literals ------------------
python3 - "$(fixture l6p)/distribution/generated.txt" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
open(path, "w", encoding="utf-8").write(text.replace("# Agent Instructions", "# Other Instructions"))
PY
neg "a declared prefix the CLI does not write is rejected" "L6" run_root "$WORK/l6p"

python3 - "$(fixture l6g)/distribution/generated.txt" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
open(path, "w", encoding="utf-8").write(text.replace("agent-truss-block.md", "claude-truss-block.md"))
PY
neg "a declared generator the CLI does not use is rejected" "L6" run_root "$WORK/l6g"

# --- L7: no legacy directory in the mirror ---------------------------------
mkdir -p "$(fixture l7)/distribution/payload/.truss-core/docs"
printf 'legacy\n' > "$WORK/l7/distribution/payload/.truss-core/docs/WORKFLOW.md"
neg "a legacy directory left in the payload mirror is rejected" "L7" run_root "$WORK/l7"

echo
echo "== payload-layout-contract summary: $OK ok, $BAD failed =="
[ "$BAD" = 0 ]
