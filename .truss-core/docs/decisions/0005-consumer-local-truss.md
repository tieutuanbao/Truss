# 0005 Consumer-Local Truss Installation

Date: 2026-09-25

## Status

Accepted

## Context

A consumer may run Truss inside a project checkout whose remote is public or
shared. The owner intent for that checkout is that the Truss payload, the
delivery plans, and the delivery-process artifacts must not reach the remote.

Verified before this decision:

- Consumer installation, status, and update never read the consumer's Git
  state. Both Git reads address the source checkout only
  (`scripts/install-truss.sh:705`, `:837` are `git -C "$SOURCE_ROOT"`), and the
  crate invokes Git only for `merge-file` on temporary files
  (`crates/truss/src/infrastructure/git_merge.rs:19-38`) plus `git --version`.
- Update compares installed base, local bytes, and incoming bytes. Provenance
  lives in `.truss-core/manifest.json` and `.truss-core/addons.json`
  (`source_ref` plus a SHA-256 per file), so version identity does not depend
  on consumer Git history.
- `README.md:413-417` already permits a local-only installation.
- Git ignore does not affect files that are already tracked. A consumer that
  already tracks payload keeps publishing it until those paths are explicitly
  untracked.

## Decision

1. Consumer-local Truss is supported as an explicit opt-in. Installation
   defaults do not change and no consumer is migrated automatically.
2. The consumer establishes the ignore rules through the local Git exclude file,
   not through the committed root `.gitignore`, so that no configuration trace
   of Truss reaches the remote. Resolve the real path rather than assuming
   `.git` is a directory:

   ```bash
   git -C "$CONSUMER_ROOT" rev-parse --path-format=absolute --git-path info/exclude
   ```

   Add idempotently, after confirming path ownership:

   ```gitignore
   /.truss-core/
   /.truss-delivery/
   ```

   Then add each installed Truss skill directory, enumerated from
   `scripts/truss-install-files.txt` and the add-on manifests, for example:

   ```gitignore
   /.agents/skills/delivery/
   /.agents/skills/delivery-setup/
   /.agents/skills/encode-invariant/
   ```

   Never ignore the whole `.agents/` tree: it may hold project-owned skills.
3. `/AGENTS.md` and `/CLAUDE.md` are added to the exclude list only when the
   owner confirms that the entire file, not only the managed Truss block, is
   local-only and not tracked. These are mixed files: the installer merges or
   refreshes a marked block inside consumer-owned content
   (`scripts/install-truss.sh:252-270`), and `:73` records that `AGENTS.md` is
   never an add-on payload file. Partial exclusion of a single file is not
   available in Git.
4. The transient delivery plan path is
   `.truss-delivery/runs/<run-key>/plan.md`, local to the approved candidate
   root. It is never staged and never committed.
5. The Truss source repository, including this one, is out of scope. A source
   checkout keeps its manifest-listed payload committed at `HEAD`, as required
   by the source-payload contract in
   `.truss-core/docs/product/installation-profiles.md`. No blanket ignore, no
   untrack, and no history rewrite applies to a source checkout.
6. A consumer that already tracks payload is not migrated silently. Untracking
   requires `git rm --cached` and a visible commit, and earlier commits still
   contain the content. Removing it from history requires a rewrite this
   decision does not recommend.
7. Git ignore is not a security boundary. `git add -f`, manual upload, CI logs,
   and pull-request bodies can still disclose content.

## Alternatives Considered

1. A committed rule in the consumer root `.gitignore`. Rejected: it publishes a
   configuration trace of Truss to the remote, which this decision exists to
   avoid.
2. Ignoring the whole `.agents/` tree. Rejected: it can hide project-authored
   or third-party skills that belong in the project.
3. Blanket ignore or untrack in the Truss source repository. Rejected: the
   source-payload contract refuses a payload path that is untracked or that
   differs from `HEAD`, so distribution would break.
4. `assume-unchanged` or `skip-worktree` for a shared `AGENTS.md`. Rejected:
   these hide content from the index instead of deciding ownership, and they
   fail on a fresh clone.
5. Rewriting consumer history to remove the payload. Out of scope; not
   recommended.

## Consequences

Positive:

- A consumer can adopt Truss without publishing payload, plans, or delivery
  process artifacts.
- Core installation, status, and update are unaffected by the ignore rules.
- Version identity survives without consumer Git history, through
  `manifest.json` and `addons.json`.

Tradeoffs:

- `.git/info/exclude` is per-clone. A fresh clone must install Truss and
  re-establish the exclude rules; teammates do not inherit them.
- A consumer with already-tracked payload keeps publishing it until an explicit
  untracking commit, and earlier commits retain the bytes.
- Mixed files (`AGENTS.md`, `CLAUDE.md`) cannot be partially ignored. Excluding
  the whole file removes the agent entrypoint from a fresh clone.
- Delivery process artifacts are absent from Git, so an independent accepting
  session must receive the approved artifacts through an authorized private
  handoff rather than through a clone.
- Ignored run artifacts are not protected: `git clean -fdx` can delete them.
  The existing delivery prohibition on `clean` (`.agents/skills/delivery/SKILL.md:215`)
  binds the delivery run, not the user or another tool.

## Follow-Up

- Open: the installer still appends the maintenance binary rules to the
  consumer root `.gitignore` (`scripts/install-truss.sh:426-448`, `:548`;
  `scripts/install-truss.ps1:193-207`, `:309`). A local-only install mode should
  write them to the resolved `info/exclude` instead. The Bash and PowerShell
  skip logic differ (`scripts/install-truss.sh:431` checks only the two binary
  rules, `scripts/install-truss.ps1:194-197` also counts the marker) and both
  need updating.
- Open: the `AGENTS.md` and `CLAUDE.md` treatment for consumers that already
  track those files.
- Proposed, not accepted: a private accepted-envelope binding for Architectural
  delivery. The delivery contract currently requires committing
  `templates/business-analysis.md`, the approved decision record, and the
  transient plan (`.agents/skills/delivery/SKILL.md:122-126`), and places the
  Architectural execution envelope in that committed plan (`:223-225`). This
  needs its own decision record; until it is accepted, Architectural delivery
  is incompatible with a fully local-only candidate.
- Note: this repository ignores `.truss-core/docs/decisions/*`
  (`.gitignore:26`), yet decisions 0001-0004 are tracked, because ignore rules
  do not affect tracked files. Whether this record is force-added or stays
  local-only is an owner decision.
