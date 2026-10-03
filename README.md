<div align="center">

# 🔺 Truss

**A clear, durable workspace for coding agents — owned by your repository.**

[![Release](https://img.shields.io/badge/release-v0.2.12-2563eb)](scripts/truss-release-tag)
[![License](https://img.shields.io/badge/license-MIT-16a34a)](LICENSE)

Repository-owned guidance · Safe updates · Optional multi-agent delivery

</div>

---

Truss gives agents one entry point and an evidence-based workflow while your repository remains the source of truth.

## Common use cases

| Problem | How Truss helps |
| --- | --- |
| The agent guesses how the repository works | One entry point routes it to repository-owned instructions, commands, and decisions. |
| Changes grow beyond the request or finish without proof | Core requires a bounded change and observable evidence before completion. |
| Work spans sessions or needs independent acceptance | Durable plans preserve context; Delivery separates implementation from review and acceptance. |

## Install

Run from the project you want Truss to manage. These commands install **Core +
Delivery** from the pinned release.

### Linux and macOS

```bash
cd /path/to/project
version=truss-v0.2.12
base="https://raw.githubusercontent.com/tieutuanbao/Truss/$version"
export TRUSS_SOURCE_BASE_URL="$base" TRUSS_CORE_SOURCE_BASE_URL="$base"
export TRUSS_RELEASE_REPO=tieutuanbao/Truss
curl -fsSL "$base/scripts/install-truss.sh" \
  | bash -s -- --directory . --with-delivery --yes
```

### Windows PowerShell

```powershell
Set-Location C:\path\to\project
$version = "truss-v0.2.12"
$base = "https://raw.githubusercontent.com/tieutuanbao/Truss/$version"
$env:TRUSS_SOURCE_BASE_URL = $base
$env:TRUSS_CORE_SOURCE_BASE_URL = $base
$env:TRUSS_RELEASE_REPO = "tieutuanbao/Truss"
$script = Invoke-RestMethod "$base/scripts/install-truss.ps1"
& ([scriptblock]::Create($script)) -Directory . -WithDelivery -Yes
```

Existing instructions are protected. Preview and choose a conflict policy when
managed paths already exist; see [Installation](docs/installation.md).

## First use

### 1. Check the installation

```bash
.truss/core/bin/truss doctor
.truss/core/bin/truss status
```

Resolve reported problems before asking an agent to edit code.

### 2. Start with Core

Open your coding agent in the project root and ask:

```text
Read AGENTS.md and .truss/core/docs/WORKFLOW.md. Explain how to build, test,
and work safely in this repository. Do not change files yet. Separate documented
facts from observations and missing information.
```

### 3. Configure Delivery

Install [Orca](https://www.onorca.dev/docs/install), then ask your agent:

```text
$delivery-setup configure the delivery roles for this repository.
```

Verify Orca and start a delivered change:

```bash
orca-ide --version
orca-ide status --json
orca-ide orchestration run-list --json
```

```text
$delivery implement <outcome> under <scope and acceptance criteria>.
```

Use Delivery for work that needs approved design and independent acceptance;
use Core directly for ordinary questions and small changes. See
[Delivery](docs/delivery.md).

## Options

Core needs no flag; optional profiles are `--with-engineering-wisdom`, `--with-planning`, and `--with-delivery`.

## Documentation

- [Installation and profiles](docs/installation.md)
- [Delivery with Orca](docs/delivery.md)
- [Maintenance and updates](docs/maintenance.md)
- [Development and releases](docs/development.md)
- [Installed workflow](distribution/payload/.truss/core/docs/WORKFLOW.md)
- [Installed documentation map](distribution/payload/.truss/core/docs/README.md)

## 🙏 Acknowledgements

Truss is informed by [dely](https://github.com/hieuphung97/dely),
[repository-harness](https://github.com/hoangnb24/repository-harness), and the
[Karpathy Guidelines](https://github.com/forrestchang/andrej-karpathy-skills) by forrestchang (MIT).
