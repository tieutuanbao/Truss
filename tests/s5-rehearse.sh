#!/usr/bin/env bash
# S5 rehearsal — installer delegation and platform parity.
#
# One command, runnable from a fresh checkout of the candidate revision:
#
#   bash tests/s5-rehearse.sh
#
# It builds the Truss CLI when it is missing, then exercises the committed
# installers against throwaway workspaces under $TMPDIR and asserts, for every
# check, an observable result. It prints one line per check and a final
# `== rehearsal summary: N ok, M failed ==`, and it exits non-zero if any check
# failed.
#
# Coverage:
#   Row 1  the bash installer delegates and the result is CLI-managed, from a
#          local checkout and from a raw source base URL pinned to the release
#   Row 2  a repeated run never skips and never clobbers (same ref, and a
#          consumer edit against a payload whose one path changed), with the
#          pre-change installer's --merge/--force counterexamples observed red
#          on the identical state
#   Row 3  PowerShell parity, static only: pwsh is not available here, so the
#          execution gap is declared and no byte-level parity is claimed
#   Override  the replaced installed root is moved into the backup at its
#          original relative path, including the two-segment `.truss/core`
#          root whose destination parent a one-level mkdir never created
#   Row A  this script is the committed instrument; `git ls-tree` proves it
#   Row B  all three add-ons (engineering-wisdom, delivery, planning) install
#          through the CLI, each judged against the path set its own existing
#          manifest declares — the manifests stay the membership owner, and no
#          second list is hard-coded here
#
# Requirements: bash, git, curl, python3, cargo (offline, --locked). No network:
# every source mode used here is a local path or a file:// URL.
#
# Note: the local source lanes require the add-on payload paths to be committed
# at HEAD, because the installer records an immutable ref that must describe the
# bytes it installs. Run the rehearsal on a candidate revision whose add-on
# payload is committed, as it is in the repository. The expected local ref
# mirrors the installer: a checkout whose HEAD carries the tag declared in
# scripts/truss-release-tag is a released source and records that tag, and every
# other checkout records the exact HEAD commit SHA.
#
# Environment knobs: S5_KEEP_FIXTURES=1 keeps the throwaway fixture directory,
# S5_EVIDENCE_DIR overrides where raw outputs are copied (default
# target/s5-evidence).
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO" || exit 1

EVIDENCE="${S5_EVIDENCE_DIR:-$REPO/target/s5-evidence}"
EV="$(mktemp -d "${TMPDIR:-/tmp}/s5-rehearse.XXXXXX")"
OK=0
BAD=0
PAYLOAD_TOUCHED=0

MANIFEST_ENGINEERING_WISDOM="scripts/engineering-wisdom-install-files.txt"
MANIFEST_DELIVERY="scripts/delivery-install-files.txt"
MANIFEST_PLANNING="scripts/plan-install-files.txt"
PAYLOAD_PROBE=".agents/skills/delivery/references/trusses.md"
# The same path as a mirror source. The dirty-payload refusal judges the bytes the
# installer stages, which live under distribution/payload/, not the destination.
PAYLOAD_SOURCE_PROBE="distribution/payload/$PAYLOAD_PROBE"
# The delivery lane's expected counts come from the delivery manifest, never
# from a literal, so a manifest change cannot leave a stale constant behind.
DELIVERY_PATHS="$(awk 'NF && $1 !~ /^#/ { n++ } END { print n + 0 }' "$MANIFEST_DELIVERY")"

cleanup() {
  if [ "$PAYLOAD_TOUCHED" = 1 ]; then
    cp "$EV/trusses.md.bak" "$PAYLOAD_SOURCE_PROBE"
  fi
  # The pre-change baseline worktree is registered in this repository's Git
  # metadata, so it must be removed as a worktree and not only as a directory,
  # or a stale registration survives into the next run.
  if [ -n "${OLD_CLI_WORKTREE:-}" ] && [ -d "$OLD_CLI_WORKTREE" ]; then
    git worktree remove --force "$OLD_CLI_WORKTREE" >/dev/null 2>&1 || true
    git worktree prune >/dev/null 2>&1 || true
  fi
  mkdir -p "$EVIDENCE/rehearse-raw"
  cp "$EV"/*.txt "$EVIDENCE/rehearse-raw/" 2>/dev/null
  if [ "${S5_KEEP_FIXTURES:-0}" = 1 ]; then
    echo "fixtures kept: $EV"
  else
    rm -rf "$EV"
  fi
}
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

fail_setup() {
  printf 'FAIL %s\n' "$1"
  printf '\n== rehearsal summary: %s ok, %s failed ==\n' "$OK" "$((BAD + 1))"
  exit 1
}

snap() {
  (cd "$1" && find . -type f -not -path './.truss-backup/*' | LC_ALL=C sort |
    while IFS= read -r f; do printf '%s %s\n' "$(sha256sum "$f" | cut -d' ' -f1)" "$f"; done)
}

file_hash() { sha256sum "$1" | cut -d' ' -f1; }

# One line per add-on: the record, the baseline, and the workspace are compared
# against the path set and the payload bytes the named manifest declares.
assert_addon_matches_manifest() { # ws name manifest expected_ref
  python3 - "$1" "$2" "$3" "$4" <<'PY'
import hashlib
import json
import os
import sys

ws, name, manifest, expected_ref = sys.argv[1:5]
paths = [line.strip() for line in open(manifest)
         if line.strip() and not line.strip().startswith('#')]
problems = []

record_path = os.path.join(ws, '.truss/core/addons.json')
try:
    state = json.load(open(record_path))
except Exception as error:  # noqa: BLE001 - reported as a failed check
    print('add-on %s: manifest %s -> unreadable record: %s' % (name, manifest, error))
    sys.exit(1)

addon = next((entry for entry in state['addons'] if entry['name'] == name), None)
if addon is None:
    problems.append('no record in .truss/core/addons.json')
else:
    if addon['source_ref'] != expected_ref:
        problems.append('recorded ref %s != %s' % (addon['source_ref'], expected_ref))
    recorded = [entry['path'] for entry in addon['files']]
    if recorded != paths:
        problems.append('recorded path set differs from the manifest (%d vs %d)' % (len(recorded), len(paths)))
    baseline_root = os.path.join(ws, '.truss/core/base-addons', name)
    if not os.path.isdir(baseline_root):
        problems.append('no baseline directory')
    for entry in addon['files']:
        for root in (ws, baseline_root):
            target = os.path.join(root, entry['path'])
            if not os.path.isfile(target):
                problems.append('missing %s' % os.path.relpath(target, ws))
                continue
            with open(target, 'rb') as handle:
                digest = hashlib.sha256(handle.read()).hexdigest()
            if digest != entry['upstream_sha256']:
                problems.append('digest mismatch %s' % os.path.relpath(target, ws))

baseline_root = os.path.join(ws, '.truss/core/base-addons', name)
baseline = sorted(os.path.relpath(os.path.join(root, name_), baseline_root)
                  for root, _, names in os.walk(baseline_root) for name_ in names)
if baseline != sorted(paths):
    problems.append('baseline path set differs from the manifest (%d vs %d)' % (len(baseline), len(paths)))

if problems:
    print('add-on %s: manifest %s paths=%d -> %s' % (name, manifest, len(paths), '; '.join(problems[:3])))
    sys.exit(1)
print('add-on %s: manifest %s paths=%d record=yes baseline=%d workspace=%d'
      % (name, manifest, len(paths), len(baseline), len(paths)))
PY
}

# The CLI's own status report must name the same path set and digests.
assert_addon_status_matches_manifest() { # status_json name manifest
  python3 - "$1" "$2" "$3" <<'PY'
import json
import sys

status_path, name, manifest = sys.argv[1:4]
paths = [line.strip() for line in open(manifest)
         if line.strip() and not line.strip().startswith('#')]
try:
    status = json.load(open(status_path))
except Exception as error:  # noqa: BLE001 - reported as a failed check
    print('add-on status %s: manifest %s -> unreadable status: %s' % (name, manifest, error))
    sys.exit(1)
record = status.get('record')
if not record:
    print('add-on status %s: manifest %s -> not recorded' % (name, manifest))
    sys.exit(1)
reported = [entry['path'] for entry in record['files']]
if reported != paths:
    print('add-on status %s: manifest %s -> status path set differs (%d vs %d)'
          % (name, manifest, len(reported), len(paths)))
    sys.exit(1)
if any(len(entry['sha256']) != 64 for entry in record['files']):
    print('add-on status %s: manifest %s -> status digest is not a sha256' % (name, manifest))
    sys.exit(1)
print('add-on status %s: manifest %s paths=%d ref=%s' % (name, manifest, len(reported), record['source_ref']))
PY
}

# --------------------------------------------------------------- fixtures ----
echo "candidate revision: $(git rev-parse HEAD)"
echo "candidate scripts:"
sha256sum scripts/install-truss.sh scripts/install-truss.ps1
echo

TARGET_ROOT="${CARGO_TARGET_DIR:-target}"
case "$TARGET_ROOT" in
  /*) ;;
  *) TARGET_ROOT="$REPO/$TARGET_ROOT" ;;
esac
CLI="$TARGET_ROOT/debug/truss"
if [ ! -x "$CLI" ]; then
  echo "building the Truss CLI for the rehearsal: cargo build --locked -p truss"
  cargo build --quiet --locked -p truss || fail_setup "could not build the Truss CLI"
fi
[ -x "$CLI" ] || fail_setup "the Truss CLI is not available at $CLI"

# The pre-change CLI, built from the same baseline commit whose installer the
# counterexample lanes below drive. The pre-change installer hardcodes
# `.truss-core/bin/truss` for the binary it stages and delegates only its core
# step to that CLI, so a faithful "pre-change installer on the identical state"
# pairing needs the binary from the same revision: paired with the post-0008 CLI
# it creates two trees and records no provenance, which is a different finding and
# is asserted separately. Built offline from the locked local commit, so the
# rehearsal stays deterministic and network-free. The worktree lives under the
# fixture directory and is deregistered by the cleanup trap, so no worktree of
# this fixture outlives the run and nothing is written under target/.
OLD_BASELINE="d9357b9"
OLD_CLI_WORKTREE="$EV/old-baseline"
OLD_CLI="$OLD_CLI_WORKTREE/target/debug/truss"
if [ ! -x "$OLD_CLI" ]; then
  echo "building the pre-change Truss CLI for the counterexample lanes: $OLD_BASELINE"
  git worktree add --detach --force "$OLD_CLI_WORKTREE" "$OLD_BASELINE" >/dev/null 2>&1 ||
    fail_setup "could not create a worktree for the pre-change baseline $OLD_BASELINE"
  (cd "$OLD_CLI_WORKTREE" && CARGO_TARGET_DIR="$OLD_CLI_WORKTREE/target" cargo build --quiet --locked --offline -p truss) ||
    fail_setup "could not build the pre-change Truss CLI from $OLD_BASELINE"
fi
[ -x "$OLD_CLI" ] || fail_setup "the pre-change Truss CLI is not available at $OLD_CLI"

copy_payload() { # dst
  local dst="$1"
  mkdir -p "$dst/scripts" "$dst/distribution/payload"
  cp scripts/delivery-install-files.txt "$dst/scripts/"
  cp distribution/layout-version "$dst/distribution/"
  # Each manifest line names a destination; the source bytes live in the mirror,
  # so the fixture copies mirror -> destination exactly as the installer stages.
  while IFS= read -r p; do
    case "$p" in ""|\#*) continue ;; esac
    mkdir -p "$dst/$(dirname "$p")" "$dst/distribution/payload/$(dirname "$p")"
    cp -p "$REPO/distribution/payload/$p" "$dst/distribution/payload/$p"
  done < scripts/delivery-install-files.txt
}

# payload repositories for the --source-git lanes: A is the candidate payload,
# B changes one path upstream
copy_payload "$EV/gitA"
git -C "$EV/gitA" init -q -b main
git -C "$EV/gitA" add -A
git -C "$EV/gitA" -c user.email=s5@example.com -c user.name=s5 commit -qm "payload A"
cp -a "$EV/gitA" "$EV/gitB"
printf '\n<!-- upstream change in the next release -->\n' >> "$EV/gitB/distribution/payload/$PAYLOAD_PROBE"
git -C "$EV/gitB" add -A
git -C "$EV/gitB" -c user.email=s5@example.com -c user.name=s5 commit -qm "payload B changes references/trusses.md"
EV_A_SHA="$(git -C "$EV/gitA" rev-parse HEAD)"
EV_B_SHA="$(git -C "$EV/gitB" rev-parse HEAD)"
echo "payload A ref: $EV_A_SHA"
echo "payload B ref: $EV_B_SHA"

# released-source layout pinned to the release tag, and two broken variants
copy_payload "$EV/raw/truss-v0.1.13"
printf 'truss-v0.1.13\n' > "$EV/raw/truss-v0.1.13/scripts/truss-release-tag"
cp -a "$EV/raw/truss-v0.1.13" "$EV/raw/main"
mkdir -p "$EV/rawm/truss-v0.1.13"
cp -a "$EV/raw/truss-v0.1.13/." "$EV/rawm/truss-v0.1.13/"
printf 'truss-v0.1.14\n' > "$EV/rawm/truss-v0.1.13/scripts/truss-release-tag"

# a local source tree that is not a git checkout
mkdir -p "$EV/nogit/scripts" "$EV/nogit/.truss/core/docs"
cp scripts/install-truss.sh "$EV/nogit/scripts/"
cp AGENTS.md "$EV/nogit/"
cp .truss-core/docs/TRUSS.md "$EV/nogit/.truss/core/docs/"

# a copy of the installer outside the repository, so it runs in raw-URL mode
cp scripts/install-truss.sh "$EV/raw/installer-remote.sh"

# A legacy-layout source pinned to the same release tag, for the counterexample
# lanes below. The pre-change installer resolves each destination relative to the
# source base URL, so its fixture carries the payload bytes at the root in the
# pre-0008 shape; the directory name keeps the tag-pinned form that installer
# requires. Nothing here reads this candidate's root copies.
copy_legacy_payload() { # dst
  local dst="$1"
  mkdir -p "$dst/scripts"
  cp scripts/delivery-install-files.txt "$dst/scripts/"
  printf 'truss-v0.1.13\n' > "$dst/scripts/truss-release-tag"
  while IFS= read -r p; do
    case "$p" in ""|\#*) continue ;; esac
    mkdir -p "$dst/$(dirname "$p")"
    cp -p "$REPO/distribution/payload/$p" "$dst/$p"
  done < scripts/delivery-install-files.txt
}
copy_legacy_payload "$EV/rawlegacy/truss-v0.1.13"
# The pre-change installer resolves its payload relative to a source checkout and
# clones that source with `--source-git`, so the counterexample runs need real git
# revisions in the legacy layout, not only the plain directory above. Revision B
# carries a changed upstream path, so the pre-change installer meets a path whose
# upstream bytes moved.
copy_legacy_payload "$EV/legacyA"
git -C "$EV/legacyA" init -q -b main
git -C "$EV/legacyA" add -A
git -C "$EV/legacyA" -c user.email=s5@example.com -c user.name=s5 commit -qm "legacy payload A"
cp -a "$EV/legacyA" "$EV/legacyB"
printf '\n<!-- upstream change in the next release -->\n' >> "$EV/legacyB/$PAYLOAD_PROBE"
git -C "$EV/legacyB" add -A
git -C "$EV/legacyB" -c user.email=s5@example.com -c user.name=s5 commit -qm "legacy payload B moves the probed path"

cp "$PAYLOAD_SOURCE_PROBE" "$EV/trusses.md.bak"
echo

# ------------------------------------------------------------ Row A ---------
echo "== Row A: the rehearsal is a committed instrument =="
if git ls-tree -r --name-only HEAD | grep -qx 'tests/s5-rehearse.sh'; then
  check "git ls-tree -r --name-only HEAD lists tests/s5-rehearse.sh" 0
else
  check "git ls-tree -r --name-only HEAD lists tests/s5-rehearse.sh" 1
fi
check "the rehearsal is running from the declared path" \
  "$([ -f "$REPO/tests/s5-rehearse.sh" ]; echo $?)"
echo

# ------------------------------------------------------------ Row 1 ---------
echo "== Row 1: the bash installer delegates and the result is CLI-managed =="
W1="$EV/w1"
scripts/install-truss.sh --directory "$W1" --with-delivery --yes > "$EV/row1-install.txt" 2>&1
st=$?
check "installer exits 0" "$st"
HEAD_SHA="$(git rev-parse HEAD)"
RELEASE_TAG_FILE="$(awk 'NF && $1 !~ /^#/ { print $1; exit }' scripts/truss-release-tag 2>/dev/null)"
LOCAL_SOURCE_REF="$HEAD_SHA"
if [ -n "$RELEASE_TAG_FILE" ] && git tag --points-at HEAD 2>/dev/null | grep -Fxq "$RELEASE_TAG_FILE"; then
  LOCAL_SOURCE_REF="$RELEASE_TAG_FILE"
fi
check "addons.json records the add-on" "$([ -f "$W1/.truss/core/addons.json" ]; echo $?)"
check "recorded ref is the resolved local source ref (release tag on a tagged HEAD, else HEAD SHA)" \
  "$([ "$(python3 -c "import json;print(json.load(open('$W1/.truss/core/addons.json'))['addons'][0]['source_ref'])")" = "$LOCAL_SOURCE_REF" ]; echo $?)"
check "recorded digest count equals the delivery manifest's path count" \
  "$([ "$(python3 -c "import json;print(len(json.load(open('$W1/.truss/core/addons.json'))['addons'][0]['files']))")" = "$DELIVERY_PATHS" ]; echo $?)"
check "baseline holds every payload path" \
  "$([ "$(find "$W1/.truss/core/base-addons/delivery" -type f | wc -l)" = "$DELIVERY_PATHS" ]; echo $?)"
check "every delivery manifest path is present in the workspace" \
  "$(python3 -c "
import os
import sys
paths = [line.strip() for line in open(sys.argv[2]) if line.strip() and not line.strip().startswith('#')]
print(0 if all(os.path.isfile(os.path.join(sys.argv[1], path)) for path in paths) else 1)" "$W1" "$MANIFEST_DELIVERY")"
check "installer output names the CLI invocation" \
  "$(grep -q 'addon install --name delivery --manifest' "$EV/row1-install.txt"; echo $?)"
check "no direct-copy helper survives in the bash installer" \
  "$([ "$(grep -c 'copy_file\|copy_manifest_files\|write_source_file' scripts/install-truss.sh)" = 0 ]; echo $?)"
"$W1/.truss/core/bin/truss" addon status --name delivery --directory "$W1" --json > "$EV/row1-status.json" 2>&1
check "a following addon status reports the record" \
  "$(python3 -c "import json,sys;d=json.load(open('$EV/row1-status.json'));print(0 if d['record'] and len(d['record']['files'])==int(sys.argv[1]) else 1)" "$DELIVERY_PATHS")"
check "workspace, baseline, and record agree" \
  "$(python3 -c "
import json,hashlib,os
w='$W1'
a=json.load(open(os.path.join(w,'.truss/core/addons.json')))['addons'][0]
bad=0
for f in a['files']:
    p=os.path.join(w,f['path']); b=os.path.join(w,'.truss/core/base-addons',a['name'],f['path'])
    for q in (p,b):
        if hashlib.sha256(open(q,'rb').read()).hexdigest()!=f['upstream_sha256']: bad=1
    if not os.path.exists(b): bad=1
print(bad)")"
echo

# ------------------------------------------------------------ Row 2 ---------
echo "== Row 2 rehearsal 1: rerun with the same ref never skips and never clobbers =="
snap "$W1" > "$EV/row2-before.tree"
ADDONS_BEFORE="$(file_hash "$W1/.truss/core/addons.json")"
scripts/install-truss.sh --directory "$W1" --with-delivery --merge --yes > "$EV/row2-rerun.txt" 2>&1
st=$?
check "rerun exits 0" "$st"
snap "$W1" > "$EV/row2-after.tree"
check "managed workspace and baseline byte-identical (only .truss-backup is new)" \
  "$(diff -q "$EV/row2-before.tree" "$EV/row2-after.tree" >/dev/null; echo $?)"
check "addons.json byte-identical" "$([ "$(file_hash "$W1/.truss/core/addons.json")" = "$ADDONS_BEFORE" ]; echo $?)"
check "rerun reports preserve for the unchanged payload" \
  "$(grep -q 'preserve .agents/skills/delivery/references/trusses.md' "$EV/row2-rerun.txt"; echo $?)"
check "the old merge-skip wording is absent from the add-on step" \
  "$([ "$(grep -c 'merge keeps existing file' "$EV/row2-rerun.txt")" = 0 ]; echo $?)"
echo

echo "== Row 2 rehearsal 2: consumer edit plus a payload whose one path changed =="
TRUSS_CORE_BINARY="$CLI" scripts/install-truss.sh --directory "$EV/w2" --with-delivery --yes \
  --source-git "file://$EV/gitA" > "$EV/row2b-install.txt" 2>&1
st=$?
check "install from payload A exits 0" "$st"
check "recorded ref is payload A's commit SHA" \
  "$([ "$(python3 -c "import json;print(json.load(open('$EV/w2/.truss/core/addons.json'))['addons'][0]['source_ref'])")" = "$EV_A_SHA" ]; echo $?)"
CONFLICT_PATH="$EV/w2/$PAYLOAD_PROBE"
printf '\n<!-- consumer local edit -->\n' >> "$CONFLICT_PATH"
CONSUMER="$(file_hash "$CONFLICT_PATH")"
rm -rf "$EV/w2/.truss-backup"
snap "$EV/w2" > "$EV/row2b-before.tree"
TRUSS_CORE_BINARY="$CLI" scripts/install-truss.sh --directory "$EV/w2" --with-delivery --merge --yes \
  --source-git "file://$EV/gitB" > "$EV/row2b-rerun.txt" 2>&1
st=$?
check "rerun against the changed payload stops non-zero" "$([ "$st" -ne 0 ]; echo $?)"
check "the CLI reported the conflict and staged a resolution" \
  "$(grep -q "conflict $PAYLOAD_PROBE" "$EV/row2b-rerun.txt"; echo $?)"
check "consumer bytes survive in the workspace" "$([ "$(file_hash "$CONFLICT_PATH")" = "$CONSUMER" ]; echo $?)"
check "nothing else moved: only the owned session appeared" \
  "$(diff "$EV/row2b-before.tree" <(snap "$EV/w2") | grep '^[<>]' | grep -v 'addon-update' | grep -q .; [ $? -ne 0 ]; echo $?)"
check "recorded ref still payload A" \
  "$([ "$(python3 -c "import json;print(json.load(open('$EV/w2/.truss/core/addons.json'))['addons'][0]['source_ref'])")" = "$EV_A_SHA" ]; echo $?)"
check "resolved path staged for the conflict" \
  "$([ -f "$EV/w2/.truss/core/addon-update/delivery/resolved/$PAYLOAD_PROBE" ]; echo $?)"
echo

# -------------------------------------------- Row 2 gitignore idempotency -----
# The two installers share one missing/skip rule for the root .gitignore: three
# required entries, skip only when all three are present, append only the missing
# ones. A run that finds one rule missing beside an existing marker used to append
# the marker a second time, so the file grew on every run. This lane installs twice
# into the same target and compares the file, and asserts each entry appears once.
echo "== Row 2 gitignore idempotency: the binary rules are appended once and never duplicated =="
WGI="$EV/w-gitignore"
TRUSS_CORE_BINARY="$CLI" scripts/install-truss.sh --directory "$WGI" --yes > "$EV/gitignore-first.txt" 2>&1
st=$?
check "first install into a fresh target exits 0" "$st"
GITIGNORE_FIRST="$(file_hash "$WGI/.gitignore")"
TRUSS_CORE_BINARY="$CLI" scripts/install-truss.sh --directory "$WGI" --merge --yes > "$EV/gitignore-second.txt" 2>&1
st=$?
check "second install into the same target exits 0" "$st"
check "the root .gitignore is byte-identical after the second install" \
  "$([ "$(file_hash "$WGI/.gitignore")" = "$GITIGNORE_FIRST" ]; echo $?)"
check "the marker line appears exactly once" \
  "$([ "$(grep -Fxc '# Truss core maintenance binary' "$WGI/.gitignore")" = 1 ]; echo $?)"
check "the Unix binary rule appears exactly once" \
  "$([ "$(grep -Fxc '.truss/core/bin/truss' "$WGI/.gitignore")" = 1 ]; echo $?)"
check "the Windows binary rule appears exactly once" \
  "$([ "$(grep -Fxc '.truss/core/bin/truss.exe' "$WGI/.gitignore")" = 1 ]; echo $?)"
# The same rule must repair a file that already carries the marker beside only one
# rule, which is the exact shape that used to duplicate the marker.
printf '# Truss core maintenance binary\n.truss/core/bin/truss.exe\n' > "$WGI/.gitignore"
TRUSS_CORE_BINARY="$CLI" scripts/install-truss.sh --directory "$WGI" --merge --yes > "$EV/gitignore-repair.txt" 2>&1
st=$?
check "repairing a partially written ignore file exits 0" "$st"
check "repairing does not duplicate the existing marker" \
  "$([ "$(grep -Fxc '# Truss core maintenance binary' "$WGI/.gitignore")" = 1 ]; echo $?)"
check "repairing adds the one missing rule" \
  "$([ "$(grep -Fxc '.truss/core/bin/truss' "$WGI/.gitignore")" = 1 ]; echo $?)"
echo

# ------------------------------------------------------------ Row B ---------
echo "== Row B: all three add-ons install through the CLI from their own manifest =="
WALL="$EV/w-all"
scripts/install-truss.sh --directory "$WALL" --with-engineering-wisdom --with-delivery --with-planning --yes \
  > "$EV/all-install.txt" 2>&1
st=$?
check "installer exits 0 with all three add-ons requested" "$st"
check "the run named one CLI invocation per add-on" \
  "$([ "$(grep -c 'via the Truss CLI' "$EV/all-install.txt")" = 3 ]; echo $?)"
check "the three manifests declare three different path sets" \
  "$(python3 -c "
import sys
def paths(manifest):
    return [l.strip() for l in open(manifest) if l.strip() and not l.strip().startswith('#')]
sets = [paths(m) for m in sys.argv[1:]]
print(0 if len(sets) == 3 and len({tuple(s) for s in sets}) == 3 else 1)" \
    "$MANIFEST_ENGINEERING_WISDOM" "$MANIFEST_DELIVERY" "$MANIFEST_PLANNING")"
for spec in "$MANIFEST_ENGINEERING_WISDOM:engineering-wisdom" "$MANIFEST_DELIVERY:delivery" "$MANIFEST_PLANNING:planning"; do
  manifest="${spec%%:*}"
  name="${spec#*:}"
  line="$(assert_addon_matches_manifest "$WALL" "$name" "$manifest" "$LOCAL_SOURCE_REF")"
  st=$?
  check "$line" "$st"
  "$WALL/.truss/core/bin/truss" addon status --name "$name" --directory "$WALL" --json > "$EV/status-$name.json" 2>&1
  line="$(assert_addon_status_matches_manifest "$EV/status-$name.json" "$name" "$manifest")"
  st=$?
  check "$line" "$st"
done
# Counterexample observed red: a record judged against another add-on's manifest
# must be rejected, so repeating one add-on's case under a different name cannot
# pass a bare "three installs" count.
line="$(assert_addon_matches_manifest "$WALL" delivery "$MANIFEST_PLANNING" "$LOCAL_SOURCE_REF")"
st=$?
check "the per-manifest assertion rejects a record judged against another manifest" "$([ "$st" -ne 0 ]; echo $?)"
echo

# ----------------------------------------------------- Row 1 (released) -----
echo "== Row 1 (released source): raw base URL pinned to the release tag =="
TRUSS_CORE_BINARY="$CLI" TRUSS_SOURCE_BASE_URL="file://$EV/raw/truss-v0.1.13" \
  TRUSS_CORE_SOURCE_BASE_URL="file://$EV/raw/truss-v0.1.13" \
  bash "$EV/raw/installer-remote.sh" --directory "$EV/w3" --with-delivery --yes > "$EV/row1-remote.txt" 2>&1
st=$?
check "install from the tag-pinned raw source exits 0" "$st"
check "recorded ref is the resolved release tag" \
  "$([ "$(python3 -c "import json;print(json.load(open('$EV/w3/.truss/core/addons.json'))['addons'][0]['source_ref'])")" = "truss-v0.1.13" ]; echo $?)"
check "the payload was staged outside the source and installed through the CLI" \
  "$(grep -q 'addon install --name delivery --manifest .*/payload' "$EV/row1-remote.txt"; echo $?)"
echo

# -------------------------------------------------------------- refusals ----
echo "== Refusals: stop with a clear message, copy nothing =="
printf '\n<!-- local dev edit -->\n' >> "$PAYLOAD_SOURCE_PROBE"
PAYLOAD_TOUCHED=1
scripts/install-truss.sh --directory "$EV/w4" --with-delivery --yes > "$EV/refusal-dirty.txt" 2>&1
st=$?
cp "$EV/trusses.md.bak" "$PAYLOAD_SOURCE_PROBE"
PAYLOAD_TOUCHED=0
check "an uncommitted local payload refuses" "$([ "$st" -ne 0 ]; echo $?)"
check "the dirty refusal left the target empty" "$([ "$(find "$EV/w4" -mindepth 1 2>/dev/null | wc -l)" = 0 ]; echo $?)"

TRUSS_CORE_BINARY="$CLI" TRUSS_SOURCE_BASE_URL="file://$EV/raw/main" TRUSS_CORE_SOURCE_BASE_URL="file://$EV/raw/main" \
  bash "$EV/raw/installer-remote.sh" --directory "$EV/w5" --with-delivery --yes > "$EV/refusal-floating.txt" 2>&1
st=$?
check "a floating raw source base URL refuses" "$([ "$st" -ne 0 ]; echo $?)"
check "the floating-URL refusal left the target empty" "$([ "$(find "$EV/w5" -mindepth 1 2>/dev/null | wc -l)" = 0 ]; echo $?)"

TRUSS_CORE_BINARY="$CLI" TRUSS_SOURCE_BASE_URL="file://$EV/rawm/truss-v0.1.13" TRUSS_CORE_SOURCE_BASE_URL="file://$EV/rawm/truss-v0.1.13" \
  bash "$EV/raw/installer-remote.sh" --directory "$EV/w6" --with-delivery --yes > "$EV/refusal-tagmismatch.txt" 2>&1
st=$?
check "a tag file that disagrees with the pinned URL refuses" "$([ "$st" -ne 0 ]; echo $?)"
check "the tag-mismatch refusal left the target empty" "$([ "$(find "$EV/w6" -mindepth 1 2>/dev/null | wc -l)" = 0 ]; echo $?)"

bash "$EV/nogit/scripts/install-truss.sh" --directory "$EV/w10" --with-delivery --yes > "$EV/refusal-nogit.txt" 2>&1
st=$?
check "a local source that is not a git checkout refuses" "$([ "$st" -ne 0 ]; echo $?)"
check "the non-git refusal left the target empty" "$([ "$(find "$EV/w10" -mindepth 1 2>/dev/null | wc -l)" = 0 ]; echo $?)"
echo

# ------------------------------------------------------------- override -----
echo "== Override: the replaced root is moved into the backup at its original path =="
# The installed root is two path segments. The override backup must create the
# destination's own parent, not only the backup directory: with a one-level mkdir
# the move has no `.truss/` to land in, and the installer exits 1 after having
# already removed AGENTS.md. The PowerShell route is asserted statically only,
# because this host has no `pwsh`.
WOV="$EV/wov"
OV_LABEL=".truss/core"
mkdir -p "$WOV/$OV_LABEL/docs"
printf 'consumer sentinel\n' > "$WOV/$OV_LABEL/MARKER"
printf 'consumer bytes\n' > "$WOV/$OV_LABEL/docs/WORKFLOW.md"
TRUSS_CORE_BINARY="$CLI" scripts/install-truss.sh --directory "$WOV" --override --yes \
  > "$EV/override-backup.txt" 2>&1
st=$?
check "override against a two-segment installed root exits 0" "$st"
OV_BACKUP="$(find "$WOV/.truss-backup" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | head -n 1)"
check "the override wrote a timestamped backup directory" \
  "$([ -n "$OV_BACKUP" ] && [ -d "$OV_BACKUP" ]; echo $?)"
check "the replaced tree is in the backup at its original relative path" \
  "$([ -f "$OV_BACKUP/$OV_LABEL/MARKER" ] && [ "$(cat "$OV_BACKUP/$OV_LABEL/docs/WORKFLOW.md" 2>/dev/null)" = "consumer bytes" ]; echo $?)"
check "the fresh core payload is installed at the same path" \
  "$([ -f "$WOV/$OV_LABEL/manifest.json" ] && [ ! -e "$WOV/$OV_LABEL/MARKER" ]; echo $?)"
check "the installer reports the backup location of the removed root" \
  "$(grep -q "^removed  $OV_LABEL (backup: .truss-backup/" "$EV/override-backup.txt"; echo $?)"
OV_MOVES="$(grep -cF 'Move-Item -LiteralPath $path -Destination $destination' scripts/install-truss.ps1)"
OV_PARENTS="$(grep -cF 'Split-Path -Parent $destination' scripts/install-truss.ps1)"
check "the PowerShell override creates its destination parent before every move (static)" \
  "$([ "$OV_MOVES" = 2 ] && [ "$OV_PARENTS" = 2 ]; echo $?)"
echo

# ------------------------------------------------------- counterexamples -----
echo "== Row 2 counterexamples: the pre-change installer on the identical state =="
git show d9357b9:scripts/install-truss.sh > "$EV/old-install.sh"
echo "baseline installer: git show d9357b9:scripts/install-truss.sh -> $(file_hash "$EV/old-install.sh")"
TRUSS_CORE_BINARY="$CLI" scripts/install-truss.sh --directory "$EV/w2c" --with-delivery --yes \
  --source-git "file://$EV/gitA" > "$EV/w2c-install.txt" 2>&1
printf '\n<!-- consumer local edit -->\n' >> "$EV/w2c/$PAYLOAD_PROBE"
CONSUMER_C="$(file_hash "$EV/w2c/$PAYLOAD_PROBE")"
cp -a "$EV/w2c" "$EV/w2c-merge"
cp -a "$EV/w2c" "$EV/w2c-force"

# The pre-change installer is driven against a legacy-layout source, the shape it
# was written for, and against the pre-change CLI from the same baseline commit:
# it hardcodes `.truss-core/bin/truss` for the binary it stages and delegates only
# its core step to that CLI, so the faithful "pre-change installer on the identical
# state" pairing is the one below. Its merge and force semantics are then the
# defects this lane exists to catch, asserted on the real run rather than replaced
# by a statement about the pairing.
check "the legacy-layout source carries the payload the old installer expects" \
  "$([ -f "$EV/rawlegacy/truss-v0.1.13/$PAYLOAD_PROBE" ] && [ ! -d "$EV/rawlegacy/truss-v0.1.13/distribution" ]; echo $?)"

TRUSS_CORE_BINARY="$OLD_CLI" bash "$EV/old-install.sh" --directory "$EV/w2c-merge" \
  --with-delivery --merge --yes --source-git "file://$EV/legacyB" > "$EV/counterexample-merge.txt" 2>&1
st=$?
check "old installer --merge exits 0 while skipping every add-on path" \
  "$([ "$st" = 0 ] && [ "$(grep -c 'merge keeps existing file' "$EV/counterexample-merge.txt")" = "$DELIVERY_PATHS" ]; echo $?)"
check "old installer --merge leaves the upstream change unapplied (stale content)" \
  "$([ "$(grep -c 'upstream change in the next release' "$EV/w2c-merge/$PAYLOAD_PROBE")" = 0 ]; echo $?)"
check "old installer --merge records no provenance for the new ref" \
  "$([ ! -f "$EV/w2c-merge/.truss-core/addons.json" ] || [ "$(python3 -c "import json;print(json.load(open('$EV/w2c-merge/.truss-core/addons.json'))['addons'][0]['source_ref'])" 2>/dev/null)" != "$(git -C "$EV/legacyB" rev-parse HEAD)" ]; echo $?)"

TRUSS_CORE_BINARY="$OLD_CLI" bash "$EV/old-install.sh" --directory "$EV/w2c-force" \
  --with-delivery --merge --force --yes --source-git "file://$EV/legacyB" > "$EV/counterexample-force.txt" 2>&1
st=$?
check "old installer --force clobbers the consumer edit" \
  "$([ "$st" = 0 ] && [ "$(file_hash "$EV/w2c-force/$PAYLOAD_PROBE")" != "$CONSUMER_C" ]; echo $?)"
# The consumer's bytes must still exist, and exist as bytes rather than as a file
# whose name merely matches. The old installer backs a forced overwrite up under
# `.truss-backup/<timestamp>/<relative path>`, one timestamped directory per run,
# and the relative path is the destination string (copy_file in
# d9357b9:scripts/install-truss.sh:147, BACKUP_DIR at :867), so the only entry that
# counts is `.truss-backup/<timestamp>/$PAYLOAD_PROBE` with exactly one directory
# segment in place of `<timestamp>`. Selecting by basename would accept an
# unrelated entry that happens to share the basename (the CLI's own backup tree
# under `.truss-backup/<session-id>/state/base/...` carries the same basename), so
# the selector matches that shape, and the middle segment is asserted to be a
# single non-empty segment.
BACKUP_ROOT="$EV/w2c-force/.truss-backup"
BACKUP_PROBE_COPIES=""
while IFS= read -r candidate; do
  [ -n "$candidate" ] || continue
  middle="${candidate#"$BACKUP_ROOT"/}"
  middle="${middle%"/$PAYLOAD_PROBE"}"
  case "$middle" in
    ""|*/*) continue ;;
  esac
  BACKUP_PROBE_COPIES="${BACKUP_PROBE_COPIES}${BACKUP_PROBE_COPIES:+
}${candidate}"
done <<EOF
$(find "$BACKUP_ROOT" -mindepth 2 -type f -path "$BACKUP_ROOT/*/$PAYLOAD_PROBE" 2>/dev/null || true)
EOF
BACKUP_PROBE_TOTAL="$(printf '%s\n' "$BACKUP_PROBE_COPIES" | grep -c . || true)"
BACKUP_PROBE_MATCHING=0
while IFS= read -r candidate; do
  [ -n "$candidate" ] || continue
  [ "$(file_hash "$candidate")" = "$CONSUMER_C" ] && BACKUP_PROBE_MATCHING=$((BACKUP_PROBE_MATCHING + 1))
done <<EOF
$BACKUP_PROBE_COPIES
EOF
check "old installer --force keeps the consumer bytes only in its own backup" \
  "$([ "$BACKUP_PROBE_TOTAL" = 1 ] && [ "$BACKUP_PROBE_MATCHING" = 1 ] && [ "$(file_hash "$EV/w2c-force/$PAYLOAD_PROBE")" != "$CONSUMER_C" ]; echo $?)"
# The pre-change installer has no provenance format at all: it copies add-on bytes
# directly and writes no record. That is asserted on the artefact itself.
check "the pre-change installer carries no add-on record or digest code" \
  "$([ "$(grep -c 'addons.json\|upstream_sha256\|source_ref' "$EV/old-install.sh")" = 0 ]; echo $?)"

# Boundary the rename imposes, asserted rather than assumed: the pre-change
# installer paired with the post-0008 CLI splits one run across two trees. The CLI
# writes `.truss/core`, the installer stages its binary at `.truss-core`, and no
# add-on record is written, because the add-on steps the installer would have run
# never record provenance. This is why the defect lanes above use the pre-change
# CLI, and it is a separate claim from theirs.
TRUSS_CORE_BINARY="$CLI" bash "$EV/old-install.sh" --directory "$EV/w2c-split" \
  --with-delivery --yes --source-git "file://$EV/legacyA" > "$EV/counterexample-split.txt" 2>&1
check "the pre-change installer paired with the post-0008 CLI leaves two trees" \
  "$([ -d "$EV/w2c-split/.truss/core" ] && [ -d "$EV/w2c-split/.truss-core" ]; echo $?)"
check "that split pairing records no add-on provenance at all" \
  "$([ "$(find "$EV/w2c-split" -maxdepth 3 -name addons.json 2>/dev/null | wc -l)" = 0 ]; echo $?)"

TRUSS_CORE_BINARY="$CLI" scripts/install-truss.sh --directory "$EV/w2c" --with-delivery --merge --yes \
  --source-git "file://$EV/gitB" > "$EV/candidate-on-same-state.txt" 2>&1
st=$?
check "candidate on the same state stages a conflict and keeps the consumer bytes" \
  "$([ "$st" -ne 0 ] && [ "$(file_hash "$EV/w2c/$PAYLOAD_PROBE")" = "$CONSUMER_C" ] && [ -f "$EV/w2c/.truss/core/addon-update/delivery/resolved/$PAYLOAD_PROBE" ]; echo $?)"
echo

# ---------------------------------------------------------------- Row 3 -----
echo "== Row 3: PowerShell parity (static; pwsh unavailable on this host) =="
if command -v pwsh >/dev/null 2>&1; then
  echo "pwsh present: $(command -v pwsh)"
else
  echo "pwsh NOT AVAILABLE: the two rehearsals were not executed on PowerShell; the execution gap is declared and no byte-level parity is claimed"
fi
parity="$(python3 - scripts/install-truss.sh scripts/install-truss.ps1 <<'PY'
import re
import sys

bash = open(sys.argv[1]).read()
ps1 = open(sys.argv[2]).read()
anchor = bash.index('addon "$operation"')
bash_args = bash[anchor:bash.index('\n  )', anchor)]
ps1_args = [line for line in ps1.splitlines() if '"addon", $operation' in line][0]

def flags(text):
    return re.findall(r'(?<![\w-])--[a-z-]+', text)

bash_flags, ps1_flags = flags(bash_args), flags(ps1_args)
print('bash add-on flags    :', ' '.join(bash_flags))
print('powershell flags     :', ' '.join(ps1_flags))
print('flag lists identical :', bash_flags == ps1_flags)
print('bash operation from record:', 'operation="install"' in bash and 'operation="update"' in bash)
print('powershell operation from record:', '$operation = "install"' in ps1 and '$operation = "update"' in ps1)
print('bash --dry-run appended when dry:', 'args+=(--dry-run)' in bash)
print('powershell --dry-run appended when dry:', '$arguments += "--dry-run"' in ps1)
PY
)"
printf '%s\n' "$parity"
check "the PowerShell delegation uses the same flags as bash" \
  "$(printf '%s\n' "$parity" | grep -q 'flag lists identical : True'; echo $?)"
check "no add-on path reaches Copy-TrussFile" \
  "$([ "$(grep -c 'Copy-TrussFile\|Write-SourceFile' scripts/install-truss.ps1)" = 0 ]; echo $?)"
check "the install/update choice from the record exists on both platforms" \
  "$(printf '%s\n' "$parity" | grep -q 'powershell operation from record: True'; echo $?)"
check "the PowerShell preflight runs before the core install" \
  "$(python3 -c "
import sys
lines = [line.strip() for line in open(sys.argv[1])]
pre = max(i for i, line in enumerate(lines) if line == 'Invoke-AddOnPreflight')
core = max(i for i, line in enumerate(lines) if line == 'Install-TrussCore')
print(0 if pre < core else 1)" scripts/install-truss.ps1)"
echo

echo "== rehearsal summary: $OK ok, $BAD failed =="
[ "$BAD" = 0 ]
