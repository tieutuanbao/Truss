# Installation

Use this guide when you need installation options beyond the README quick start.
Truss copies managed files into a target repository; it does not invoke skills,
start services, or modify global agent configuration.

## Requirements

- Git
- A current Rust toolchain when building from a source checkout
- Bash on Linux/macOS or PowerShell on Windows

Run installers from the project root and use `--directory .` / `-Directory .`.

## Core and add-ons

| Profile | Bash flag | PowerShell flag | Purpose |
| --- | --- | --- | --- |
| Core | none | none | Agent entry point, workflow, templates, and core skills |
| Engineering Wisdom | `--with-engineering-wisdom` | `-WithEngineeringWisdom` | Explicit engineering review guidance |
| Planning | `--with-planning` | `-WithPlanning` | Product planning from an idea |
| Delivery | `--with-delivery` | `-WithDelivery` | Multi-agent delivery through Orca |

Add-ons are independent. Installation does not invoke them, and omitting a flag
does not remove an add-on that is already installed.

## Install from a pinned release

Remote add-on installation requires an immutable source tag. Replace the version
below when a newer release is available.

### Linux and macOS

```bash
version=truss-v0.2.14
base="https://raw.githubusercontent.com/tieutuanbao/Truss/$version"
export TRUSS_SOURCE_BASE_URL="$base"
export TRUSS_CORE_SOURCE_BASE_URL="$base"
export TRUSS_RELEASE_REPO=tieutuanbao/Truss
curl -fsSL "$base/scripts/install-truss.sh" \
  | bash -s -- --directory . --with-delivery --yes
```

### Windows PowerShell

```powershell
$version = "truss-v0.2.14"
$base = "https://raw.githubusercontent.com/tieutuanbao/Truss/$version"
$env:TRUSS_SOURCE_BASE_URL = $base
$env:TRUSS_CORE_SOURCE_BASE_URL = $base
$env:TRUSS_RELEASE_REPO = "tieutuanbao/Truss"
$script = Invoke-RestMethod "$base/scripts/install-truss.ps1"
& ([scriptblock]::Create($script)) -Directory . -WithDelivery -Yes
```

## Install from a checkout

```bash
git clone https://github.com/tieutuanbao/Truss.git
cd Truss
scripts/install-truss.sh --directory /path/to/project --with-delivery --yes
```

PowerShell:

```powershell
git clone https://github.com/tieutuanbao/Truss.git
Set-Location Truss
.\scripts\install-truss.ps1 -Directory C:\path\to\project -WithDelivery -Yes
```

## Existing repositories

Protected files such as `AGENTS.md` are never silently overwritten. Preview an
installation first:

```bash
scripts/install-truss.sh --directory /path/to/project \
  --with-delivery --merge --dry-run --yes
```

Remove `--dry-run` after reviewing the plan. Use `--override` only when replacing
protected Truss paths is intentional; backups are created before replacement.

| Bash | PowerShell | Effect |
| --- | --- | --- |
| `--dry-run` | `-DryRun` | Preview only |
| `--merge` | `-Merge` | Keep protected files and install missing managed paths |
| `--override` | `-Override` | Back up and replace protected Truss paths |
| `--force` | `-Force` | Back up and overwrite individual managed files |
| `--refresh-agent-shim` | `-RefreshAgentShim` | Refresh the managed Truss block in `AGENTS.md` |
| `--claude` | — | Install or refresh the Bash `CLAUDE.md` shim |

## What is installed

Core owns `AGENTS.md`, `.truss/core/`, and its core skills under
`.agents/skills/`. Optional profiles add their own skills. Project code, tests,
continuous integration, and existing application documentation remain in place.

`.truss/` holds three namespaces with three owners:

| Path | Owner | Contents |
| --- | --- | --- |
| `.truss/core/` | the Truss CLI | Installed payload, manifest, baselines, and update state |
| `.truss/delivery/` | the Delivery skill | Run coordination: transient plan, approved envelope, prompts, handoffs, run state, and approval receipts |
| `.truss/authority/` | the repository | Durable authority: `product/`, `architecture/`, `decisions/`, `plans/`, and `communication.md` |

Placement follows purpose and lifecycle, not filename. A repository-hosted
consumer commits `.truss/core/` and its approved durable authority. A
consumer-local consumer ignores `.truss/` with one rule and never stages,
commits, or force-adds authority. See
[Maintenance](maintenance.md) for the full ownership and placement rules.

## Verify

```bash
.truss/core/bin/truss doctor
.truss/core/bin/truss status
```

`doctor` validates provenance and transaction health. `status` reports the
installed version and local differences from the managed baseline.

## Start using Core

Open your coding agent in the installed project root and begin read-only:

```text
Read AGENTS.md and .truss/core/docs/WORKFLOW.md. Then explain:
1. where product intent and architecture are documented;
2. how to build, test, lint, and run the application;
3. which checks are required before completion; and
4. which important facts are missing or ambiguous.
Do not change files. Separate documented facts from observations and gaps.
```

This confirms that the agent can retrieve repository instructions without
permission to invent missing policy.

### Choose how agents explain their work

```text
Help me configure how agents explain their work in this repository.
Show expert, intermediate, and layperson styles using the same example.
Wait for my choice before saving the repository default.
```

The saved choice lives under `.truss/authority/`. A conversation can request a
different style without changing the repository default.

### Onboard an unfamiliar repository

```text
$onboard-repository inspect this repository, trace one real operational path,
and propose the smallest agent-facing documentation improvements. Keep the
first pass read-only. Do not install dependencies, start services, or edit files.
```

For higher-risk repositories, audit the proposal before applying it:

```text
$audit-onboarding-proposal audit the onboarding result and its proposed patch.
Do not apply changes.
```

The project must supply its real setup, architecture, test, release, and
operational commands. Do not fill unknown fields with generic guesses.

## Daily prompts

| Goal | Prompt |
| --- | --- |
| Ask or diagnose | `Explain why <behavior> occurs. Inspect only; do not change files.` |
| Review | `Review <scope>. Report findings first with paths and proof. Do not edit.` |
| Small change | `Change <specific behavior>. Preserve unrelated work and run focused and required checks.` |
| Enforce an accepted rule | `$encode-invariant enforce <documented rule> from <authority path>. Include positive and negative proof.` |
| Improve recurring friction | `$improve-truss improve <observed friction> using the recorded failed trajectory.` |

Create one execution plan under `.truss/authority/plans/active/` only when work
must survive sessions, coordinate contributors, track dependencies, or recover
safely. Small, clear changes need no durable plan.
