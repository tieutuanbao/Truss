# Development and releases

This document is for Truss contributors. Consumer repositories normally need
only the root README and the installed documentation under `.truss/core/docs/`.

## Repository layout

| Path | Purpose |
| --- | --- |
| `crates/truss/` | Rust installer, updater, and state management |
| `distribution/payload/` | The only source of installed payload bytes |
| `distribution/entrypoints/` | Managed instruction blocks |
| `scripts/` | Bootstrap, manifests, validation, and release helpers |
| `docs/` | Source-repository installation, Delivery, maintenance, and contributor docs |

The Rust binary embeds the distribution payload. Add or remove a shipped file
through its owning manifest and update the reviewed digest baseline.

## Local validation

Run the complete source validation entry point:

```bash
scripts/validate-premerge.sh
```

It checks shell syntax, Rust formatting, all workspace tests, Clippy warnings,
payload layout, reviewed hashes, entrypoint parity, shipped final newlines, and
the current Git diff. Development requires a current stable Rust toolchain,
Git, and ripgrep.

For a focused payload check:

```bash
cargo test -p truss --test payload_layout
```

## Installer coverage

Rust integration tests exercise install, update, migration, release, and add-on
flows in throwaway workspaces. The payload-layout suite checks manifests,
generated entrypoint parity, and reviewed bytes. These tests run through
`scripts/validate-premerge.sh`; no separate installer rehearsal script exists.

## Release proof

Before tagging, the pre-merge gate above is the current proof. After creating
`truss-vX.Y.Z`, compare a fresh tagged checkout with installation through the
published raw source at the same tag:

```bash
tag=truss-vX.Y.Z
base="https://raw.githubusercontent.com/tieutuanbao/Truss/$tag"

# Fresh source checkout.
git checkout --detach "$tag"
scripts/install-truss.sh --directory /tmp/truss-fresh \
  --with-engineering-wisdom --with-delivery --with-planning --yes

# Published source at the same immutable tag.
export TRUSS_SOURCE_BASE_URL="$base"
export TRUSS_CORE_SOURCE_BASE_URL="$base"
export TRUSS_RELEASE_REPO=tieutuanbao/Truss
curl -fsSL "$base/scripts/install-truss.sh" | bash -s -- \
  --directory /tmp/truss-released \
  --with-engineering-wisdom --with-delivery --with-planning --yes
```

Every add-on manifest path and both generated `addons.json` files must match
byte-for-byte. Both installs must record `source_ref=truss-vX.Y.Z`.

The published-source half cannot run before the tag and release binaries exist;
it is a release gate, not current proof. On a host without PowerShell, Windows
parity remains static verification rather than an executed cross-platform claim.

## Scripts reference

| File | Purpose |
| --- | --- |
| `install-truss.sh` | Linux/macOS bootstrap |
| `install-truss.ps1` | Windows PowerShell bootstrap |
| `validate-premerge.sh` | Complete validation entry point |
| `truss-install-files.txt` | Core payload manifest |
| `engineering-wisdom-install-files.txt` | Engineering Wisdom manifest |
| `plan-install-files.txt` | Planning manifest |
| `delivery-install-files.txt` | Delivery manifest |
| `agent-truss-block.md` | Managed `AGENTS.md` block |
| `claude-truss-block.md` | Bash `CLAUDE.md` import block |
| `truss-release-tag` | Current release pointer |
