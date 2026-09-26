#!/usr/bin/env bash
# Legacy-install migration rehearsal — the constructing instrument.
#
#   bash tests/legacy-migration-rehearsal.sh --tag truss-v0.1.16
#
# This is the runnable instrument for an approved acceptance row whose
# provenance is a *released* pre-0008 installation. It does not name that
# artifact and it does not accept a fixture that merely resembles it: it
# installs the exact release tag through that tag's own installer, and then
# proves the current build migrates it.
#
# Why this exists: in the 2026-09-26 migration delivery, four acceptance
# sessions satisfied a row whose approved instrument was "a complete legacy
# installation built from the truss-v0.1.16 ref" while the executed fixture was
# the *current* install renamed to `.truss-core` with `core_version` stamped to
# 0.1.16. Bytes could not tell the difference. This script makes the
# substitution impossible: it fetches the tag, and it fails if the legacy
# artifact's reported version equals the current build's, so a relabelled
# current tree can never satisfy it.
#
# Requirements: bash, git, curl, python3, cargo (--locked), and network access
# to the raw source of the release tag and to its release assets. It is
# therefore deliberately NOT part of scripts/validate-premerge.sh, which stays
# offline; invoke it by name when an approved row requires this provenance.
#
# Options:
#   --tag <truss-vX.Y.Z>   required; an immutable release tag, never a branch
#   --directory <path>     reuse an existing target instead of a temp dir
#   --keep                 keep the fixture directory and print its path
#   --evidence-dir <path>  copy raw outputs here (default target/legacy-migration-evidence)
#
# Environment: TRUSS_RELEASE_REPO overrides the release repository derived from
# the `origin` remote. Exit status is non-zero if any check fails.

set -u

OK=0
BAD=0
WORK=""
KEEP=0
TAG=""
TARGET=""
EVIDENCE="${LEGACY_REHEARSAL_EVIDENCE_DIR:-target/legacy-migration-evidence}"

check() {
    local description="$1"
    local status="$2"
    if [ "$status" = "0" ]; then
        OK=$((OK + 1))
        printf 'ok   %s\n' "$description"
    else
        BAD=$((BAD + 1))
        printf 'FAIL %s\n' "$description"
    fi
}

say() { printf '%s\n' "$*"; }

cleanup() {
    if [ -n "$WORK" ] && [ "$KEEP" = "0" ]; then
        rm -rf "$WORK"
    fi
}
trap cleanup EXIT

while [ $# -gt 0 ]; do
    case "$1" in
        --tag) TAG="${2:-}"; shift 2 ;;
        --directory) TARGET="${2:-}"; shift 2 ;;
        --keep) KEEP=1; shift ;;
        --evidence-dir) EVIDENCE="${2:-}"; shift 2 ;;
        *) say "unknown argument: $1"; exit 2 ;;
    esac
done

if [ -z "$TAG" ]; then
    say "usage: bash tests/legacy-migration-rehearsal.sh --tag truss-vX.Y.Z [--keep]"
    exit 2
fi

# The approved provenance is an immutable release tag. A branch, a short SHA, or
# a revision expression is not that artifact, so refuse before any network use.
case "$TAG" in
    truss-v[0-9]*.[0-9]*.[0-9]*) ;;
    *) say "refused: --tag must be an immutable release tag such as truss-v0.1.16, got '$TAG'"; exit 2 ;;
esac

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 2

if [ -z "${TRUSS_RELEASE_REPO:-}" ]; then
    origin="$(git remote get-url origin 2>/dev/null || true)"
    case "$origin" in
        *github.com[:/]*/*) TRUSS_RELEASE_REPO="$(printf '%s' "$origin" | sed -E 's#.*github\.com[:/]([^/]+/[^/.]+)(\.git)?$#\1#')" ;;
    esac
fi
if [ -z "${TRUSS_RELEASE_REPO:-}" ]; then
    say "refused: set TRUSS_RELEASE_REPO=owner/name (could not derive it from the origin remote)"
    exit 2
fi
export TRUSS_RELEASE_REPO

BASE="https://raw.githubusercontent.com/${TRUSS_RELEASE_REPO}/${TAG}"
export TRUSS_SOURCE_BASE_URL="$BASE"
export TRUSS_CORE_SOURCE_BASE_URL="$BASE"

if [ -z "$TARGET" ]; then
    WORK="$(mktemp -d "${TMPDIR:-/tmp}/legacy-migration-rehearsal.XXXXXX")"
    TARGET="$WORK/target"
fi
mkdir -p "$TARGET"

say "legacy migration rehearsal"
say "  release repository : $TRUSS_RELEASE_REPO"
say "  legacy artifact    : $TAG"
say "  target             : $TARGET"
say

# ---------------------------------------------------------------- the artifact
# Fetch the tag's own installer. A tag that predates the single-root layout
# installs `.truss-core`; that is the provenance under test.
installer_log="$EVIDENCE/install-$TAG.log"
mkdir -p "$EVIDENCE"
if curl -fsSL "$BASE/scripts/install-truss.sh" -o "$WORK/install-legacy.sh" 2>"$installer_log"; then
    check "the release tag ships its own installer" 0
else
    check "the release tag ships its own installer" 1
    say "  (no network, or $TAG does not exist; see $installer_log)"
    say
    say "== legacy migration rehearsal summary: $OK ok, $BAD failed =="
    exit 1
fi

bash "$WORK/install-legacy.sh" --directory "$TARGET" --yes \
    --with-delivery --with-planning --with-engineering-wisdom \
    >"$EVIDENCE/install-$TAG.out" 2>&1
check "the tag's installer completes against a fresh target" "$?"

check "the legacy artifact is a pre-0008 installation (.truss-core)" \
    "$([ -d "$TARGET/.truss-core" ] && [ ! -d "$TARGET/.truss" ]; echo $?)"

LEGACY_BIN="$TARGET/.truss-core/bin/truss"
LEGACY_VERSION="$("$LEGACY_BIN" --version 2>/dev/null | awk '{print $NF}')"

# ------------------------------------------------------- the anti-relabel guard
cargo build --release --locked -p truss >"$EVIDENCE/build.out" 2>&1
check "the current build compiles" "$?"
CURRENT_BIN="$ROOT/target/release/truss"
CURRENT_VERSION="$("$CURRENT_BIN" --version 2>/dev/null | awk '{print $NF}')"

check "the legacy artifact reports the legacy version, not the current one" \
    "$([ -n "$LEGACY_VERSION" ] && [ "$LEGACY_VERSION" != "$CURRENT_VERSION" ]; echo $?)"
if [ "$LEGACY_VERSION" = "$CURRENT_VERSION" ]; then
    say "  legacy '$LEGACY_VERSION' equals current '$CURRENT_VERSION': this rehearsal cannot"
    say "  distinguish the artifacts, so it proves nothing and refuses to continue"
    say
    say "== legacy migration rehearsal summary: $OK ok, $BAD failed =="
    exit 1
fi
say "  legacy version $LEGACY_VERSION -> current version $CURRENT_VERSION"
say

# ------------------------------------------------------------------- migration
before="$(cd "$TARGET" && find . -printf '%P %s %m\n' | sort | md5sum)"
"$CURRENT_BIN" migrate --directory "$TARGET" >"$EVIDENCE/preview.out" 2>&1
preview_status="$?"
after="$(cd "$TARGET" && find . -printf '%P %s %m\n' | sort | md5sum)"
check "preview exits 0" "$preview_status"
check "preview mutates nothing" "$([ "$before" = "$after" ]; echo $?)"
check "preview reports ready" \
    "$(grep -q 'migration preview: ready' "$EVIDENCE/preview.out"; echo $?)"
check "preview prints the backup template, not a concrete timestamp" \
    "$(grep -q '<UTC-timestamp>' "$EVIDENCE/preview.out" && ! grep -qE '\.truss-migration-backup/[0-9]{8}T' "$EVIDENCE/preview.out"; echo $?)"

"$CURRENT_BIN" migrate --directory "$TARGET" --apply >"$EVIDENCE/apply.out" 2>&1
apply_status="$?"
check "apply exits 0" "$apply_status"
check "apply reports migrated" "$(grep -q 'migration: migrated' "$EVIDENCE/apply.out"; echo $?)"
check "the legacy root is retired" "$([ ! -d "$TARGET/.truss-core" ]; echo $?)"
check "the new root exists" "$([ -d "$TARGET/.truss/core" ]; echo $?)"

backup="$(grep -oE 'retained at [^ ]+' "$EVIDENCE/apply.out" | awk '{print $3}')"
check "apply prints the retained backup path" "$([ -n "$backup" ] && [ -d "$backup" ]; echo $?)"
check "the backup holds the legacy installation" "$([ -d "$backup/backup/.truss-core" ]; echo $?)"
if [ -n "$backup" ] && [ -x "$backup/backup/.truss-core/bin/truss" ]; then
    restored="$("$backup/backup/.truss-core/bin/truss" --version 2>/dev/null | awk '{print $NF}')"
    check "the retained backup holds the runnable legacy artifact" \
        "$([ "$restored" = "$LEGACY_VERSION" ]; echo $?)"
else
    check "the retained backup holds the runnable legacy artifact" 1
fi

# ------------------------------------------------- the post-migration contract
MIGRATED_BIN="$TARGET/.truss/core/bin/truss"
check "the bundled executable is present and executable" "$([ -x "$MIGRATED_BIN" ]; echo $?)"

migrated_version="$("$MIGRATED_BIN" --version 2>/dev/null | awk '{print $NF}')"
check "the bundled executable is the current build, not the legacy one" \
    "$([ "$migrated_version" = "$CURRENT_VERSION" ]; echo $?)"
if [ "$migrated_version" != "$CURRENT_VERSION" ]; then
    say "  bundled '$migrated_version' stayed at the legacy version: the migrated"
    say "  installation has no working CLI of its own"
fi

"$MIGRATED_BIN" status --directory "$TARGET" >"$EVIDENCE/status.out" 2>&1
check "the bundled executable reads the migrated root (not not_installed)" \
    "$(! grep -q 'not_installed' "$EVIDENCE/status.out"; echo $?)"
check "the migrated installation reports the legacy payload as updatable, not current" \
    "$(grep -q 'update_available' "$EVIDENCE/status.out"; echo $?)"

"$MIGRATED_BIN" doctor --directory "$TARGET" >"$EVIDENCE/doctor.out" 2>&1
check "the migrated installation passes its own doctor" \
    "$(grep -qE 'doctor: [0-9]+ checks?, 0 failed|^pass ' "$EVIDENCE/doctor.out"; echo $?)"

for addon in delivery planning engineering-wisdom; do
    "$MIGRATED_BIN" addon status --name "$addon" --directory "$TARGET" \
        >"$EVIDENCE/addon-$addon.out" 2>&1
    check "the $addon add-on is still recorded after migration" \
        "$(grep -q "Add-on $addon" "$EVIDENCE/addon-$addon.out"; echo $?)"
done

if [ "$KEEP" = "1" ]; then
    say
    say "fixture kept at $TARGET"
fi

say
say "== legacy migration rehearsal summary: $OK ok, $BAD failed =="
[ "$BAD" = 0 ]
