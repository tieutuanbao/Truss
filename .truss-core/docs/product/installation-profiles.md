# Installation Contract

Truss has one product profile and independent opt-in add-ons.

## Core

The exact core payload is declared in
`scripts/truss-install-files.txt`. It contains generic repository guidance,
working-memory structure, an invariant-encoding pattern and skill, and
explicit-only onboarding and improvement skills.

The platform bootstrap installs a checksum-verified `truss` binary under
`.truss-core/bin/` and delegates installation or update to that candidate.

Core installation:

- records exact upstream bytes under `.truss-core/`;
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
and their decision-record and plan templates.

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
