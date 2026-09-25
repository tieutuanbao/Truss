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
#   L5  the payload mirror's bytes are the recorded bytes: every payload file's
#       sha256 equals its entry in tests/payload-layout-digests.txt, and the
#       mirror's file set equals the manifest destination set minus the generated
#       one. Both entrypoint copies must exist and each must be byte-identical
#       to its scripts/ counterpart, which holds a straight copy for the shell
#       route until the duplicate-removal plan retires it.
#   L6  the generated declaration agrees with the literals the CLI composes
#       AGENTS.md from.
#   L7  the payload mirror carries no .truss-core directory; the installed
#       destination prefix is .truss/core since decision 0008.
#
# L5 no longer compares the mirror against the stale pre-0008 root tree. Decision
# 0008 renamed the installed prefix and rewrote the shipped prose, so the root
# tree and crates/truss/assets are stale by design until the duplicate-removal
# plan deletes them, and they are not authoritative content. The recorded digest
# file is the drift guard in their place: it was generated from the mirror at the
# commit that landed the rename and is reviewed as data, so any later content
# change in the mirror fails L5 instead of being normalised away.
#
# Maintaining the digest baseline: tests/payload-layout-digests.txt is a reviewed
# baseline, not a generated report. When a payload change is legitimate, regenerate
# it and commit the new values in the same commit as the payload change:
#
#   python3 - <<'PY'
#   import hashlib, os
#   roots = ("distribution/payload", "distribution/entrypoints")
#   rows = []
#   for root in roots:
#       for base, _, names in os.walk(root):
#           for name in names:
#               path = os.path.join(base, name)
#               with open(path, "rb") as handle:
#                   rows.append((path, hashlib.sha256(handle.read()).hexdigest()))
#   with open("tests/payload-layout-digests.txt", "w", encoding="utf-8") as out:
#       for path, digest in sorted(rows):
#           out.write("%s  %s\n" % (digest, path))
#   PY
#
# It covers both distribution/payload/** and distribution/entrypoints/**, so no
# entrypoint can drift outside the baseline. The loader skips blank lines only, so
# the file itself carries no comment header: a '#' line is not a valid record.
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
import hashlib
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

# The installed destination prefix decision 0008 introduced, and the legacy
# directory name it replaced. L7 uses the legacy name to prove the mirror carries
# no directory under it; the two prefixes are the trailing-slash forms L5 un-applies
# when it compares the generator entrypoint against its scripts/ counterpart.
NEW_PREFIX = ".truss/core/"
LEGACY_PREFIX = ".truss-core/"
LEGACY_DIRECTORY = ".truss-core"

# The recorded byte expectations for the mirror, generated from it at the commit
# that landed the rename and reviewed as data. A later edit to a payload file must
# fail L5 until the expectation is updated deliberately.
DIGESTS = os.path.join(ROOT, "tests", "payload-layout-digests.txt")


def sha256_of(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()

def read_bytes(path):
    with open(path, "rb") as handle:
        return handle.read()


def recorded_digests():
    """The recorded expectations as {repo-relative path: sha256}, in file order."""
    if not os.path.isfile(DIGESTS):
        fail("payload-layout-contract L5",
             "tests/payload-layout-digests.txt is missing; the mirror's bytes are "
             "unverified without the recorded expectations")
        return {}
    recorded = {}
    with open(DIGESTS, encoding="utf-8") as handle:
        for number, line in enumerate(handle, 1):
            line = line.rstrip("\n")
            if not line.strip():
                continue
            fields = line.split("  ", 1)
            if len(fields) != 2 or len(fields[0]) != 64:
                fail("payload-layout-contract L5",
                     "tests/payload-layout-digests.txt:%d is not "
                     "'<sha256>  <repo-relative path>'" % number)
                continue
            digest, path = fields
            if path in recorded:
                fail("payload-layout-contract L5",
                     "tests/payload-layout-digests.txt:%d records %s twice"
                     % (number, path))
                continue
            recorded[path] = digest
    return recorded

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


def check_payload_digests(entries, recorded):
    """Every mirror file and entrypoint block matches its recorded digest.

    Replaces the pre-0008 counterpart comparison: after decision 0008 those trees
    are stale by design and no longer authoritative content, so the recorded
    digests are the drift guard until the duplicate-removal plan deletes the
    counterparts. The entrypoint blocks are generator inputs rather than manifest
    destinations, so digest coverage is what proves their shipped bytes.

    Digest coverage alone does not prove the entrypoints still correspond to the
    scripts/ files the installers read, so `check_entrypoint_counterparts` asserts
    that relation separately.
    """
    expected = {}
    for _, _, dest in entries:
        path = os.path.join(PAYLOAD, dest)
        if os.path.isfile(path):
            expected["distribution/payload/" + dest] = path
    for name in ENTRYPOINT_BLOCKS:
        path = os.path.join(ENTRY, name)
        if os.path.isfile(path):
            expected["distribution/entrypoints/" + name] = path

    for path in sorted(recorded):
        if path not in expected:
            fail("payload-layout-contract L5",
                 "tests/payload-layout-digests.txt records %s, which the distribution "
                 "tree does not hold; the recorded expectations and the tree must "
                 "describe the same file set" % path)
    for path in sorted(expected):
        if path not in recorded:
            fail("payload-layout-contract L5",
                 "%s carries no recorded digest; a shipped file must have a reviewed "
                 "expectation" % path)
            continue
        actual = sha256_of(expected[path])
        if actual != recorded[path]:
            fail("payload-layout-contract L5",
                 "%s has digest %s while tests/payload-layout-digests.txt records %s; "
                 "the shipped bytes moved since the rename landed"
                 % (path, actual, recorded[path]))


def check_entrypoints_present():
    """Both entrypoint blocks exist, because a missing generator input is silent."""
    for name in ENTRYPOINT_BLOCKS:
        if not os.path.isfile(os.path.join(ENTRY, name)):
            fail("payload-layout-contract L5",
                 "distribution/entrypoints/%s is missing; every entrypoint block must "
                 "exist there" % name)


def check_entrypoint_counterparts():
    """Each entrypoint copy still corresponds to the scripts/ file it replaces.

    Both blocks now carry the canonical new-root paths, and scripts/ holds a
    straight copy of each block for the shell route
    (scripts/install-truss.sh:182), so each pair must be byte-identical: any
    divergence between the two copies of one block is rejected. The
    duplicate-removal plan retires the scripts/ copies and points the shell
    route at distribution/entrypoints.
    """
    for name in ENTRYPOINT_BLOCKS:
        moved = os.path.join(ENTRY, name)
        counterpart = os.path.join(ROOT, "scripts", name)
        if not os.path.isfile(moved) or not os.path.isfile(counterpart):
            continue
        moved_bytes = read_bytes(moved)
        counterpart_bytes = read_bytes(counterpart)
        if moved_bytes != counterpart_bytes:
            fail("payload-layout-contract L5",
                 "distribution/entrypoints/%s differs from scripts/%s; both copies of "
                 "one block must be byte-identical while the duplicate window lasts"
                 % (name, name))


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
    recorded = recorded_digests()
    check_marker()
    check_legacy_directory()
    check_mapping(declarations, entries)
    check_generators(declarations)
    check_generated_binding(declarations)
    check_payload_digests(entries, recorded)
    check_entrypoints_present()
    check_entrypoint_counterparts()
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
# its manifests and for the two entrypoint counterparts, the Rust source the
# generated-binding rule reads, and the recorded digest file L5 verifies against,
# so that a mutation makes exactly one rule fire instead of also tripping another
# rule on missing files.
fixture() { # name
  local name="$1"
  rm -rf "$WORK/$name"
  mkdir -p "$WORK/$name"
  cp -a "$ROOT/distribution" "$WORK/$name/distribution"
  cp -a "$ROOT/scripts" "$WORK/$name/scripts"
  cp -a "$ROOT/crates" "$WORK/$name/crates"
  mkdir -p "$WORK/$name/tests"
  cp -p "$ROOT/tests/payload-layout-digests.txt" "$WORK/$name/tests/payload-layout-digests.txt"
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

# --- L5: the mirror's bytes against the recorded expectations --------------
# A payload file edited in place moves its digest away from the recorded one.
printf 'diverged\n' >> "$(fixture l5)/distribution/payload/.truss/core/docs/README.md"
neg "a payload file whose digest moved is rejected" "L5" run_root "$WORK/l5"

# A rename-shaped edit: replace one occurrence of the legacy prefix in a file that
# should no longer contain it. The digest catches it because the recorded
# expectation was taken after the rename, which is exactly what the normalising
# comparison could not see.
python3 - "$(fixture l5r)/distribution/payload/.truss/core/docs/WORKFLOW.md" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
open(path, "w", encoding="utf-8").write(text.replace(".truss/core/docs/", ".truss-core/docs/", 1))
PY
neg "a rename-shaped edit in a payload file is rejected" "L5" run_root "$WORK/l5r"

# A payload file with no recorded expectation, and a recorded expectation with no
# payload file: the two sides must describe the same set. The first fixture drops a
# line from the expectations file rather than adding a stray file, so L5 fires
# alone instead of L3 also reporting an unlisted payload file.
grep -v 'docs/README.md$' "$ROOT/tests/payload-layout-digests.txt" \
  > "$(fixture l5u)/tests/payload-layout-digests.txt"
neg "a payload file with no recorded digest is rejected" "L5" run_root "$WORK/l5u"

printf '%s  %s\n' "0000000000000000000000000000000000000000000000000000000000000000" \
  ".truss/core/docs/absent.md" >> "$(fixture l5m)/tests/payload-layout-digests.txt"
neg "a recorded digest with no payload file is rejected" "L5" run_root "$WORK/l5m"

rm -f "$(fixture l5e)/distribution/entrypoints/claude-truss-block.md"
neg "a missing entrypoint copy is rejected" "L5" run_root "$WORK/l5e"

# The entrypoint blocks are generator inputs, not manifest destinations, so their
# bytes are guarded by their own recorded digests. A mutated block must fail.
printf '\n<!-- drifted -->\n' >> "$(fixture l5d)/distribution/entrypoints/agent-truss-block.md"
neg "an entrypoint block whose digest moved is rejected" "L5" run_root "$WORK/l5d"

# Digest coverage proves the block's shipped bytes, not that the copy still
# corresponds to the scripts/ file its consumer reads. Repoint only the scripts/
# counterpart of one entrypoint at the legacy text: the digest side is untouched,
# so only the counterpart relation can catch it.
printf '.truss-core/docs/WORKFLOW.md\n' > "$(fixture l5c)/scripts/claude-truss-block.md"
neg "a Claude entrypoint copy diverging from its scripts/ counterpart is rejected" \
  "L5" run_root "$WORK/l5c"

# The generator entrypoint is compared after un-applying the rename, so the
# permitted difference is exactly the rename. Mutate it in a way the rename does
# not explain, leaving scripts/ alone: the digest side is untouched.
python3 - "$(fixture l5a)/distribution/entrypoints/agent-truss-block.md" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
open(path, "w", encoding="utf-8").write(text.replace("No control-plane operation", "No plane operation", 1))
PY
neg "a generator entrypoint diverging from its scripts/ counterpart beyond the rename is rejected" \
  "L5" run_root "$WORK/l5a"

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
