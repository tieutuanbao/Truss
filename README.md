<div align="center">

# 🔺 Truss

**Turn any software repository into a clear, durable workspace for coding agents.**

[![Release](https://img.shields.io/badge/release-v0.1.10-2563eb)](scripts/truss-release-tag)
[![License](https://img.shields.io/badge/license-MIT-16a34a)](LICENSE)
[![Language](https://img.shields.io/badge/rust-2021%20edition-f74c00?logo=rust&logoColor=white)](Cargo.toml)
[![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-64748b)](#quick-start)

Repository-owned guidance · Safe transactional updates · Optional Orca delivery

[Quick start](#quick-start) · [After installation](#after-installation) · [Daily playbook](#daily-playbook) · [Example](#example) · [Maintenance](#maintenance) · [Documentation](#documentation)

</div>

---

Truss installs a compact operating layer for coding agents while keeping the
repository as the system of record. It gives an agent a reliable entry point,
a workflow for choosing the right amount of process, durable planning when work
needs it, and evidence requirements for claiming completion.

Truss does not replace the application's README, architecture, tests, CI,
runbooks, credentials, or product decisions. Its job is to help agents find and
respect that repository-owned truth.

## ✨ What Truss gives you

| Need | Truss provides |
| --- | --- |
| Consistent agent behavior | One canonical `AGENTS.md` entry point |
| Repository context | A documentation map and work-shape contract |
| Long-running work | Active and completed execution-plan directories |
| Safer changes | Authority checks and behavior-matched proof |
| Safe maintenance | Checksums, provenance, backup, and three-way updates |
| Multi-agent delivery | Optional planning, implementation, and independent review through Orca |

<a id="quick-start"></a>

## 🚀 Quick start

### Linux and macOS

Install the core into a project:

```bash
scripts/install-truss.sh --directory /path/to/project --yes
```

Install the core together with Delivery:

```bash
scripts/install-truss.sh \
  --directory /path/to/project \
  --with-delivery \
  --yes
```

For an existing repository that already has `AGENTS.md` or `.truss-core/docs/`, preview
first and choose an explicit conflict policy:

```bash
scripts/install-truss.sh \
  --directory /path/to/project \
  --with-delivery \
  --merge \
  --dry-run \
  --yes
```

Remove `--dry-run` after reviewing the preview.

### Windows PowerShell

```powershell
scripts/install-truss.ps1 `
  -Directory C:\path\to\project `
  -WithDelivery `
  -Yes
```

### Remote bootstrap from Git

The installer shallow-clones the configured source and builds the CLI from that
checkout. A current Rust toolchain is required:

```bash
export TRUSS_SOURCE_GIT=https://github.com/tieutuanbao/Truss.git

curl -fsSL https://raw.githubusercontent.com/tieutuanbao/Truss/main/scripts/install-truss.sh \
  | bash -s -- --source-git "$TRUSS_SOURCE_GIT" --yes
```

To install a specific branch or tag, clone it explicitly and run
`scripts/install-truss.sh` from that checkout.

### Remote bootstrap from published binaries

```bash
export TRUSS_SOURCE_BASE_URL=https://raw.githubusercontent.com/tieutuanbao/Truss/main
export TRUSS_CORE_SOURCE_BASE_URL="$TRUSS_SOURCE_BASE_URL"
export TRUSS_RELEASE_REPO=tieutuanbao/Truss

curl -fsSL "$TRUSS_SOURCE_BASE_URL/scripts/install-truss.sh" \
  | bash -s -- --yes
```

Installation copies files only. It does not invoke skills, start services, or
modify global agent configuration.

<a id="after-installation"></a>

## ✅ After installation: your first 30 minutes

The most effective next step is to connect Truss to the truth already present
in the target repository. Run these steps from the installed project root.

### 1. Verify the installation

```bash
.truss-core/bin/truss doctor
.truss-core/bin/truss status
```

`doctor` checks provenance, managed paths, merge support, and transaction
health. `status` shows the installed version and whether managed files differ
from their installed baseline. Resolve reported installation problems before
asking an agent to change application code.

### 2. Start the agent at the repository root

Open your preferred coding agent in the project root and begin with this
read-only request:

```text
Read AGENTS.md and .truss-core/docs/WORKFLOW.md. Then inspect only the repository material
needed to explain:

1. where product intent and architecture are documented;
2. how to build, test, lint, and run the application;
3. which validation commands are required before completion; and
4. which important operational facts are missing or ambiguous.

Do not change files yet. Separate documented facts from observations and gaps.
```

This confirms that the agent can retrieve the installed instructions without
granting it permission to invent missing project policy.

### 3. Onboard an existing or unfamiliar repository

For a brownfield project, invoke the core onboarding skill:

```text
$onboard-repository inspect this repository, trace one real operational path,
and propose the smallest agent-facing documentation improvements. Keep the
first pass read-only. Do not install dependencies, start services, or edit files.
```

The first pass should return an evidence-backed repository map and an exact
proposal. Review that proposal before authorizing changes. For higher-risk
repositories, have a fresh agent independently inspect the proposal:

```text
$audit-onboarding-proposal audit the onboarding result and its proposed patch.
Do not apply changes.
```

Apply only the proposal items you explicitly accept. A useful approval is
specific:

```text
Apply proposal items 1 and 3 only. Preserve all existing application content,
run the repository's documentation checks, and show the final diff.
```

### 4. Fill the project-owned gaps

Truss supplies locations and templates; the project must supply its real
commands and policies. Establish these facts early:

| Project truth | Recommended owner |
| --- | --- |
| Product purpose, setup, and common commands | Project `README.md` and `.truss-core/docs/product/` |
| Current component boundaries and data flow | Architecture documentation |
| Accepted lasting technical choices | `.truss-core/docs/decisions/` |
| Build, lint, test, and release commands | Existing task runner, CI, and developer docs |
| How to run and verify the real application | A consumer-owned runbook, optionally based on `.truss-core/docs/templates/application-runbook.md` |
| Work that must survive sessions | One file under `.truss-core/docs/plans/active/` |

Do not fill unknown fields with generic commands. Ask the responsible human for
missing product policy, credentials, external ownership, or destructive
recovery decisions.

### 5. Configure Delivery once, if installed

If Truss was installed with `--with-delivery`, first configure which coding
harness, model, and effort own each role:

```text
$delivery-setup configure the delivery roles for this repository.
```

Choose the quick path to use the current harness defaults, or customize the
five roles: `plan`, `plan-review`, `implement`, `review`, and `consult`.
Delivery also requires the Orca execution plane; see
[Delivery with Orca](#delivery-with-orca).

This setup is repository configuration. Run it again only when the available
harnesses or role preferences change.

<a id="daily-playbook"></a>

## 🧭 Daily playbook

Use the smallest workflow that fits the work. The prompts below are intended to
be copied and adapted.

| Goal | Prompt | Expected behavior |
| --- | --- | --- |
| Ask or diagnose | `Explain why <behavior> occurs. Inspect only; do not change files.` | Read-only inspection and an evidence-backed answer |
| Review code | `Review <scope>. Report findings first with paths and proof. Do not edit.` | Read-only findings ranked by impact |
| Make a bounded change | `Change <specific behavior>. Preserve unrelated work, run focused and required checks, and report evidence.` | Small coherent edit with repository-owned validation |
| Continue multi-session work | `Create or resume one execution plan in .truss-core/docs/plans/active/ for <outcome>, then work from it.` | Durable progress, decisions, recovery, and validation in one plan |
| Enforce an accepted rule | `$encode-invariant enforce <documented rule> from <authority path>. Include positive and negative proof.` | Small repository-native guard without inventing policy |
| Get engineering advice | `$engineering-wisdom review <scope>. Keep advice separate from repository policy and name trade-offs.` | Contextual review based on observed code and explicit heuristics |
| Deliver with independent review | `$delivery implement <outcome> under <scope and acceptance criteria>.` | Approved design, separate implementation, exact-HEAD review, and release preparation |
| Improve agent effectiveness | `$improve-truss improve <observed recurring friction> using the recorded failed trajectory.` | One evidence-backed intervention followed by a fresh-agent rerun |

### When should work get a durable plan?

Use one file under `.truss-core/docs/plans/active/` when the work:

- spans sessions;
- coordinates multiple contributors;
- has meaningful dependencies;
- needs explicit recovery; or
- cannot safely resume from the Git diff alone.

Use `.truss-core/docs/templates/exec-plan.md`. Keep progress and task-local decisions in
that one file. Move it to `.truss-core/docs/plans/completed/` only after the outcome and its
validation are complete.

Small, clear changes do not need a durable plan. The agent should inspect the
relevant authority and proof, make the smallest coherent change, validate it,
and report the result.

<a id="example"></a>

## 🛠️ A concrete end-to-end example

Suppose a user reports that an API accepts an invalid order state.

### A normal bounded change

```text
Fix the API so it rejects the invalid order state documented in
.truss-core/docs/product/orders.md. Keep the change within the order validation boundary.
Add or update focused proof, run the repository-required checks, and report the
final behavior and any validation limits.
```

The agent should:

1. read `AGENTS.md`, `.truss-core/docs/WORKFLOW.md`, and the named product authority;
2. locate the validator and its existing tests;
3. confirm the requested behavior is unambiguous;
4. make the smallest implementation and test change;
5. run focused tests and required repository checks; and
6. report the result, changed files, evidence, and unresolved risks.

### The same change through Delivery

Use Delivery when the contract is public, the change is architectural, or the
implementer should not review its own work:

```text
$delivery change the order API to reject the invalid state documented in
.truss-core/docs/product/orders.md. The public error contract must remain compatible, the
focused test must distinguish the old and new behavior, and an independent
reviewer must approve the exact final commit.
```

Delivery should produce an approved design contract, dispatch implementation
to a separate worker, run repository proof, obtain an independent review bound
to the exact final revision, and prepare only the release action authorized by
the user.

## 📦 Installation profiles

| Profile | Flag | Contents | Use it when |
| --- | --- | --- | --- |
| Core | default | Agent instructions, workflow, documentation structure, planning templates, onboarding, invariant, and improvement skills | Every repository |
| Engineering Wisdom | `--with-engineering-wisdom` | Advisory heuristics and engineering references | You want an explicitly requested, repository-grounded engineering review |
| Delivery | `--with-delivery` | Role configuration, design approval, implementation, independent review, and release coordination through Orca | A change needs supervised separation of responsibilities |

Add-ons are explicit and independent. Installation alone does not activate a
skill, and omitting an add-on flag does not remove an existing copy.

### ⚙️ Installation behavior

| Bash option | PowerShell option | Behavior |
| --- | --- | --- |
| `--dry-run` | `-DryRun` | Preview changes without writing files |
| `--merge` | `-Merge` | Keep existing protected files and install missing managed paths |
| `--override` | `-Override` | Back up and replace protected Truss paths |
| `--force` | `-Force` | Back up and overwrite individual managed files |
| `--refresh-agent-shim` | `-RefreshAgentShim` | Refresh the managed Truss block in `AGENTS.md` |
| `--claude` | — | Install or refresh the `CLAUDE.md` import shim using the Bash installer |

When protected files already exist, non-interactive installation stops unless
an explicit conflict policy such as `--merge` or `--override` is supplied.

## 🌳 What gets installed?

```text
project/
├── AGENTS.md
├── .agents/
│   └── skills/
│       ├── onboard-repository/
│       ├── audit-onboarding-proposal/
│       ├── encode-invariant/
│       └── improve-truss/
└── .truss-core/
    ├── docs/
    │   ├── WORKFLOW.md
    │   ├── README.md
    │   ├── decisions/
    │   ├── patterns/
    │   ├── plans/
    │   │   ├── active/
    │   │   └── completed/
    │   ├── product/
    │   └── templates/
    ├── bin/
    │   └── truss
    ├── manifest.json
    └── base/
```

Your source code, tests, CI, and any existing `docs/` or `scripts/` folders
stay exactly where they are. Everything Truss installs lives under
`.truss-core/`, the root `AGENTS.md` entry point, and `.agents/skills/` —
the standard location agent tools scan for skill discovery. To keep Truss
local-only, ignore those paths in `.gitignore` or `.git/info/exclude`.

Optional profiles add their own skills under `.agents/skills/`. Exact payloads
are declared by the manifests in [`scripts/`](#scripts-reference).

<a id="maintenance"></a>

## 🔁 Maintenance

Use this sequence when checking or updating an installed Truss core:

```bash
# 1. Inspect local modifications to managed files.
.truss-core/bin/truss status

# 2. Validate installation and transaction health.
.truss-core/bin/truss doctor

# 3. Preview the incoming update.
.truss-core/bin/truss update --dry-run

# 4. Apply the verified update.
.truss-core/bin/truss update
```

Updates verify release checksums, preserve consumer edits, and use three-way
merging for managed text files. If local and incoming edits overlap, Truss
stages an explicit resolution session. Edit only the conflict copies under:

```text
.truss-core/update/resolved/
```

Then continue the same update:

```bash
.truss-core/bin/truss update --continue
```

Do not delete `.truss-core/`: it stores the baseline and provenance needed for
safe updates.

<a id="delivery-with-orca"></a>

## 🤝 Delivery with Orca

The Delivery add-on uses Orca as its required execution plane. The current
session remains in control of approvals and release authority while Orca runs
separate planning, consultation, implementation, and review workers.

On Linux:

1. Install and launch the Orca AppImage from
   [onorca.dev/.truss-core/docs/install](https://www.onorca.dev/.truss-core/docs/install).
2. Register the CLI under **Settings → Experimental → CLI**.
3. Enable **Settings → Experimental → Orchestration**.
4. Verify the execution plane:

```bash
orca-ide --version
orca-ide status --json
orca-ide orchestration run-list --json
```

The registered CLI is normally `orca-ide`. On Linux, `orca` commonly refers to
the GNOME screen reader and is not the execution-plane command.

<a id="documentation"></a>

## 📚 Documentation

| Document | Purpose |
| --- | --- |
| [Product model](.truss-core/docs/TRUSS.md) | Responsibilities, boundaries, profiles, and evidence model |
| [Architecture](.truss-core/docs/ARCHITECTURE.md) | Rust layers, installation state, transactions, and trust boundaries |
| [Repository workflow](.truss-core/docs/WORKFLOW.md) | Work shapes, task flows, validation, and completion standards |
| [Documentation map](.truss-core/docs/README.md) | Entry point to product, decisions, plans, patterns, and templates |
| [Installation contract](.truss-core/docs/product/installation-profiles.md) | Exact profile, conflict, update, and release-source behavior |
| [Encoding invariants](.truss-core/docs/patterns/encoding-invariants.md) | Turning accepted rules into mechanical validation |

## 🧪 Development

Run the complete local validation entry point:

```bash
scripts/validate-premerge.sh
```

It checks shell syntax, Rust formatting, tests, Clippy warnings, the source
contract, and the current Git diff. Development requires a current stable Rust
toolchain, Git, and ripgrep.

<a id="scripts-reference"></a>

## 📜 Scripts reference

| File | Purpose |
| --- | --- |
| `install-truss.sh` | Linux/macOS local and remote bootstrap entry point |
| `install-truss.ps1` | Windows PowerShell bootstrap entry point |
| `validate-premerge.sh` | Complete source validation entry point |
| `truss-install-files.txt` | Core installation manifest embedded by the Rust binary |
| `engineering-wisdom-install-files.txt` | Engineering Wisdom add-on manifest |
| `delivery-install-files.txt` | Delivery add-on manifest |
| `agent-truss-block.md` | Managed instruction block installed into `AGENTS.md` |
| `claude-truss-block.md` | `CLAUDE.md` import block used by the Bash installer |
| `truss-release-tag` | Version pointer consumed by bootstrap and self-update |

When the core payload changes, keep `truss-install-files.txt` synchronized with
the embedded Rust distribution.

---

<div align="center">

**Start with repository truth. Use only the process the work actually needs.**

</div>
