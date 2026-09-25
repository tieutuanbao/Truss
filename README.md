<div align="center">

# 🔺 Truss

**Turn any software repository into a clear, durable workspace for coding agents.**

[![Release](https://img.shields.io/badge/release-v0.1.16-2563eb)](scripts/truss-release-tag)
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
| Multi-agent delivery | Optional seven-role delivery with independent acceptance through Orca |

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

For an existing repository that already has `AGENTS.md` or `.truss/core/docs/`, preview
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
.truss/core/bin/truss doctor
.truss/core/bin/truss status
```

`doctor` checks provenance, managed paths, merge support, and transaction
health. `status` shows the installed version and whether managed files differ
from their installed baseline. Resolve reported installation problems before
asking an agent to change application code.

### 2. Start the agent at the repository root

Open your preferred coding agent in the project root and begin with this
read-only request:

```text
Read AGENTS.md and .truss/core/docs/WORKFLOW.md. Then inspect only the repository material
needed to explain:

1. where product intent and architecture are documented;
2. how to build, test, lint, and run the application;
3. which validation commands are required before completion; and
4. which important operational facts are missing or ambiguous.

Do not change files yet. Separate documented facts from observations and gaps.
```

This confirms that the agent can retrieve the installed instructions without
granting it permission to invent missing project policy.

### Optional: choose how agents explain their work

No additional installation flag is required.

Ask your agent:

```text
Help me configure how agents explain their work in this repository.
Show expert, intermediate, and layperson styles using the same example.
Wait for my choice before saving the repository default.
```

The styles range from technical terminology with minimal background
(`expert`), through terms explained on first use (`intermediate`), to plain
language and explicit steps (`layperson`). You choose; the agent records
your choice in
[`.truss/core/docs/communication.md`](.truss/core/docs/communication.md).

Requests such as "explain this more simply" override the default for the
conversation without changing the saved setting. The guidance also asks
agents to suggest a different level when requests for re-explanation recur,
and to wait for approval before saving that change. Simpler explanations
must still preserve risks, uncertainty, and exact commands.

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
| Product purpose, setup, and common commands | Project `README.md` and `.truss/core/docs/product/` |
| Current component boundaries and data flow | Architecture documentation |
| Accepted lasting technical choices | `.truss/core/docs/decisions/` |
| Build, lint, test, and release commands | Existing task runner, CI, and developer docs |
| How to run and verify the real application | A consumer-owned runbook, optionally based on `.truss/core/docs/templates/application-runbook.md` |
| Work that must survive sessions | One file under `.truss/core/docs/plans/active/` |

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
six dispatched roles: `ba`, `architect`, `planner`, `implement`,
`visual-engineering`, and `tester-debugger`. `project-manager` is the current
interactive session and is never dispatched. Delivery also requires the Orca
execution plane; see [Delivery with Orca](#delivery-with-orca).

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
| Continue multi-session work | `Create or resume one execution plan in .truss/core/docs/plans/active/ for <outcome>, then work from it.` | Durable progress, decisions, recovery, and validation in one plan |
| Enforce an accepted rule | `$encode-invariant enforce <documented rule> from <authority path>. Include positive and negative proof.` | Small repository-native guard without inventing policy |
| Get engineering advice | `$engineering-wisdom review <scope>. Keep advice separate from repository policy and name trade-offs.` | Contextual review based on observed code and explicit heuristics |
| Deliver with independent acceptance | `$delivery implement <outcome> under <scope and acceptance criteria>.` | Approved design, separate implementation, exact-HEAD tester-debugger acceptance, and release preparation |
| Improve agent effectiveness | `$improve-truss improve <observed recurring friction> using the recorded failed trajectory.` | One evidence-backed intervention followed by a fresh-agent rerun |

### When should work get a durable plan?

Use one file under `.truss/core/docs/plans/active/` when the work:

- spans sessions;
- coordinates multiple contributors;
- has meaningful dependencies;
- needs explicit recovery; or
- cannot safely resume from the Git diff alone.

Use `.truss/core/docs/templates/exec-plan.md`. Keep progress and task-local decisions in
that one file. Move it to `.truss/core/docs/plans/completed/` only after the outcome and its
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
.truss/core/docs/product/orders.md. Keep the change within the order validation boundary.
Add or update focused proof, run the repository-required checks, and report the
final behavior and any validation limits.
```

The agent should:

1. read `AGENTS.md`, `.truss/core/docs/WORKFLOW.md`, and the named product authority;
2. locate the validator and its existing tests;
3. confirm the requested behavior is unambiguous;
4. make the smallest implementation and test change;
5. run focused tests and required repository checks; and
6. report the result, changed files, evidence, and unresolved risks.

### The same change through Delivery

Use Delivery when the contract is public, the change is architectural, or the
implementer should not accept its own work:

```text
$delivery change the order API to reject the invalid state documented in
.truss/core/docs/product/orders.md. The public error contract must remain compatible, the
focused test must distinguish the old and new behavior, and an independent
tester-debugger session must accept the exact final commit.
```

Delivery should produce an approved design contract, dispatch BA, architecture,
and planning, then an isolated implementation to separate workers, run
repository proof, obtain an independent tester-debugger acceptance bound to the
exact final revision, and prepare only the release action authorized by the
user.

## 📦 Installation profiles

| Profile | Flag | Contents | Use it when |
| --- | --- | --- | --- |
| Core | default | Agent instructions, workflow, documentation structure, planning templates, onboarding, invariant, and improvement skills | Every repository |
| Engineering Wisdom | `--with-engineering-wisdom` | Advisory heuristics and engineering references | You want an explicitly requested, repository-grounded engineering review |
| Planning | `--with-planning` | Explicit-only skill that turns an idea into user-approved product authority and a first implementation slice | A new project starts from an idea rather than an existing body of product truth |
| Delivery | `--with-delivery` | Role configuration, design approval, isolated implementation, independent tester-debugger acceptance, and release coordination through Orca | A change needs supervised separation of responsibilities |

Add-ons are explicit and independent. Installation alone does not activate a
skill, and omitting an add-on flag does not remove an existing copy.

### 🔁 Add-on lifecycle

Every add-on is one distribution with its own immutable provenance. For the
add-ons you request with `--with-*`, the installer acquires the payload and
delegates every write to the Truss CLI, which owns the plan, the baseline, and
the record. The same lifecycle is available directly:

```bash
.truss/core/bin/truss addon status   --name delivery --directory /path/to/project
.truss/core/bin/truss addon install  --name delivery --directory /path/to/project \
  --manifest scripts/delivery-install-files.txt \
  --source <staged-payload-dir> --source-ref truss-v0.1.13
.truss/core/bin/truss addon update   --name delivery --directory /path/to/project \
  --manifest scripts/delivery-install-files.txt \
  --source <staged-payload-dir> --source-ref truss-v0.1.14
.truss/core/bin/truss addon continue --name delivery --directory /path/to/project
.truss/core/bin/truss addon abort    --name delivery --directory /path/to/project
```

- `status` reports the recorded `source_ref`, the core version the payload was
  acquired with, and the path plus SHA-256 of every managed file. It exits `1`
  when the add-on is not recorded. Every subcommand accepts `--json`.
- `install` and `update` are the only writers. Both accept `--dry-run`, which
  reports the plan and changes nothing.
- `--source-ref` is an immutable ref: a `truss-vX.Y.Z` release tag or an exact
  40/64-character commit SHA. A branch name, `HEAD`, or a short SHA is refused
  before any state is opened.
- The installer refuses a requested add-on when it cannot resolve such a ref,
  when a manifest-listed payload path is not committed at `HEAD`, or when the
  payload bytes differ from `HEAD`.

Overlapping local and upstream edits are never overwritten. `addon update`
stops with exit `2` and stages the frozen plan under
`.truss/core/addon-update/<name>/resolved/<path>`. Edit those copies, then apply
the frozen decision with `addon continue`; `addon abort` removes only the owned
session and leaves managed files unchanged.

The record lives in `.truss/core/addons.json` and the payload baseline in
`.truss/core/base-addons/<name>/`; both are owned by the CLI. `AGENTS.md` is a
core payload file, never an add-on payload file: the CLI refuses an add-on that
declares a path the core manifest or another recorded add-on already owns.

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
└── .truss/
    ├── core/                 # installed payload and CLI-owned state
    │   ├── docs/
    │   │   ├── WORKFLOW.md
    │   │   ├── README.md
    │   │   ├── decisions/
    │   │   ├── patterns/
    │   │   ├── plans/
    │   │   │   ├── active/
    │   │   │   └── completed/
    │   │   ├── product/
    │   │   └── templates/
    │   ├── bin/
    │   │   └── truss
    │   ├── manifest.json
    │   └── base/
    ├── delivery/             # local-only: per-run working memory
    │   ├── runs/<run-key>/
    │   └── approvals/<run-key>.md
    └── authority/            # this project's own authority (local-only)
        ├── architecture/
        └── decisions/
```

Your source code, tests, CI, and any existing `docs/` or `scripts/` folders
stay exactly where they are. Everything Truss installs lives under
`.truss/core/`, the root `AGENTS.md` entry point, and `.agents/skills/` —
the standard location agent tools scan for skill discovery. To keep Truss
local-only, ignore those paths in `.gitignore` or `.git/info/exclude`.

`.truss/` holds three namespaces with three owners. `core/` is the installed
payload and the installation state the CLI owns; the CLI never reads or writes
the other two. `delivery/` holds one delivery run's working memory — its plan,
its approved envelope, its dispatch artifacts, and its approval receipt — and is
written by the delivery skill, never committed. `authority/` holds that
project's own durable authority — architecture notes, decision records, plans,
and product documents — and is never committed either. A consumer that keeps
Truss in version control commits `.truss/core/` and ignores `.truss/delivery/`
and `.truss/authority/`; a consumer running Truss local-only ignores `.truss/`
with a single rule.

Optional profiles add their own skills under `.agents/skills/`. Exact payloads
are declared by the manifests in [`scripts/`](#scripts-reference).

<a id="maintenance"></a>

## 🔁 Maintenance

Use this sequence when checking or updating an installed Truss core:

```bash
# 1. Inspect local modifications to managed files.
.truss/core/bin/truss status

# 2. Validate installation and transaction health.
.truss/core/bin/truss doctor

# 3. Preview the incoming update.
.truss/core/bin/truss update --dry-run

# 4. Apply the verified update.
.truss/core/bin/truss update
```

Updates verify release checksums, preserve consumer edits, and use three-way
merging for managed text files. If local and incoming edits overlap, Truss
stages an explicit resolution session. Edit only the conflict copies under:

```text
.truss/core/update/resolved/
```

Then continue the same update:

```bash
.truss/core/bin/truss update --continue
```

Do not delete `.truss/core/`: it stores the baseline and provenance needed for
safe updates.

<a id="delivery-with-orca"></a>

## 🤝 Delivery with Orca

The Delivery add-on uses Orca as its required execution plane. The current
session is the `project-manager`: it keeps approvals and release authority
while Orca runs separate BA, architecture, planning, implementation, and
tester-debugger workers.

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
| [Product model](.truss-core/docs/TRUSS.md) | Responsibilities, boundaries, profiles, and evidence model. Source-authority document: present in a Truss source checkout, not installed into a consumer. |
| [Architecture](.truss-core/docs/ARCHITECTURE.md) | Rust layers, installation state, transactions, and trust boundaries. Source-authority document: present in a Truss source checkout, not installed into a consumer. |
| [Repository workflow](.truss/core/docs/WORKFLOW.md) | Work shapes, task flows, validation, and completion standards |
| [Documentation map](.truss/core/docs/README.md) | Entry point to product, decisions, plans, patterns, and templates |
| [Installation contract](.truss-core/docs/product/installation-profiles.md) | Exact profile, conflict, update, and release-source behavior. Source-authority document: present in a Truss source checkout, not installed into a consumer. |
| [Encoding invariants](.truss/core/docs/patterns/encoding-invariants.md) | Turning accepted rules into mechanical validation |

## 🧪 Development

Run the complete local validation entry point:

```bash
scripts/validate-premerge.sh
```

It checks shell syntax, Rust formatting, tests, Clippy warnings, the source
contract, and the current Git diff. Development requires a current stable Rust
toolchain, Git, and ripgrep.

The committed installer rehearsal is the validation entry point for the add-on
installation path:

```bash
bash tests/s5-rehearse.sh
```

It builds the CLI when it is missing, then installs into throwaway workspaces
and asserts that both installers delegate to the CLI; that all three add-ons
install against their own manifest with a recorded ref and per-file digests;
that a repeated run preserves managed bytes instead of skipping them; that a
consumer edit plus a changed payload stages a conflict with the consumer bytes
intact; that a dirty payload, a non-git source, a floating raw base URL, and a
tag mismatch each stop with a clear message and copy nothing; and that the
PowerShell script delegates with the same flags and never falls back to a
direct copy. It needs no network and exits non-zero on any failed check.
`scripts/validate-premerge.sh` runs it.

### 🚀 Release

A release is proven twice. Before tagging, the current proof is the committed
rehearsal above. After tagging `truss-vX.Y.Z`, the released source at that tag
must reach the same add-on tree and the same `addons.json` provenance as the
fresh source at the same ref. Run both and compare:

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
a release gate, not a current proof; the local rehearsal above is the current
proof.

Two limits are stated rather than implied. Windows parity is static
verification on the host without `pwsh`: the rehearsal compares the PowerShell
and Bash argument construction statically, declares the execution gap, and
claims no byte-level cross-platform equality. A dirty add-on payload is refused
rather than recorded: a checkout whose manifest-listed payload paths are
untracked or differ from `HEAD` stops before any mutation, and no dirty
provenance format is invented.

<a id="scripts-reference"></a>

## 📜 Scripts reference

| File | Purpose |
| --- | --- |
| `install-truss.sh` | Linux/macOS local and remote bootstrap entry point |
| `install-truss.ps1` | Windows PowerShell bootstrap entry point |
| `validate-premerge.sh` | Complete source validation entry point |
| `truss-install-files.txt` | Core installation manifest embedded by the Rust binary |
| `engineering-wisdom-install-files.txt` | Engineering Wisdom add-on manifest |
| `plan-install-files.txt` | Planning add-on manifest |
| `delivery-install-files.txt` | Delivery add-on manifest |
| `agent-truss-block.md` | Managed instruction block installed into `AGENTS.md` |
| `claude-truss-block.md` | `CLAUDE.md` import block used by the Bash installer |
| `truss-release-tag` | Version pointer consumed by bootstrap and self-update |

When the core payload changes, keep `truss-install-files.txt` synchronized with
the embedded Rust distribution.

<a id="acknowledgements"></a>

## 🙏 Acknowledgements

Truss takes inspiration from two earlier projects by friends and colleagues:

- [dely](https://github.com/hieuphung97/dely) — Orca-based delivery
  orchestration for coding agents, which shaped the optional Delivery add-on.
- [repository-harness](https://github.com/hoangnb24/repository-harness) — a
  repository protocol and safe updater built around the repository as the
  system of record, the foundation of the Truss core.

---

<div align="center">

**Start with repository truth. Use only the process the work actually needs.**

</div>
