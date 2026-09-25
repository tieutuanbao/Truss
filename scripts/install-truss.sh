#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: install-truss.sh [options] [path]

Bootstrap the Rust `truss` CLI and install the Truss core into a target.

Options:
  -d, --directory <path>  Target directory. Defaults to the current directory.
  -y, --yes              Accept defaults and skip prompts.
      --with-engineering-wisdom
                         Add the explicit-only engineering-wisdom advisory
                         skill. It is excluded from the default core.
      --with-delivery    Add the explicit-only delivery orchestration add-on
                         (Orca execution plane). It is excluded from the
                         default core.
      --with-planning    Add the explicit-only product-planning add-on that
                         turns an idea into user-approved repository
                         authority. It is excluded from the default core.
      --merge            On protected-path conflict, keep existing files in
                         place and install only missing Truss files.
      --refresh-agent-shim
                         Refresh an existing AGENTS.md into the small Truss
                         shim after backing it up. Old Truss-generated files
                         are replaced; custom files receive a marked block.
      --claude           Also install or refresh CLAUDE.md so Claude Code
                         auto-loads the truss context. Claude Code never
                         auto-loads AGENTS.md; the shim @-imports AGENTS.md
                         as its single policy source inside a marked block.
                         Existing CLAUDE.md files get the block appended
                         after a backup; a stale block is refreshed in place.
      --override         On protected-path conflict, back up and replace
                         AGENTS.md and the installed tree's docs/.
      --force            Overwrite existing files after backing them up.
      --dry-run          Show what would change without writing files.
      --source-git URL   Clone the Truss checkout from a git URL (depth 1),
                         then install from that checkout. Installs the latest
                         committed state; binary is built from source, so a
                         current Rust toolchain is required.
  -h, --help             Show this help.

Safety:
  The installer installs the repository-centered core plus the Rust
  maintenance CLI. It performs no compatibility CLI or SQLite/control-plane
  download and no database write. If AGENTS.md or the installed docs/ already exist,
  interactive installs ask
  whether to merge missing files, override after backup, or stop. Merge is the
  safe update path for repositories that already have Truss: existing files
  stay in place and new Truss files are appended by path. Non-
  interactive installs stop unless --merge or --override is provided. If a
  target .gitignore receives only the Rust maintenance binary rules.
  Optional add-ons requested with --with-engineering-wisdom, --with-delivery,
  or --with-planning are acquired here and installed or updated by the Truss
  CLI; see Add-ons below for the lifecycle. --merge and --force do not apply to
  add-on files, and this installer never copies an add-on file directly.

Add-ons:
  Every add-on is one distribution with its own immutable provenance. The
  Truss CLI owns its plan, baseline, and record under .truss/core/:
    truss addon status   --name <name>
    truss addon install  --name <name> --manifest <manifest> --source <dir> --source-ref <ref>
    truss addon update   --name <name> --manifest <manifest> --source <dir> --source-ref <ref>
    truss addon continue --name <name>
    truss addon abort    --name <name>
  --source-ref is an immutable release tag or exact commit SHA, never a branch
  name, and each managed file is recorded with its SHA-256 in
  .truss/core/addons.json. --dry-run previews the plan without writing.
  Overlapping local and upstream edits are never overwritten: update stops and
  stages the conflict under .truss/core/addon-update/<name>/resolved/, and
  continue applies the operator-edited copies while abort removes only the
  session. AGENTS.md is never an add-on payload file. A remote raw source base
  URL (TRUSS_SOURCE_BASE_URL) must be pinned to the release tag or the add-on
  step stops.

Examples:
  scripts/install-truss.sh
  scripts/install-truss.sh --directory /path/to/project --yes
  scripts/install-truss.sh --directory /path/to/project --with-delivery --yes
  scripts/install-truss.sh ./my-project --force

  # Remote bootstrap from a git URL: always installs the latest committed
  # state; the CLI binary is built from the cloned source (needs cargo).
  export TRUSS_SOURCE_GIT=https://github.com/tieutuanbao/Truss.git
  curl -fsSL https://raw.githubusercontent.com/tieutuanbao/Truss/main/scripts/install-truss.sh \
    | bash -s -- --source-git "$TRUSS_SOURCE_GIT" --yes

  # Raw-URL bootstrap (uses published release binaries):
  export TRUSS_SOURCE_BASE_URL=https://raw.githubusercontent.com/tieutuanbao/Truss/main
  export TRUSS_CORE_SOURCE_BASE_URL=$TRUSS_SOURCE_BASE_URL
  curl -fsSL "$TRUSS_SOURCE_BASE_URL/scripts/install-truss.sh" | bash -s -- --yes
  curl -fsSL "$TRUSS_SOURCE_BASE_URL/scripts/install-truss.sh" | bash -s -- --merge --yes
  curl -fsSL "$TRUSS_SOURCE_BASE_URL/scripts/install-truss.sh" | bash -s -- --merge --refresh-agent-shim --yes
  curl -fsSL "$TRUSS_SOURCE_BASE_URL/scripts/install-truss.sh" | bash -s -- --claude --yes
EOF
}

log() {
  printf '%s\n' "$*"
}

fail() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}

warn_stop() {
  printf 'Warning: %s\n' "$*" >&2
  exit 1
}

can_prompt() {
  # ponytail: stat-mode test alone passes with no controlling tty (ENXIO on open);
  # actually open /dev/tty read-write to prove a prompt is possible.
  { : <> /dev/tty; } 2>/dev/null
}

prompt_tty() {
  printf '%s' "$1" > /dev/tty
}

read_tty() {
  local value
  IFS= read -r value < /dev/tty
  printf '%s\n' "$value"
}

expand_path() {
  case "$1" in
    "~")
      printf '%s\n' "$HOME"
      ;;
    "~/"*)
      printf '%s/%s\n' "$HOME" "${1#~/}"
      ;;
    /*)
      printf '%s\n' "$1"
      ;;
    *)
      printf '%s/%s\n' "$PWD" "$1"
      ;;
  esac
}

make_absolute_parent() {
  local path="$1"
  local parent
  parent="$(dirname "$path")"
  [ -d "$parent" ] || fail "Parent directory does not exist: $parent"
  (cd "$parent" && printf '%s/%s\n' "$(pwd -P)" "$(basename "$path")")
}

read_source_text() {
  local relative="$1"

  if [ "$SOURCE_MODE" = "local" ]; then
    local source="$SOURCE_ROOT/$relative"
    [ -f "$source" ] || fail "Source file missing: $source"
    cat "$source"
    return
  fi

  local url="$SOURCE_BASE_URL/$relative"
  curl -fsSL "$url" || fail "Could not download $url"
}

read_payload_manifest() {
  local payload_manifest="$1"
  if [ "$SOURCE_MODE" = "local" ]; then
    local manifest="$SOURCE_ROOT/$payload_manifest"
    [ -f "$manifest" ] || fail "Payload manifest missing: $manifest"
    cat "$manifest"
    return
  fi

  local url="$SOURCE_BASE_URL/$payload_manifest"
  curl -fsSL "$url" || fail "Could not download $url"
}

agent_shim_block() {
  read_source_text "scripts/agent-truss-block.md"
}

claude_shim_block() {
  read_source_text "scripts/claude-truss-block.md"
}

backup_agent_file() {
  local target="$TARGET_DIR/AGENTS.md"

  [ -e "$target" ] || return 0
  mkdir -p "$BACKUP_DIR"
  [ -e "$BACKUP_DIR/AGENTS.md" ] && return 0
  cp -p "$target" "$BACKUP_DIR/AGENTS.md"
}

append_or_replace_agent_truss_block() {
  local target="$TARGET_DIR/AGENTS.md"
  local block_tmp tmp

  block_tmp="$(mktemp)"
  agent_shim_block >"$block_tmp"
  [ -s "$block_tmp" ] || fail "canonical AGENTS.md Truss block is empty"
  tmp="$(mktemp)"
  if grep -Fq "<!-- TRUSS:BEGIN -->" "$target" &&
     grep -Fq "<!-- TRUSS:END -->" "$target"; then
    awk '
      /<!-- TRUSS:BEGIN -->/ {
        while ((getline line < block_file) > 0) {
          print line
        }
        in_block = 1
        next
      }
      /<!-- TRUSS:END -->/ && in_block {
        in_block = 0
        next
      }
      !in_block { print }
    ' block_file="$block_tmp" "$target" > "$tmp"
  else
    {
      cat "$target"
      printf '\n'
      agent_shim_block
    } > "$tmp"
  fi
  mv "$tmp" "$target"
  rm -f "$block_tmp"
}

validate_truss_markers() {
  local target="$1" label="$2"
  local begin_count end_count begin_line end_line
  begin_count=$(grep -Fc '<!-- TRUSS:BEGIN -->' "$target" || true)
  end_count=$(grep -Fc '<!-- TRUSS:END -->' "$target" || true)
  if [ "$begin_count" -eq 0 ] && [ "$end_count" -eq 0 ]; then
    return 0
  fi
  if [ "$begin_count" -ne 1 ] || [ "$end_count" -ne 1 ]; then
    fail "$label must contain exactly one complete Truss marker pair"
  fi
  begin_line=$(grep -Fn '<!-- TRUSS:BEGIN -->' "$target" | cut -d: -f1)
  end_line=$(grep -Fn '<!-- TRUSS:END -->' "$target" | cut -d: -f1)
  [ "$begin_line" -lt "$end_line" ] || fail "$label Truss markers are out of order"
}

refresh_agent_shim() {
  [ "$REFRESH_AGENT_SHIM" -eq 1 ] || return 0

  local target="$TARGET_DIR/AGENTS.md"
  [ -e "$target" ] || return 0

  if [ "$SOURCE_MODE" = "local" ] && [ "$SOURCE_ROOT/AGENTS.md" -ef "$target" ]; then
    log "skip     AGENTS.md (source file)"
    return 0
  fi

  validate_truss_markers "$target" "AGENTS.md"

  if [ "$DRY_RUN" -eq 1 ]; then
    log "refresh  AGENTS.md (append or replace marked Truss block, backup first)"
    UPDATED=$((UPDATED + 1))
    return 0
  fi

  backup_agent_file
  append_or_replace_agent_truss_block
  log "updated  AGENTS.md (refreshed Truss block; backup: ${BACKUP_DIR#$TARGET_DIR/}/AGENTS.md)"
  UPDATED=$((UPDATED + 1))
}

backup_claude_file() {
  local target="$TARGET_DIR/CLAUDE.md"

  [ -e "$target" ] || return 0
  mkdir -p "$BACKUP_DIR"
  [ -e "$BACKUP_DIR/CLAUDE.md" ] && return 0
  cp -p "$target" "$BACKUP_DIR/CLAUDE.md"
}

write_claude_shim() {
  [ "$INSTALL_CLAUDE_SHIM" -eq 1 ] || return 0

  local target="$TARGET_DIR/CLAUDE.md"
  local block_tmp tmp

  if [ "$SOURCE_MODE" = "local" ] && [ -e "$target" ] &&
     [ "$SOURCE_ROOT/CLAUDE.md" -ef "$target" ]; then
    log "skip     CLAUDE.md (source file)"
    SKIPPED=$((SKIPPED + 1))
    return 0
  fi

  if [ -e "$target" ]; then
    validate_truss_markers "$target" "CLAUDE.md"
  fi

  block_tmp="$(mktemp)"
  claude_shim_block > "$block_tmp"

  if [ -e "$target" ] &&
     grep -Fq "<!-- TRUSS:BEGIN -->" "$target" &&
     grep -Fq "<!-- TRUSS:END -->" "$target"; then
    local current_tmp
    current_tmp="$(mktemp)"
    awk '
      /<!-- TRUSS:BEGIN -->/ { in_block = 1 }
      in_block { print }
      /<!-- TRUSS:END -->/ { in_block = 0 }
    ' "$target" > "$current_tmp"
    if cmp -s "$current_tmp" "$block_tmp"; then
      log "skip     CLAUDE.md (Truss block current)"
      SKIPPED=$((SKIPPED + 1))
      rm -f "$current_tmp" "$block_tmp"
      return 0
    fi
    rm -f "$current_tmp"

    if [ "$DRY_RUN" -eq 1 ]; then
      log "update   CLAUDE.md (refresh marked Truss block, backup first)"
    else
      backup_claude_file
      tmp="$(mktemp)"
      awk '
        /<!-- TRUSS:BEGIN -->/ {
          while ((getline line < block_file) > 0) {
            print line
          }
          in_block = 1
          next
        }
        /<!-- TRUSS:END -->/ && in_block {
          in_block = 0
          next
        }
        !in_block { print }
      ' block_file="$block_tmp" "$target" > "$tmp"
      mv "$tmp" "$target"
      log "updated  CLAUDE.md (refreshed Truss block; backup: ${BACKUP_DIR#$TARGET_DIR/}/CLAUDE.md)"
    fi
    UPDATED=$((UPDATED + 1))
  elif [ -e "$target" ]; then
    if [ "$DRY_RUN" -eq 1 ]; then
      log "update   CLAUDE.md (append Truss block, backup first)"
    else
      backup_claude_file
      {
        printf '\n'
        cat "$block_tmp"
      } >> "$target"
      log "updated  CLAUDE.md (appended Truss block; backup: ${BACKUP_DIR#$TARGET_DIR/}/CLAUDE.md)"
    fi
    UPDATED=$((UPDATED + 1))
  else
    if [ "$DRY_RUN" -eq 1 ]; then
      log "create   CLAUDE.md"
    else
      {
        printf '# Project Rules\n\n'
        cat "$block_tmp"
      } > "$target"
      log "created  CLAUDE.md"
    fi
    CREATED=$((CREATED + 1))
  fi
  rm -f "$block_tmp"
}

detect_cli_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os:$arch" in
    Darwin:arm64)  printf 'macos-arm64' ;;
    Darwin:x86_64) printf 'macos-x64' ;;
    Linux:x86_64)  printf 'linux-x64' ;;
    Linux:aarch64|Linux:arm64) printf 'linux-arm64' ;;
    *)
      fail "Unsupported Truss CLI platform: $os/$arch."
      ;;
  esac
}

sha256_file() {
  local file="$1"
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$file" | awk '{ print $1 }'
  elif command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$file" | awk '{ print $1 }'
  else
    fail "shasum or sha256sum is required to verify the Truss core download"
  fi
}

download_file() {
  local url="$1"
  local target="$2"
  curl -fsSL "$url" -o "$target" || fail "Could not download $url"
}

read_truss_release_tag() {
  local tag_file="scripts/truss-release-tag"
  local tag=""
  if [ -n "${TRUSS_CORE_RELEASE_TAG:-}" ]; then
    printf '%s\n' "$TRUSS_CORE_RELEASE_TAG"
    return
  fi
  if [ "$SOURCE_MODE" = "local" ]; then
    [ -f "$SOURCE_ROOT/$tag_file" ] &&
      tag="$(awk 'NF && $1 !~ /^#/ { print $1; exit }' "$SOURCE_ROOT/$tag_file")"
  else
    local tag_tmp
    tag_tmp="$(mktemp)"
    if curl -fsSL "$CORE_SOURCE_BASE_URL/$tag_file" -o "$tag_tmp" 2>/dev/null; then
      tag="$(awk 'NF && $1 !~ /^#/ { print $1; exit }' "$tag_tmp")"
    fi
    rm -f "$tag_tmp"
  fi
  [ -n "$tag" ] || fail "Truss core release tag is missing"
  printf '%s\n' "$tag"
}

merge_core_gitignore() {
  local target="$1"
  local marker="# Truss core maintenance binary"
  local unix_rule="$TARGET_STATE_LABEL/bin/truss"
  local windows_rule="$TARGET_STATE_LABEL/bin/truss.exe"
  # One missing/skip rule shared with the PowerShell twin: three required lines,
  # skip only when all three are present, append only the missing ones. Keying the
  # skip on the two rules alone duplicated the marker on every run that found a
  # rule missing beside an existing marker.
  local rules=("$marker" "$unix_rule" "$windows_rule")
  local missing_rules=()
  local rule
  for rule in "${rules[@]}"; do
    [ -f "$target" ] && grep -Fxq "$rule" "$target" || missing_rules+=("$rule")
  done
  if [ "${#missing_rules[@]}" -eq 0 ]; then
    log "skip     .gitignore ($TARGET_STATE_LABEL binary rules already present)"
    return
  fi
  if [ "$DRY_RUN" -eq 1 ]; then
    log "update   .gitignore (append Truss core binary rules)"
    return
  fi
  {
    [ -s "$target" ] && printf '\n'
    printf '%s\n' "${missing_rules[@]}"
  } >> "$target"
  log "updated  .gitignore (appended Truss core binary rules)"
}

stage_truss_core_cli() {
  CORE_STAGE_ROOT="$(mktemp -d)"
  CORE_STAGED_BINARY="$CORE_STAGE_ROOT/truss"
  CORE_PLATFORM="${TRUSS_CORE_CLI_PLATFORM:-$(detect_cli_platform)}"
  CORE_BINARY_NAME="truss-$CORE_PLATFORM"
  if [ -n "${TRUSS_CORE_BINARY:-}" ]; then
    [ -x "$TRUSS_CORE_BINARY" ] || fail "TRUSS_CORE_BINARY is not executable: $TRUSS_CORE_BINARY"
    cp "$TRUSS_CORE_BINARY" "$CORE_STAGED_BINARY"
  elif [ "$SOURCE_MODE" = "local" ]; then
    command -v cargo >/dev/null 2>&1 || fail "cargo is required for a local Truss source install"
    cargo build --quiet --manifest-path "$SOURCE_ROOT/Cargo.toml" -p truss --locked
    local cargo_target_root
    if [ -z "${CARGO_TARGET_DIR:-}" ]; then
      cargo_target_root="$SOURCE_ROOT/target"
    elif [ "${CARGO_TARGET_DIR#/}" != "$CARGO_TARGET_DIR" ]; then
      cargo_target_root="$CARGO_TARGET_DIR"
    else
      cargo_target_root="$(pwd -P)/$CARGO_TARGET_DIR"
    fi
    cp "$cargo_target_root/debug/truss" "$CORE_STAGED_BINARY"
  else
    local release_tag base_url binary_url checksum_url checksum_tmp expected actual
    if [ -n "${CORE_PENDING_VERSION:-}" ]; then
      release_tag="truss-v$CORE_PENDING_VERSION"
    else
      release_tag="$(read_truss_release_tag)"
    fi
    [[ "$release_tag" =~ ^truss-v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9]+)*$ ]] ||
      fail "invalid Truss core release tag: $release_tag"
    if [ -n "${TRUSS_CORE_CLI_BASE_URL:-}" ]; then
      base_url="$TRUSS_CORE_CLI_BASE_URL"
    else
      [ -n "${TRUSS_RELEASE_REPO:-}" ] || fail "set TRUSS_RELEASE_REPO (owner/name) or TRUSS_CORE_CLI_BASE_URL for a remote core download"
      base_url="https://github.com/${TRUSS_RELEASE_REPO}/releases/download/$release_tag"
    fi
    binary_url="${base_url%/}/$CORE_BINARY_NAME"
    checksum_url="$binary_url.sha256"
    checksum_tmp="$CORE_STAGE_ROOT/$CORE_BINARY_NAME.sha256"
    download_file "$binary_url" "$CORE_STAGED_BINARY"
    download_file "$checksum_url" "$checksum_tmp"
    expected="$(awk '{ print $1; exit }' "$checksum_tmp")"
    actual="$(sha256_file "$CORE_STAGED_BINARY")"
    [ -n "$expected" ] && [ "$expected" = "$actual" ] ||
      fail "Checksum mismatch for $CORE_BINARY_NAME: expected $expected, got $actual"
    local reported_version
    chmod 755 "$CORE_STAGED_BINARY"
    reported_version="$("$CORE_STAGED_BINARY" --version | awk '{ print $NF; exit }')"
    [ "$reported_version" = "${release_tag#truss-v}" ] ||
      fail "Truss core release identity mismatch: tag=${release_tag#truss-v}, binary=$reported_version"
  fi
  chmod 755 "$CORE_STAGED_BINARY"
}

install_truss_core() {
  local command="install"
  [ -f "$TARGET_STATE_DIR/manifest.json" ] && command="update"
  CORE_PENDING_VERSION=""
  if [ "$command" = "update" ] && [ -f "$TARGET_STATE_DIR/update/session.json" ]; then
    CORE_PENDING_VERSION="$(sed -n 's/.*"to_version":[[:space:]]*"\([^"]*\)".*/\1/p' "$TARGET_STATE_DIR/update/session.json" | head -n 1)"
    [ -n "$CORE_PENDING_VERSION" ] || fail "could not read pending Truss update version"
  fi
  stage_truss_core_cli
  local args=("$command" --directory "$TARGET_DIR")
  [ "$command" = "update" ] && args+=(--candidate)
  [ -n "$CORE_PENDING_VERSION" ] && args+=(--continue)
  [ "$DRY_RUN" -eq 1 ] && args+=(--dry-run)
  local runner="$CORE_STAGED_BINARY"
  local binary_target="" binary_temp=""
  if [ "$DRY_RUN" -eq 0 ]; then
    binary_target="$TARGET_STATE_DIR/bin/truss"
    binary_temp="$TARGET_STATE_DIR/bin/.truss.$$.tmp"
    [ ! -L "$TARGET_STATE_DIR" ] || fail "refusing symlink for $TARGET_STATE_LABEL"
    [ ! -L "$TARGET_STATE_DIR/bin" ] || fail "refusing symlink for $TARGET_STATE_LABEL/bin directory"
    [ ! -L "$binary_target" ] || fail "refusing symlink for repository Truss executable"
    mkdir -p "$(dirname "$binary_target")"
    cp "$CORE_STAGED_BINARY" "$binary_temp"
    chmod 755 "$binary_temp"
    if [ -e "$binary_target" ]; then
      mkdir -p "$BACKUP_DIR/$TARGET_STATE_LABEL/bin"
      cp -p "$binary_target" "$BACKUP_DIR/$TARGET_STATE_LABEL/bin/truss"
    fi
  fi
  set +e
  "$runner" "${args[@]}"
  local command_status=$?
  set -e
  if [ "$command_status" -eq 2 ] && [ "$DRY_RUN" -eq 0 ]; then
    local retained="$TARGET_STATE_DIR/update-candidate/truss"
    [ ! -L "$TARGET_STATE_DIR" ] || fail "refusing symlink for $TARGET_STATE_LABEL"
    [ ! -L "$TARGET_STATE_DIR/update-candidate" ] || fail "refusing symlink for retained candidate directory"
    [ ! -L "$retained" ] || fail "refusing symlink for retained update candidate"
    mkdir -p "$(dirname "$retained")"
    cp "$CORE_STAGED_BINARY" "$retained"
    chmod 755 "$retained"
  fi
  if [ "$command_status" -eq 0 ] && [ "$DRY_RUN" -eq 0 ]; then
    mv -f "$binary_temp" "$binary_target"
    rm -rf "$TARGET_STATE_DIR/update-candidate"
    merge_core_gitignore "$TARGET_DIR/.gitignore"
    log "installed $TARGET_STATE_LABEL/bin/truss ($CORE_PLATFORM)"
  elif [ -n "$binary_temp" ]; then
    rm -f "$binary_temp"
  fi
  rm -rf "$CORE_STAGE_ROOT"
  CORE_STAGE_ROOT=""
  if [ "$command_status" -eq 2 ]; then
    fail "Truss core update needs resolution; edit $TARGET_STATE_LABEL/update/resolved/, then rerun this installer or truss update --continue"
  fi
  [ "$command_status" -eq 0 ] || fail "truss $command failed with exit code $command_status"
}

check_protected_target_paths() {
  local conflicts=()

  [ -e "$TARGET_DIR/AGENTS.md" ] && conflicts+=("AGENTS.md")
  [ -e "$TARGET_STATE_DIR" ] && conflicts+=("$TARGET_STATE_LABEL/")
  [ "${#conflicts[@]}" -gt 0 ] || return 0

  local joined=""
  local item
  for item in "${conflicts[@]}"; do
    if [ -n "$joined" ]; then
      joined="$joined, $item"
    else
      joined="$item"
    fi
  done

  case "$REQUESTED_CONFLICT_ACTION" in
    merge)
      CONFLICT_ACTION="merge"
      log "Continuing with merge. Existing files will be skipped."
      return 0
      ;;
    override)
      CONFLICT_ACTION="override"
      override_protected_target_paths
      return 0
      ;;
    stop)
      warn_stop "target already contains protected Truss paths: $joined. Refusing to install so existing project instructions or docs are not mixed or overwritten."
      ;;
  esac

  if [ "$YES" -eq 1 ] || ! can_prompt; then
    warn_stop "target already contains protected Truss paths: $joined. Refusing to install so existing project instructions or docs are not mixed or overwritten. Use an empty target directory, or move those paths before running the installer."
  fi

  {
    printf 'Warning: target already contains protected Truss paths: %s\n' "$joined"
    printf 'Choose how to continue:\n'
    printf '  1. Merge    Copy missing Truss files and skip existing files\n'
    printf '  2. Override Back up and replace AGENTS.md and %s/docs/\n' "$TARGET_STATE_LABEL"
    printf '  3. Stop     Exit without writing files (recommended)\n'
  } > /dev/tty
  prompt_tty 'Choice [1/2/3, default 3]: '

  local choice
  choice="$(read_tty)"
  case "$choice" in
    1|m|M|merge|Merge)
      CONFLICT_ACTION="merge"
      log "Continuing with merge. Existing files will be skipped."
      ;;
    2|o|O|override|Override)
      CONFLICT_ACTION="override"
      override_protected_target_paths
      ;;
    ""|3|s|S|stop|Stop)
      warn_stop "installation stopped by user."
      ;;
    *)
      warn_stop "unknown choice: $choice"
      ;;
  esac
}

override_protected_target_paths() {
  local protected

  for protected in AGENTS.md "$TARGET_STATE_LABEL"; do
    [ -e "$TARGET_DIR/$protected" ] || continue

    if [ "$DRY_RUN" -eq 1 ]; then
      log "override $protected (backup first)"
      continue
    fi

    # The installed root is `.truss/core`, two path segments. Create the
    # destination's own parent and not only the backup directory, or the move
    # has no `.truss/` to land in.
    local backup_path="$BACKUP_DIR/$protected"
    mkdir -p "$(dirname "$backup_path")"
    mv "$TARGET_DIR/$protected" "$backup_path"
    log "removed  $protected (backup: ${BACKUP_DIR#$TARGET_DIR/}/$protected)"
  done

}

# ---------------------------------------------------------------------------
# Add-on installation: delegated to the Truss CLI, never copied.
#
# The membership manifests stay the owner of what an add-on contains: their
# non-comment lines are exactly the payload path set staged for the CLI. The
# CLI is invoked once per requested add-on and owns planning, preservation,
# update, adoption, and conflict staging against the recorded baseline, so
# every add-on install writes `<state>/addons.json` and the
# `<state>/base-addons/<name>/` copies of the payload bytes, where `<state>` is
# the target's resolved root. No add-on path
# is written by a direct copy path in this installer.
# ---------------------------------------------------------------------------

ADDON_SOURCE_REF=""
ADDON_SOURCE_CORE_VERSION=""
ADDON_STAGED_PAYLOAD=""
ADDON_STAGED_MANIFEST=""

# An immutable ref: exactly a released tag or a full commit SHA. A branch name,
# `HEAD`, or a short SHA can move, so none of them is accepted.
is_immutable_source_ref() {
  [[ "$1" =~ ^truss-v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9]+)*$ ]] ||
    [[ "$1" =~ ^([0-9a-f]{40}|[0-9a-f]{64})$ ]]
}

require_immutable_source_ref() {
  local ref="$1"
  local label="$2"
  if ! is_immutable_source_ref "$ref"; then
    fail "$label did not resolve to an immutable --source-ref (got '${ref:-nothing}'); an add-on records exactly a truss-vX.Y.Z release tag or a full commit SHA, and the installer stops instead of copying files"
  fi
}

# An add-on is installed the first time and updated once it is recorded. The
# record is read from the CLI-owned state file, never from the workspace.
addon_name_is_recorded() {
  local name="$1"
  local record="$TARGET_STATE_DIR/addons.json"

  [ -f "$record" ] || return 1
  grep -Fq "\"name\": \"$name\"" "$record"
}

resolve_addon_source_ref() {
  [ -n "$ADDON_SOURCE_REF" ] && return 0

  if [ "$SOURCE_MODE" = "local" ]; then
    resolve_local_addon_source_ref
  else
    resolve_remote_addon_source_ref
  fi
  require_immutable_source_ref "$ADDON_SOURCE_REF" "the Truss source"
}

# A released source records the release tag; every other git or local checkout
# records the exact commit SHA of its HEAD, never a branch name.
resolve_local_addon_source_ref() {
  command -v git >/dev/null 2>&1 ||
    fail "git is required to resolve an immutable --source-ref for a local Truss source checkout"

  local head=""
  if ! head="$(git -C "$SOURCE_ROOT" rev-parse --verify HEAD 2>/dev/null)"; then
    head=""
  fi
  [ -n "$head" ] ||
    fail "the local Truss source at $SOURCE_ROOT is not a git checkout, so no immutable --source-ref can be resolved; install add-ons from a git checkout, or from a source base URL pinned to the released ref, and never from a direct copy"

  local tag=""
  if [ -f "$SOURCE_ROOT/scripts/truss-release-tag" ]; then
    tag="$(awk 'NF && $1 !~ /^#/ { print $1; exit }' "$SOURCE_ROOT/scripts/truss-release-tag")"
  fi
  if [ -n "$tag" ] && git -C "$SOURCE_ROOT" tag --points-at HEAD 2>/dev/null | grep -Fxq "$tag"; then
    ADDON_SOURCE_REF="$tag"
    return 0
  fi
  ADDON_SOURCE_REF="$head"
}

# A raw source base URL is a released source only when the URL itself pins the
# released ref: the last URL segment must be the tag that the tag file at that
# same URL declares. A floating URL (a branch) has no immutable ref to record,
# so the installer stops instead of recording a tag for unreleased bytes.
resolve_remote_addon_source_ref() {
  local tag_file="scripts/truss-release-tag"
  local url_tag="${SOURCE_BASE_URL##*/}"

  require_immutable_source_ref "$url_tag" "the raw source base URL ($SOURCE_BASE_URL)"

  local tag_tmp=""
  local tag=""
  tag_tmp="$(mktemp)"
  if ! curl -fsSL "$SOURCE_BASE_URL/$tag_file" -o "$tag_tmp"; then
    rm -f "$tag_tmp"
    fail "could not download $SOURCE_BASE_URL/$tag_file to confirm the add-on payload ref"
  fi
  tag="$(awk 'NF && $1 !~ /^#/ { print $1; exit }' "$tag_tmp")"
  rm -f "$tag_tmp"
  if [ "$tag" != "$url_tag" ]; then
    fail "the raw source base URL pins $url_tag but $tag_file declares '${tag:-nothing}'; install add-ons from a base URL pinned to the released ref"
  fi
  ADDON_SOURCE_REF="$tag"
}

# The core version the payload was acquired with: the released ref's version,
# the installed CLI's reported version, or the local source checkout's own
# crate version when no CLI is installed yet (a dry run).
resolve_addon_source_core_version() {
  [ -n "$ADDON_SOURCE_CORE_VERSION" ] && return 0

  case "$ADDON_SOURCE_REF" in
    truss-v*)
      ADDON_SOURCE_CORE_VERSION="${ADDON_SOURCE_REF#truss-v}"
      return 0
      ;;
  esac

  local runner="$TARGET_STATE_DIR/bin/truss"
  if [ -x "$runner" ]; then
    local reported=""
    if reported="$("$runner" --version 2>/dev/null | awk '{ print $NF; exit }')" && [ -n "$reported" ]; then
      ADDON_SOURCE_CORE_VERSION="$reported"
      return 0
    fi
  fi

  local cargo_toml="$SOURCE_ROOT/crates/truss/Cargo.toml"
  [ -f "$cargo_toml" ] ||
    fail "could not resolve the source core version for the add-on install: neither an installed Truss CLI nor $cargo_toml is available"
  local version=""
  version="$(awk -F'"' '/^[[:space:]]*version[[:space:]]*=/ { print $2; exit }' "$cargo_toml")"
  [ -n "$version" ] ||
    fail "could not read the Truss source core version from $cargo_toml"
  ADDON_SOURCE_CORE_VERSION="$version"
}

# The payload bytes live in the distribution mirror at
# `distribution/payload/<destination>`, while a manifest line names the
# destination the CLI installs. This prefix is the one place that mapping is
# written down.
PAYLOAD_SOURCE_PREFIX="distribution/payload"

stage_addon_payload() {
  local name="$1"
  local manifest="$2"

  ADDON_STAGED_PAYLOAD=""
  ADDON_STAGED_MANIFEST=""

  if [ "$SOURCE_MODE" = "local" ]; then
    # The checkout's distribution mirror is the payload: no copy is made. The
    # bytes must equal the recorded commit's bytes, or the recorded ref would
    # describe something else, so a checkout whose add-on payload is not
    # committed is refused.
    assert_local_addon_payload_is_committed "$name" "$manifest"
    ADDON_STAGED_PAYLOAD="$SOURCE_ROOT/$PAYLOAD_SOURCE_PREFIX"
    ADDON_STAGED_MANIFEST="$SOURCE_ROOT/$manifest"
    return 0
  fi

  ADDON_TMP="$(mktemp -d)"
  ADDON_STAGED_PAYLOAD="$ADDON_TMP/payload"
  ADDON_STAGED_MANIFEST="$ADDON_TMP/$manifest"
  mkdir -p "$ADDON_STAGED_PAYLOAD" "$(dirname "$ADDON_STAGED_MANIFEST")"
  read_payload_manifest "$manifest" > "$ADDON_STAGED_MANIFEST"

  local relative=""
  while IFS= read -r relative || [ -n "$relative" ]; do
    relative="${relative%$'\r'}"
    case "$relative" in
      ""|\#*)
        continue
        ;;
    esac
    mkdir -p "$ADDON_STAGED_PAYLOAD/$(dirname "$relative")"
    download_file "$SOURCE_BASE_URL/$PAYLOAD_SOURCE_PREFIX/$relative" \
      "$ADDON_STAGED_PAYLOAD/$relative"
  done < "$ADDON_STAGED_MANIFEST"
}

# Prove the bytes a local run will install belong to the recorded ref.
#
# Each manifest line names a destination, so the checked path is the mirror
# source `distribution/payload/<destination>`. The check resolves that path in
# the recorded commit itself with `git cat-file`, which sees an untracked or
# ignored file as missing. That is deliberate: `git ls-files --others
# --exclude-standard` skips ignored files and `git diff --quiet HEAD` cannot see
# untracked ones, so the pair this replaced would silently stop verifying a path
# that had been ignored.
assert_local_addon_payload_is_committed() {
  local name="$1"
  local manifest="$2"
  local relative=""
  local paths=()

  while IFS= read -r relative || [ -n "$relative" ]; do
    relative="${relative%$'\r'}"
    case "$relative" in
      ""|\#*)
        continue
        ;;
    esac
    paths+=("$PAYLOAD_SOURCE_PREFIX/$relative")
  done < <(read_payload_manifest "$manifest")

  [ "${#paths[@]}" -gt 0 ] ||
    fail "the $name add-on payload manifest $manifest lists no files"

  command -v git >/dev/null 2>&1 ||
    fail "git is required to prove the local Truss payload belongs to the recorded ref"

  local head=""
  head="$(git -C "$SOURCE_ROOT" rev-parse --verify HEAD 2>/dev/null)" || head=""
  [ -n "$head" ] ||
    fail "the local Truss source at $SOURCE_ROOT is not a git checkout, so the payload bytes cannot be proven to belong to a recorded ref"

  local path="" absent=""
  for path in "${paths[@]}"; do
    if ! git -C "$SOURCE_ROOT" cat-file -e "$head:$path" 2>/dev/null; then
      absent="$absent $path"
      continue
    fi
    local committed="" worktree=""
    committed="$(git -C "$SOURCE_ROOT" rev-parse "$head:$path" 2>/dev/null)" || committed=""
    worktree="$(git -C "$SOURCE_ROOT" hash-object -- "$SOURCE_ROOT/$path" 2>/dev/null)" || worktree=""
    if [ -z "$worktree" ]; then
      absent="$absent $path(unreadable)"
    elif [ "$committed" != "$worktree" ]; then
      absent="$absent $path(differs-from-$head)"
    fi
  done

  [ -z "$absent" ] ||
    fail "the $name add-on payload is not committed in $SOURCE_ROOT:$absent; an immutable --source-ref must describe the bytes that are installed, so this installer stops instead of recording one. Run `git status` in $SOURCE_ROOT, commit the payload under $PAYLOAD_SOURCE_PREFIX/, and retry"
}

# Refuse an unsupported payload layout before anything is written. A bootstrap
# and a payload must agree on the layout: a pre-0008 tag carries the old one, and
# a raw base URL has no way to negotiate. `--source-git <tag>` stays valid for an
# old tag because that ref ships its own bootstrap.
require_supported_layout() {
  local value=""
  value="$(read_source_text "distribution/layout-version" 2>/dev/null | head -n 1)" || true
  [ -n "$value" ] ||
    fail "the Truss source at $SOURCE_ROOT declares no distribution/layout-version; this bootstrap requires layout $REQUIRED_LAYOUT and refuses to guess"
  [ "$value" = "$REQUIRED_LAYOUT" ] ||
    fail "the Truss source declares layout $value; this bootstrap requires layout $REQUIRED_LAYOUT. For a pre-0008 tag use --source-git <tag>, which ships its own bootstrap"
}

# Resolve the immutable ref, and prove that a local checkout really carries the
# payload that ref names, before any mutation. A mutable ref or a checkout whose
# add-on payload is not committed stops the installer before the core install
# writes anything.
preflight_addons() {
  if [ "$INSTALL_ENGINEERING_WISDOM" -ne 1 ] && [ "$INSTALL_DELIVERY" -ne 1 ] && [ "$INSTALL_PLANNING" -ne 1 ]; then
    return 0
  fi

  resolve_addon_source_ref
  [ "$SOURCE_MODE" = "local" ] || return 0

  if [ "$INSTALL_ENGINEERING_WISDOM" -eq 1 ]; then
    assert_local_addon_payload_is_committed "engineering-wisdom" "$ENGINEERING_WISDOM_PAYLOAD_MANIFEST"
  fi
  if [ "$INSTALL_DELIVERY" -eq 1 ]; then
    assert_local_addon_payload_is_committed "delivery" "$DELIVERY_PAYLOAD_MANIFEST"
  fi
  if [ "$INSTALL_PLANNING" -eq 1 ]; then
    assert_local_addon_payload_is_committed "planning" "$PLANNING_PAYLOAD_MANIFEST"
  fi
}

# One CLI invocation per add-on. `install` for a new add-on, `update` for one
# already recorded; the CLI then plans, preserves, updates, adopts, or stages a
# conflict, and writes the record and baseline. `--merge` and `--force` never
# reach an add-on: there is no installer-side skip or overwrite path left.
install_addon() {
  local name="$1"
  local manifest="$2"
  local operation="install"
  local runner="$TARGET_STATE_DIR/bin/truss"
  local dry_preview=0
  local status=0

  stage_addon_payload "$name" "$manifest"
  resolve_addon_source_ref
  resolve_addon_source_core_version
  if addon_name_is_recorded "$name"; then
    operation="update"
  fi

  if [ "$DRY_RUN" -eq 1 ] && [ ! -f "$TARGET_STATE_DIR/manifest.json" ]; then
    # A dry run installs no core state for the CLI to validate. Nothing is
    # copied either way; the invocation that a real run would make is reported.
    dry_preview=1
  elif [ ! -x "$runner" ]; then
    fail "the Truss CLI is required to install the $name add-on but is not available at $runner; run a full install first (a dry run installs no CLI and no core state)"
  fi

  local args=(
    addon "$operation"
    --name "$name"
    --manifest "$ADDON_STAGED_MANIFEST"
    --source "$ADDON_STAGED_PAYLOAD"
    --source-ref "$ADDON_SOURCE_REF"
    --source-core-version "$ADDON_SOURCE_CORE_VERSION"
    --directory "$TARGET_DIR"
  )
  if [ "$DRY_RUN" -eq 1 ]; then
    args+=(--dry-run)
  fi

  if [ "$dry_preview" -eq 1 ]; then
    log "add-on $name: dry run would delegate to the Truss CLI (no core state in the target yet)"
    log "  $runner ${args[*]}"
    return 0
  fi

  log "add-on $name: $operation via the Truss CLI"
  log "  $runner ${args[*]}"
  set +e
  "$runner" "${args[@]}"
  status=$?
  set -e
  if [ "$status" -eq 2 ] && [ "$DRY_RUN" -eq 1 ]; then
    log "add-on $name: the Truss CLI preview reports conflicts; the dry run changed nothing"
    return 0
  fi
  if [ "$status" -eq 2 ]; then
    fail "the $name add-on update stopped on a conflict and staged a resolution; edit the files under $TARGET_STATE_LABEL/addon-update/$name/resolved/, then run: $runner addon continue --name $name --directory $TARGET_DIR"
  fi
  if [ "$status" -ne 0 ]; then
    fail "the Truss CLI failed to $operation the $name add-on with exit code $status"
  fi
}

install_engineering_wisdom() {
  [ "$INSTALL_ENGINEERING_WISDOM" -eq 1 ] || return 0
  install_addon "engineering-wisdom" "$ENGINEERING_WISDOM_PAYLOAD_MANIFEST"
}

install_planning() {
  [ "$INSTALL_PLANNING" -eq 1 ] || return 0
  install_addon "planning" "$PLANNING_PAYLOAD_MANIFEST"
}

install_delivery() {
  [ "$INSTALL_DELIVERY" -eq 1 ] || return 0
  install_addon "delivery" "$DELIVERY_PAYLOAD_MANIFEST"
}

TARGET_INPUT="${TRUSS_TARGET_DIR:-$PWD}"
YES=0
FORCE=0
DRY_RUN=0
INSTALL_ENGINEERING_WISDOM=0
INSTALL_DELIVERY=0
INSTALL_PLANNING=0
REFRESH_AGENT_SHIM=0
INSTALL_CLAUDE_SHIM=0
REQUESTED_CONFLICT_ACTION=""
POSITIONAL_TARGET=""
SOURCE_GIT_URL="${TRUSS_SOURCE_GIT:-}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    -d|--directory)
      [ "$#" -ge 2 ] || fail "$1 requires a path"
      TARGET_INPUT="$2"
      shift 2
      ;;
    -y|--yes)
      YES=1
      shift
      ;;
    --with-engineering-wisdom)
      INSTALL_ENGINEERING_WISDOM=1
      shift
      ;;
    --with-delivery)
      INSTALL_DELIVERY=1
      shift
      ;;
    --with-planning)
      INSTALL_PLANNING=1
      shift
      ;;
    --force)
      FORCE=1
      shift
      ;;
    --merge)
      REQUESTED_CONFLICT_ACTION="merge"
      shift
      ;;
    --refresh-agent-shim)
      REFRESH_AGENT_SHIM=1
      shift
      ;;
    --claude)
      INSTALL_CLAUDE_SHIM=1
      shift
      ;;
    --override)
      REQUESTED_CONFLICT_ACTION="override"
      shift
      ;;
    --stop)
      REQUESTED_CONFLICT_ACTION="stop"
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --source-git)
      [ "$#" -ge 2 ] || fail "$1 requires a git URL"
      SOURCE_GIT_URL="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      fail "Unknown option: $1"
      ;;
    *)
      [ -z "$POSITIONAL_TARGET" ] || fail "Only one target path is supported"
      POSITIONAL_TARGET="$1"
      shift
      ;;
  esac
done

if [ "$#" -gt 0 ]; then
  [ -z "$POSITIONAL_TARGET" ] || fail "Only one target path is supported"
  POSITIONAL_TARGET="$1"
  shift
fi

[ "$#" -eq 0 ] || fail "Unexpected extra arguments"

if [ -n "$POSITIONAL_TARGET" ]; then
  TARGET_INPUT="$POSITIONAL_TARGET"
fi

SCRIPT_PATH="${BASH_SOURCE[0]:-$0}"
SCRIPT_DIR="$(cd "$(dirname "$SCRIPT_PATH")" 2>/dev/null && pwd -P || printf '')"
SOURCE_ROOT=""
SOURCE_MODE="remote"
SOURCE_BASE_URL="${TRUSS_SOURCE_BASE_URL:-}"
SOURCE_BASE_URL="${SOURCE_BASE_URL%/}"
CORE_SOURCE_BASE_URL="${TRUSS_CORE_SOURCE_BASE_URL:-}"
CORE_SOURCE_BASE_URL="${CORE_SOURCE_BASE_URL%/}"
PAYLOAD_MANIFEST="scripts/truss-install-files.txt"
ENGINEERING_WISDOM_PAYLOAD_MANIFEST="scripts/engineering-wisdom-install-files.txt"
DELIVERY_PAYLOAD_MANIFEST="scripts/delivery-install-files.txt"
PLANNING_PAYLOAD_MANIFEST="scripts/plan-install-files.txt"
# The payload layout this bootstrap stages. Decision 0008 moved the installed
# tree to `.truss/core`; a payload declaring anything else is refused.
REQUIRED_LAYOUT="3"

# A Truss source checkout is recognised by its distribution tree, which only a
# source repository carries. An installed consumer has `.truss/core/` but no
# `distribution/`, and product authority such as TRUSS.md lives in the source
# repository's own `.truss/authority/`, never inside the installed payload.
if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/../AGENTS.md" ] && \
   [ -f "$SCRIPT_DIR/../distribution/layout-version" ]; then
  SOURCE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
  SOURCE_MODE="local"
fi

# Git source mode overrides both local detection and raw-URL remote mode:
# a shallow clone is a full checkout, so the local pipeline (file copy plus
# cargo-built binary) applies and tracks the remote's latest commit.
TRUSS_GIT_TMP=""
ADDON_TMP=""
cleanup_tmp() {
  if [ -n "$TRUSS_GIT_TMP" ]; then
    rm -rf "$TRUSS_GIT_TMP"
  fi
  if [ -n "$ADDON_TMP" ]; then
    rm -rf "$ADDON_TMP"
  fi
}
trap cleanup_tmp EXIT
if [ -n "$SOURCE_GIT_URL" ]; then
  command -v git >/dev/null 2>&1 || fail "git is required for --source-git"
  TRUSS_GIT_TMP="$(mktemp -d)"
  git clone --quiet --depth 1 "$SOURCE_GIT_URL" "$TRUSS_GIT_TMP/checkout" ||
    fail "could not clone Truss source from: $SOURCE_GIT_URL"
  SOURCE_ROOT="$(cd "$TRUSS_GIT_TMP/checkout" && pwd -P)"
  SOURCE_MODE="local"
  log "Truss source (git): $SOURCE_GIT_URL"
fi

if [ "$YES" -eq 0 ] && can_prompt; then
  prompt_tty "Install Truss into [$TARGET_INPUT]: "
  REPLY_TARGET="$(read_tty)"
  if [ -n "$REPLY_TARGET" ]; then
    TARGET_INPUT="$REPLY_TARGET"
  fi
fi

# Refuse an unsupported payload layout before this run touches anything: not
# before the target directory is created, and not before an override has moved
# protected paths into a backup. The marker is read through the source reader, so
# this must sit after the source mode, source root, and source base URL are
# resolved, and before every mutation below.
require_supported_layout

TARGET_DIR="$(make_absolute_parent "$(expand_path "$TARGET_INPUT")")"
BACKUP_DIR="$TARGET_DIR/.truss-backup/$(date +%Y%m%d%H%M%S)"

# The installed tree's root inside the target. A new installation writes
# `.truss/core`; an installation that predates decision 0008 keeps
# `.truss-core`. Resolution is by presence, exactly as the CLI resolves it, so
# the bootstrap and the CLI never address different trees in one run. A target
# holding both is refused rather than guessed: two trees mean two locks and two
# baselines, and the CLI refuses the same pair.
resolve_target_state() {
  local new="$TARGET_DIR/.truss/core"
  local legacy="$TARGET_DIR/.truss-core"
  local new_installed=0 legacy_installed=0
  [ -f "$new/manifest.json" ] || [ -d "$new/base" ] && new_installed=1
  [ -f "$legacy/manifest.json" ] || [ -d "$legacy/base" ] && legacy_installed=1
  if [ "$new_installed" -eq 1 ] && [ "$legacy_installed" -eq 1 ]; then
    fail "both .truss/core and .truss-core in $TARGET_DIR hold a Truss installation; decide which tree this repository keeps before running the installer"
  fi
  if [ "$legacy_installed" -eq 1 ]; then
    TARGET_STATE_LABEL=".truss-core"
  else
    TARGET_STATE_LABEL=".truss/core"
  fi
  TARGET_STATE_DIR="$TARGET_DIR/$TARGET_STATE_LABEL"
}

resolve_target_state
CREATED=0
UPDATED=0
SKIPPED=0
CONFLICT_ACTION="install"

if [ "$DRY_RUN" -eq 1 ]; then
  log "Dry run: no files will be written."
elif [ ! -d "$TARGET_DIR" ]; then
  mkdir -p "$TARGET_DIR"
fi

if [ ! -d "$TARGET_DIR" ]; then
  [ "$DRY_RUN" -eq 1 ] || fail "Target directory could not be created: $TARGET_DIR"
  log "Target directory would be created: $TARGET_DIR"
fi

if [ -d "$TARGET_DIR" ]; then
  [ -w "$TARGET_DIR" ] || fail "Target directory is not writable: $TARGET_DIR"
else
  [ -w "$(dirname "$TARGET_DIR")" ] || fail "Target parent directory is not writable: $(dirname "$TARGET_DIR")"
fi

if [ -d "$TARGET_DIR" ]; then
  check_protected_target_paths
fi

if [ "$SOURCE_MODE" = "local" ]; then
  log "Truss source: $SOURCE_ROOT"
else
  command -v curl >/dev/null 2>&1 || fail "curl is required for remote installation"
  [ -n "$SOURCE_BASE_URL" ] || fail "remote install requires TRUSS_SOURCE_BASE_URL (raw source base URL) or TRUSS_SOURCE_GIT (git URL)"
  [ -n "$CORE_SOURCE_BASE_URL" ] || fail "remote install requires TRUSS_CORE_SOURCE_BASE_URL (raw source base URL)"
  log "Truss source: $SOURCE_BASE_URL"
fi
log "Truss profile: core"
if [ "$INSTALL_ENGINEERING_WISDOM" -eq 1 ]; then
  log "Engineering wisdom: included (explicit opt-in)"
else
  log "Engineering wisdom: excluded"
fi
if [ "$INSTALL_DELIVERY" -eq 1 ]; then
  log "Delivery add-on: included (explicit opt-in)"
else
  log "Delivery add-on: excluded"
fi
if [ "$INSTALL_PLANNING" -eq 1 ]; then
  log "Planning add-on: included (explicit opt-in)"
else
  log "Planning add-on: excluded"
fi
log "Target project: $TARGET_DIR"
log "Installed tree: $TARGET_STATE_LABEL"

preflight_addons

install_truss_core
install_engineering_wisdom
install_delivery
install_planning
refresh_agent_shim
write_claude_shim

log ""
log "Done. Created: $CREATED, updated: $UPDATED, skipped: $SKIPPED."

if [ "$SKIPPED" -gt 0 ] && [ "$FORCE" -eq 0 ]; then
  log "Existing files were left untouched. Re-run with --force to overwrite with backups."
fi

if [ "$FORCE" -eq 1 ] && [ "$UPDATED" -gt 0 ] && [ "$DRY_RUN" -eq 0 ]; then
  log "Backups were written to: $BACKUP_DIR"
fi
