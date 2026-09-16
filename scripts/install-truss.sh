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
                         AGENTS.md and .truss-core/docs/.
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
  download and no database write. If AGENTS.md or .truss-core/docs/ already exist,
  interactive installs ask
  whether to merge missing files, override after backup, or stop. Merge is the
  safe update path for repositories that already have Truss: existing files
  stay in place and new Truss files are appended by path. Non-
  interactive installs stop unless --merge or --override is provided. If a
  target .gitignore receives only the Rust maintenance binary rules.

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

copy_file() {
  local relative="$1"
  local target="$TARGET_DIR/$relative"

  if [ -e "$target" ]; then
    if [ "$SOURCE_MODE" = "local" ] && [ "$SOURCE_ROOT/$relative" -ef "$target" ]; then
      log "skip     $relative (source file)"
      SKIPPED=$((SKIPPED + 1))
      return
    fi

    if [ "$FORCE" -eq 1 ]; then
      if [ "$DRY_RUN" -eq 1 ]; then
        log "overwrite $relative (backup first)"
      else
        local backup="$BACKUP_DIR/$relative"
        mkdir -p "$(dirname "$backup")"
        cp -p "$target" "$backup"
        write_source_file "$relative" "$target"
        log "updated $relative (backup: ${backup#$TARGET_DIR/})"
      fi
      UPDATED=$((UPDATED + 1))
    elif [ "$CONFLICT_ACTION" = "merge" ]; then
      log "skip     $relative (merge keeps existing file)"
      SKIPPED=$((SKIPPED + 1))
    else
      log "skip     $relative (already exists)"
      SKIPPED=$((SKIPPED + 1))
    fi
    return
  fi

  if [ "$DRY_RUN" -eq 1 ]; then
    log "create   $relative"
  else
    mkdir -p "$(dirname "$target")"
    write_source_file "$relative" "$target"
    log "created  $relative"
  fi
  CREATED=$((CREATED + 1))
}

write_source_file() {
  local relative="$1"
  local target="$2"

  if [ "$relative" = "AGENTS.md" ]; then
    {
      printf '# Agent Instructions\n\n'
      agent_shim_block
    } > "$target"
    return
  fi

  if [ "$SOURCE_MODE" = "local" ]; then
    local source="$SOURCE_ROOT/$relative"
    [ -f "$source" ] || fail "Source file missing: $source"
    cp -p "$source" "$target"
    return
  fi

  local url="$SOURCE_BASE_URL/$relative"
  curl -fsSL "$url" -o "$target" || fail "Could not download $url"
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

copy_manifest_files() {
  local payload_manifest="$1"
  local manifest
  local relative

  manifest="$(read_payload_manifest "$payload_manifest")"
  while IFS= read -r relative || [ -n "$relative" ]; do
    relative="${relative%$'\r'}"
    case "$relative" in
      ""|\#*)
        continue
        ;;
    esac
    copy_file "$relative"
  done <<EOF
$manifest
EOF
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
  local unix_rule=".truss-core/bin/truss"
  local windows_rule=".truss-core/bin/truss.exe"
  if [ -f "$target" ] && grep -Fxq "$unix_rule" "$target" && grep -Fxq "$windows_rule" "$target"; then
    log "skip     .gitignore (Truss core binary rules already present)"
    return
  fi
  if [ "$DRY_RUN" -eq 1 ]; then
    log "update   .gitignore (append Truss core binary rules)"
    return
  fi
  local missing_rules=()
  [ -f "$target" ] && grep -Fxq "$unix_rule" "$target" || missing_rules+=("$unix_rule")
  [ -f "$target" ] && grep -Fxq "$windows_rule" "$target" || missing_rules+=("$windows_rule")
  {
    [ -s "$target" ] && printf '\n'
    printf '%s\n' "$marker"
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
  [ -f "$TARGET_DIR/.truss-core/manifest.json" ] && command="update"
  CORE_PENDING_VERSION=""
  if [ "$command" = "update" ] && [ -f "$TARGET_DIR/.truss-core/update/session.json" ]; then
    CORE_PENDING_VERSION="$(sed -n 's/.*"to_version":[[:space:]]*"\([^"]*\)".*/\1/p' "$TARGET_DIR/.truss-core/update/session.json" | head -n 1)"
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
    binary_target="$TARGET_DIR/.truss-core/bin/truss"
    binary_temp="$TARGET_DIR/.truss-core/bin/.truss.$$.tmp"
    [ ! -L "$TARGET_DIR/.truss-core" ] || fail "refusing symlink for .truss-core"
    [ ! -L "$TARGET_DIR/.truss-core/bin" ] || fail "refusing symlink for .truss-core/bin directory"
    [ ! -L "$binary_target" ] || fail "refusing symlink for repository Truss executable"
    mkdir -p "$(dirname "$binary_target")"
    cp "$CORE_STAGED_BINARY" "$binary_temp"
    chmod 755 "$binary_temp"
    if [ -e "$binary_target" ]; then
      mkdir -p "$BACKUP_DIR/.truss-core/bin"
      cp -p "$binary_target" "$BACKUP_DIR/.truss-core/bin/truss"
    fi
  fi
  set +e
  "$runner" "${args[@]}"
  local command_status=$?
  set -e
  if [ "$command_status" -eq 2 ] && [ "$DRY_RUN" -eq 0 ]; then
    local retained="$TARGET_DIR/.truss-core/update-candidate/truss"
    [ ! -L "$TARGET_DIR/.truss-core" ] || fail "refusing symlink for .truss-core"
    [ ! -L "$TARGET_DIR/.truss-core/update-candidate" ] || fail "refusing symlink for retained candidate directory"
    [ ! -L "$retained" ] || fail "refusing symlink for retained update candidate"
    mkdir -p "$(dirname "$retained")"
    cp "$CORE_STAGED_BINARY" "$retained"
    chmod 755 "$retained"
  fi
  if [ "$command_status" -eq 0 ] && [ "$DRY_RUN" -eq 0 ]; then
    mv -f "$binary_temp" "$binary_target"
    rm -rf "$TARGET_DIR/.truss-core/update-candidate"
    merge_core_gitignore "$TARGET_DIR/.gitignore"
    log "installed .truss-core/bin/truss ($CORE_PLATFORM)"
  elif [ -n "$binary_temp" ]; then
    rm -f "$binary_temp"
  fi
  rm -rf "$CORE_STAGE_ROOT"
  CORE_STAGE_ROOT=""
  if [ "$command_status" -eq 2 ]; then
    fail "Truss core update needs resolution; edit .truss-core/update/resolved/, then rerun this installer or truss update --continue"
  fi
  [ "$command_status" -eq 0 ] || fail "truss $command failed with exit code $command_status"
}

check_protected_target_paths() {
  local conflicts=()

  [ -e "$TARGET_DIR/AGENTS.md" ] && conflicts+=("AGENTS.md")
  [ -e "$TARGET_DIR/.truss-core" ] && conflicts+=(".truss-core/")
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
    printf '  2. Override Back up and replace AGENTS.md and .truss-core/docs/\n'
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

  for protected in AGENTS.md .truss-core; do
    [ -e "$TARGET_DIR/$protected" ] || continue

    if [ "$DRY_RUN" -eq 1 ]; then
      log "override $protected (backup first)"
      continue
    fi

    mkdir -p "$BACKUP_DIR"
    mv "$TARGET_DIR/$protected" "$BACKUP_DIR/$protected"
    log "removed  $protected (backup: ${BACKUP_DIR#$TARGET_DIR/}/$protected)"
  done

}

install_engineering_wisdom() {
  [ "$INSTALL_ENGINEERING_WISDOM" -eq 1 ] || return 0
  copy_manifest_files "$ENGINEERING_WISDOM_PAYLOAD_MANIFEST"
}

install_planning() {
  [ "$INSTALL_PLANNING" -eq 1 ] || return 0
  copy_manifest_files "$PLANNING_PAYLOAD_MANIFEST"
}

install_delivery() {
  [ "$INSTALL_DELIVERY" -eq 1 ] || return 0
  copy_manifest_files "$DELIVERY_PAYLOAD_MANIFEST"
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

if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/../AGENTS.md" ] && [ -f "$SCRIPT_DIR/../.truss-core/docs/TRUSS.md" ]; then
  SOURCE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
  SOURCE_MODE="local"
fi

# Git source mode overrides both local detection and raw-URL remote mode:
# a shallow clone is a full checkout, so the local pipeline (file copy plus
# cargo-built binary) applies and tracks the remote's latest commit.
TRUSS_GIT_TMP=""
cleanup_git_tmp() {
  [ -n "$TRUSS_GIT_TMP" ] && rm -rf "$TRUSS_GIT_TMP"
}
if [ -n "$SOURCE_GIT_URL" ]; then
  command -v git >/dev/null 2>&1 || fail "git is required for --source-git"
  TRUSS_GIT_TMP="$(mktemp -d)"
  trap cleanup_git_tmp EXIT
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

TARGET_DIR="$(make_absolute_parent "$(expand_path "$TARGET_INPUT")")"
BACKUP_DIR="$TARGET_DIR/.truss-backup/$(date +%Y%m%d%H%M%S)"
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
