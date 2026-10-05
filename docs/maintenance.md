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

## Artifact ownership

`.truss/` holds three namespaces with three owners:

| Path | Owner | Contents |
| --- | --- | --- |
| `.truss/core/` | the Truss CLI | Installed payload, manifest, baselines, and update state |
| `.truss/delivery/` | the Delivery skill | Run coordination: transient plan, approved envelope, prompts, handoffs, run state, and approval receipts |
| `.truss/authority/` | the repository | Durable authority: `product/`, `architecture/`, `decisions/`, `plans/active/`, and `plans/completed/` |

Classify an artifact by purpose and lifecycle, not by filename. Content that
must survive the run is authority; content used only to dispatch, communicate,
hand off, bind an approval, or track run state is run coordination. Split a
mixed-purpose artifact between a run-local copy and the owning authority record.
Ambiguous classification, persistence, or retrieval authority returns
`NEEDS_INPUT`.

A repository-hosted consumer commits approved durable authority in the approved
baseline. When an ignore rule matches an approved authority path, use
path-scoped `git add -f -- <path>` only if the approved envelope grants staging
and commit authority. A consumer-local consumer keeps authority
repository-local and never stages, commits, or force-adds it.

`AGENTS.md` is owned by the Core profile, not by an add-on. The CLI refuses an
add-on whose manifest overlaps Core or another recorded add-on.
