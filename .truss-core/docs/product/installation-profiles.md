# Installation Contract

Truss has one product profile and independent opt-in add-ons.

## Core

The exact core payload is declared in
`scripts/truss-install-files.txt`. It contains generic repository guidance,
working-memory structure, an invariant-encoding pattern and skill, and
explicit-only onboarding and improvement skills.

The platform bootstrap installs a checksum-verified `truss` binary under
`.truss/core/bin/` and delegates installation or update to that candidate.

`.truss/` carries three namespaces with three owners. The CLI owns and writes
only `core/`, which holds the installed payload plus the installation state
(`manifest.json`, `addons.json`, the lock, the transaction record, `base/`,
`base-addons/`, and the update namespaces). `delivery/` holds one delivery run's
working memory — its plan, its approved envelope, its dispatch artifacts, and its
approval receipt — and is written by the delivery skill, never committed, and
never touched by the CLI. `authority/` holds that project's own durable authority such as
architecture notes and decision records. A repository holding a legacy
`.truss-core/` installation is read at that root; a repository holding both roots
is refused rather than resolved by precedence.

Core installation:

- records exact upstream bytes under `.truss/core/`;
- preserves consumer files through merge or human-directed conflict handling;
- backs up replaced files;
- does not install an application stack or product policy;
- does not install schemas, databases, orchestration, or background processes.

## Engineering Wisdom Add-On

`--with-engineering-wisdom` or `-WithEngineeringWisdom` copies the
explicit-only advisory skill declared in
`scripts/engineering-wisdom-install-files.txt`.

Omitting the flag does not install or activate the skill. A later install
without the flag leaves an existing copy untouched. Removal is explicit and
stateless: delete only `.agents/skills/engineering-wisdom/`.

Advice cannot establish consumer policy or authorize an architecture rewrite.

## Delivery Add-On

`--with-delivery` or `-WithDelivery` copies the explicit-only delivery
control protocol declared in `scripts/delivery-install-files.txt`: the
`$delivery` and `$delivery-setup` skills, their supporting references,
and their business-analysis, decision-record, and plan templates.

Omitting the flag does not install or activate the add-on. A later install
without the flag leaves an existing copy untouched. Removal is explicit and
stateless: delete only `.agents/skills/delivery/` and
`.agents/skills/delivery-setup/`.

The delivery add-on requires the Orca execution plane at run time. On Linux,
install the Orca AppImage from <https://www.onorca.dev/.truss-core/docs/install>, register
its CLI under **Settings → Experimental → CLI**, and enable **Settings →
Experimental → Orchestration**. The registered CLI is normally `orca-ide`, not
the GNOME screen-reader binary named `orca`. Installing the add-on configures
nothing and starts nothing.

## Merge And Override

- `--merge` / `-Merge`: preserve existing files and add missing managed
  paths.
- `--override` / `-Override`: back up and replace protected Truss paths.
- `--force` / `-Force`: overwrite individual managed files with backups.
- `--dry-run` / `-DryRun`: preview without writing.

## Update

`truss update` verifies release identity and checksum, compares installed
base, local bytes, and incoming bytes, and applies the complete plan
transactionally.

Overlapping text edits stage a frozen resolution session. Structural conflicts
must be corrected before replanning. Successful activation writes provenance
last and replaces only the selected repository's executable after core files
succeed.

## Release Repository Configuration

The bootstrap and the `truss` binary carry no publisher identity. Remote
installation and self-update require explicit configuration:

- `TRUSS_SOURCE_GIT` (or `--source-git URL`): a git URL to shallow-clone the
  source checkout from. The clone is treated as a local source: files are
  copied from it and the CLI binary is built from it, so the install tracks
  the remote's latest commit. Requires `git` and a current Rust toolchain.
- `TRUSS_SOURCE_BASE_URL` and `TRUSS_CORE_SOURCE_BASE_URL`: raw base URLs
  of a published source checkout, for bootstrap file downloads.
- `TRUSS_RELEASE_REPO` (`owner/name`) or `TRUSS_CORE_CLI_BASE_URL`: where
  core release binaries are downloaded from.
- `TRUSS_CORE_RELEASE_TAG`: pin a specific `truss-vX.Y.Z` core release.

Local-source installs (running the bootstrap from a checkout) need none of
these.

## Add-On Source Rules

An add-on payload is recorded with one immutable `source_ref` and a SHA-256 for
each declared file. The installer resolves that ref and proves the payload
before any mutation:

- A released source records the exact `truss-vX.Y.Z` release tag, and only when
the checkout is exactly that release: the tag must be declared by
`scripts/truss-release-tag` and point at `HEAD` (`git tag --points-at HEAD`). A
local or `--source-git` checkout that is not exactly that release records the
exact 40/64-character commit SHA of `HEAD` instead.
- A branch name, `HEAD`, a short SHA, or any other ref that can move is never
accepted. The installer stops with a clear message before opening any state.
- `TRUSS_SOURCE_BASE_URL` raw mode must be tag-pinned: the final URL segment
must be an immutable ref, and `scripts/truss-release-tag` downloaded from that
same URL must declare the same ref. A floating base URL (for example `/main`)
or a tag mismatch stops the add-on step; a core-only install is unaffected.
- A checkout whose manifest-listed payload paths are not committed at `HEAD` is
refused. An untracked payload path, or one that differs from `HEAD`, stops
before any mutation rather than recording a ref that does not describe the
installed bytes.

## Add-On Release Gate

The current proof for the add-on install path is the committed rehearsal:

```bash
bash tests/s5-rehearse.sh
```

`scripts/validate-premerge.sh` runs it. After tagging `truss-vX.Y.Z`, the
released source at that tag must reach the same add-on tree and the same
`addons.json` provenance as the fresh source at the same ref:

```bash
tag=truss-vX.Y.Z
base="https://raw.githubusercontent.com/tieutuanbao/Truss/$tag"

# 1. Fresh source: the tagged checkout itself.
git checkout --detach "$tag"
scripts/install-truss.sh --directory /tmp/truss-fresh \
  --with-engineering-wisdom --with-delivery --with-planning --yes

# 2. Released source: the raw base URL pinned to the same tag.
export TRUSS_SOURCE_BASE_URL="$base"
export TRUSS_CORE_SOURCE_BASE_URL="$base"
export TRUSS_RELEASE_REPO=tieutuanbao/Truss
curl -fsSL "$base/scripts/install-truss.sh" | bash -s -- \
  --directory /tmp/truss-released \
  --with-engineering-wisdom --with-delivery --with-planning --yes

# 3. Same add-on tree: every add-on manifest path is byte-identical.
for manifest in scripts/engineering-wisdom-install-files.txt \
                scripts/delivery-install-files.txt \
                scripts/plan-install-files.txt; do
  while IFS= read -r p; do
    case "$p" in ""|\#*) continue ;; esac
    cmp "/tmp/truss-fresh/$p" "/tmp/truss-released/$p"
  done < "$manifest"
done

# 4. Same provenance: identical addons.json.
cmp /tmp/truss-fresh/.truss/core/addons.json \
    /tmp/truss-released/.truss/core/addons.json
```

Both installs must record `source_ref=truss-vX.Y.Z`. The released-source half
of this gate cannot run before the tag and its release binaries exist, so it is
a release gate, not a current proof.

## Add-On Limits

- **Windows parity is static verification only on a host without `pwsh`.** The
  rehearsal compares the PowerShell and Bash argument construction statically,
  declares the execution gap, and claims no byte-level cross-platform equality.
  Windows execution and lock semantics remain unverified until a host with
  `pwsh` runs the same rehearsal.
- **A dirty add-on payload is refused, not recorded.** A checkout whose
  manifest-listed payload paths are untracked or differ from `HEAD` stops before
  any mutation. No dirty provenance format is invented; the recorded
  `source_ref` always describes the installed bytes.
