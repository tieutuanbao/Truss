# Maintenance and updates

Truss records provenance and managed baselines so updates can preserve local
work instead of overwriting it.

## Core status and updates

Run from the installed repository:

```bash
# Inspect local changes to managed files.
.truss/core/bin/truss status

# Validate installation and transaction health.
.truss/core/bin/truss doctor

# Preview the incoming update.
.truss/core/bin/truss update --dry-run

# Apply it.
.truss/core/bin/truss update
```

Updates verify source checksums, preserve consumer edits, and use three-way
merging for managed text. Do not delete `.truss/core/`; it contains the baseline
and provenance required for safe updates.

## Resolve an update conflict

When local and incoming edits overlap, Truss stages conflict copies under:

```text
.truss/core/update/resolved/
```

Edit only those copies, then continue:

```bash
.truss/core/bin/truss update --continue
```

## Add-on lifecycle

Each optional add-on has its own immutable provenance and managed baseline:

```bash
.truss/core/bin/truss addon status --name delivery --directory .
.truss/core/bin/truss addon install --name delivery --directory . \
  --manifest scripts/delivery-install-files.txt \
  --source <staged-payload-dir> --source-ref truss-vX.Y.Z
.truss/core/bin/truss addon update --name delivery --directory . \
  --manifest scripts/delivery-install-files.txt \
  --source <staged-payload-dir> --source-ref truss-vX.Y.Z
.truss/core/bin/truss addon continue --name delivery --directory .
.truss/core/bin/truss addon abort --name delivery --directory .
```

`--source-ref` must be an immutable release tag or an exact 40/64-character
commit SHA. Branch names, `HEAD`, uncommitted payload files, and payload bytes
that differ from the selected revision are refused before mutation.

If add-on changes overlap local edits, the update stops with exit code `2` and
stages its frozen resolution under:

```text
.truss/core/addon-update/<name>/resolved/
```

Edit those copies and run `addon continue`. `addon abort` removes only that
session and leaves managed files unchanged.

## Ownership boundaries

- `.truss/core/` — installed payload and CLI-owned state.
- `.truss/delivery/` — local Delivery run memory; never committed.
- `.truss/authority/` — the repository's own decisions and plans; never written by the CLI.
- `.agents/skills/` — core and optional skills discovered by agent tools.

`AGENTS.md` is owned by the Core profile, not by an add-on. The CLI refuses an
add-on whose manifest overlaps Core or another recorded add-on.
