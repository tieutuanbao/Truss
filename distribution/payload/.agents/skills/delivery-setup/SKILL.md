---
name: delivery-setup
description: Configure a project's AGENTS.md with one managed delivery block — per-role truss, model and effort for the six dispatched roles (ba, architect, planner, implement, visual-engineering, tester-debugger) plus the current project-manager row, discovered from the live truss surface. Use at the start of a Control Session, when the project has no managed block, or when those pins need rewriting from the installed trusses. Not for installing skills, trusting hooks, or delivering a change; that is delivery.
---

# Delivery Setup

Write exactly one managed block into the project's `AGENTS.md`. Discover
models and effort from the installed trusses. Do not store a catalogue. Do
not install, trust, or enumerate anything.

Read `AGENTS.md` first. Replace only the region between this skill's own
markers. Prose outside the block is not read, merged, moved, or deleted.

## Seven roles

The managed block configures delivery preferences for exactly seven roles —
`project-manager` is the current interactive session and is never dispatched;
the other six are dispatched per task:

| Role | Owns | Dispatched |
| --- | --- | --- |
| `project-manager` | Project coordination, required dev/ops preflight, approvals, dispatch, integration, recovery and authorized release | no — the current interactive session |
| `ba` | The concrete business analysis: goals, actors, flows, business rules, exceptions, scope, stable requirement IDs, acceptance and planner handoff | yes |
| `architect` | Technical contracts, interfaces, boundaries, dependencies, risks, and the decision record mapping choices to business requirements | yes |
| `planner` | The detailed task plan: inputs, outputs, exact path ownership, dependency DAG, commands, waves, integration and recovery | yes |
| `implement` | Task-scoped code and documentation, tests, evidence, and commits | yes |
| `visual-engineering` | UI/UX/visual advice and authorized UI implementation, including relevant code, docs, and tests | yes, when the task needs it |
| `tester-debugger` | Testing and diagnosis, authorized fixes, or independent acceptance in a separate fresh read-only session | yes |

There is no separate coordinator or orchestrator field, because Orca is the
constant, required execution plane and the `project-manager` row already
records the current interactive session, which is never dispatched and always
owns the two human gates. There is no release row, because release has no LLM
worker and stays with `project-manager`. One session holds one role per run: a
planner never implements or accepts, an implementer never accepts its own
candidate, and code review is an activity inside acceptance, never an eighth
role.

## Two paths

**Quick.** Use the current truss for every dispatched role. Use that truss's
defaults for model and effort, written as the literal `default`.

**Customize.** For each dispatched role, offer the discovered trusses, models
and effort levels and write what the human chooses. `project-manager` is not
offered a truss, model, or effort: it is always written as the literal
`current` in all three cells.

Ask which path. Do not start writing until that is answered.

## What to write

Exactly one block, and nothing else:

```markdown
<!-- delivery:begin -->
## Delivery

Architectural work and explicitly requested deliveries invoke
`$delivery`; ordinary bounded work does not. Spike starts no delivery run.

Seven roles share the run. `project-manager` is the current interactive
session and is never dispatched; the other six rows are dispatched with these
preferences.

| Role | Truss | Model | Effort |
| --- | --- | --- | --- |
| `project-manager` | current | current | current |
| `ba` | … | … | … |
| `architect` | … | … | … |
| `planner` | … | … | … |
| `implement` | … | … | … |
| `visual-engineering` | … | … | … |
| `tester-debugger` | … | … | … |
<!-- delivery:end -->
```

If the markers already exist, replace the region between them. If they do
not, append the block. Touch nothing else.

## Discovery

Run these commands. They are the catalogue. Do not copy a list from this
package, from memory, or from `.truss-core/docs/`.

- Claude Code models: `claude -p "/model" --output-format json`
- Claude Code effort: `claude -p "/effort" --output-format json`
- Codex: `codex debug models`
- Grok models: `grok models`
- Antigravity CLI models: `agy models` (offer the slug column of its TSV output)
- Pi models: `pi --list-models` (offer the `provider/model` pair)
- Zcode: run the resolved standalone terminal client's `version` and
  `doctor --json`. The doctor must identify process name `zcode-cli`; final
  usability is proved by a clean interactive TUI readiness check at dispatch.
  Its current CLI has no non-interactive model catalogue, so write literal
  `default` for Model and Effort rather than inspecting credential or provider
  configuration.

The two Claude probes are answered locally: `num_turns` 0, `total_cost_usd` 0,
no model turn. Do not treat them as a dispatch.

Codex slugs with `visibility: hide` are not offered. Codex reasoning levels
are `supported_reasoning_levels` on each slug, not one vocabulary per truss.

Grok effort is not discovered by calling the model. Read the installed CLI's
help and any validation already observed. Do not run a Grok prompt to learn
the flag.

Antigravity CLI effort is `low|medium|high`, read from `agy --help`'s
`--effort` flag; do not prompt the model to learn it. Some model slugs already
end in `-high`, `-medium`, or `-low` — that suffix names the model, not the
effort flag, so do not strip it.

Pi effort is `off|minimal|low|medium|high|xhigh|max`, read from `pi --help`'s
`--thinking` flag. Pi may expose router aliases instead of vendor model names;
record the discovered `provider/model` pair exactly. The literal `default`
omits both `--model` and `--thinking`.

The desktop `zcode` launcher is not the terminal CLI. Prefer a configured
standalone terminal-client command; do not accept the Electron launcher or its
bundled `resources/glm/zcode.cjs`, because a desktop bundle may omit the TUI
package even when `version` and `doctor --json` pass. Omit Zcode when no
standalone candidate passes those probes. Zcode's role row uses `default` for
Model and Effort, and delivery performs the final interactive readiness check.

A truss that is not installed is omitted from the offer, not an error.

### Kiro CLI

Kiro CLI models: `kiro-cli chat --list-models --format json` (offer each
`model_id`). Kiro CLI effort is read from `kiro-cli chat --help`'s `--effort`
flag; do not prompt the model to learn it and do not store a catalogue. Live
discovery may offer only `auto` — that is a valid result, not a reason to
invent model names. Omit Kiro discovery that is unavailable or unusable
rather than guessing.

### Cursor Agent CLI

Cursor Agent CLI models: `cursor-agent models` (offer the slug before ` - `).
There is no `--effort` flag: write the literal `default` for Effort. Do not
invent an effort vocabulary, do not strip effort suffixes from slugs, and do
not synthesize parameterized `[effort=…]` forms. Omit Cursor discovery that
is unavailable or unusable rather than guessing.

### GitHub Copilot CLI

GitHub Copilot CLI models: there is no non-interactive listing. Write
the literal `default` for Model; do not invent a catalogue, do not prompt the
model to learn one, and do not treat `copilot -p "/model"` as
discovery. GitHub Copilot CLI effort is read from `copilot --help`'s
`--effort` flag. Omit Copilot discovery that is unavailable or unusable
rather than guessing.

## Migrating a legacy block

A block that still names the retired five-role set is migrated in place, never
replaced wholesale. The retired roles were `plan`, `plan-review`, `implement`,
`review`, and `consult`; they are not delivered roles and must not remain in
the rewritten block.

1. Validate marker integrity first, as under § Refusals. A broken block is
   reported, not repaired.
2. Recognize the exact five-role block. Carry each legacy row's chosen Truss,
   Model, and Effort onto its migrated role; do not reset a custom pin to the
   truss default.
3. Map with the table below. `project-manager` has no legacy row: write
   `current` in all three cells.
4. Replace only the managed block. Unrelated prose outside it stays
   byte-identical, and the procedure is stable when run again.

| Retired role | Migrated to | Pins |
| --- | --- | --- |
| `plan` | `planner` | preserve that row's truss, model and effort |
| `plan-review` | `architect` | preserve that row's truss, model and effort |
| `implement` | `implement` and `visual-engineering` | preserve that row's truss, model and effort for both |
| `review` | `tester-debugger` | preserve that row's truss, model and effort |
| `consult` | `ba` | preserve that row's truss, model and effort |
| (no legacy row) | `project-manager` | write `current` in all three cells |

A block that already has the seven roles is preserved unchanged unless the
human explicitly asks to reconfigure it. A block with missing, mixed, or
ambiguous legacy rows is reported for focused resolution; do not guess a
mapping, silently drop a row, or lose a custom pin. No human question is
needed for the known five-role mapping above when the block is complete.

## Pinning

Where a managed block exists, pin each dispatched role's Truss, Model and
Effort on every dispatch of that role. `project-manager` is never dispatched
and its `current` cells are representational, never a model lookup or a launch
argument. Before dispatch, verify that the selected agent is registered in the
live Orca agent surface and that its requested model and effort are supported.
A receipt showing Requested and Effective values proves only preference
resolution; it does not prove agent readiness or that the provider accepted the
model. The literal `default` in Model or Effort means the truss default is
wanted: omit that flag. That is not the same as an unset cell — defaults are a
deployment preference, not a reproducible pin, and the execution envelope
records the configured value and, where the truss exposes it, the actual
observed model and effort.

Where `AGENTS.md` carries no managed block, or a dispatched row is missing,
`delivery` falls back per role: every dispatched role runs on the current
truss with that truss's defaults, and `project-manager` remains the current
session with no launch fallback. Missing BA, architect, or planner pins never
move those duties into `project-manager`. Delivery setup is a convenience over
those fallbacks, not a precondition for them.

## When a choice cannot be offered

Where a choice cannot be offered, do not make it. Take the conservative
action — which may be writing a conservative value, and may be doing
nothing — and report the choice that was not offered, naming what was
available and how to set it; do not write that report into the managed
block.

For the `CLAUDE.md` import: if the current truss is Claude Code, the import
is absent, and delivery setup cannot ask, write nothing and report the offer
that was not made.

## Refusals

Stop and report to the human, unchanged, when:

- the markers are broken (a `begin` without a matching `end`, or an `end`
  before its `begin`)
- more than one `<!-- delivery:begin -->` is present
- a legacy phase table outside the block contradicts the block
- the block's roles are neither the known five-role set nor the seven-role set

Do not merge two tables, delete a legacy table, or guess which is
authoritative.

## Claude Code and `AGENTS.md`

Claude Code does not read `AGENTS.md`. The persistent instruction reaches
Codex, Grok, Antigravity CLI, Kiro CLI, Cursor Agent CLI, and GitHub
Copilot CLI natively. It reaches Claude Code only where the project has a
`CLAUDE.md` that imports `AGENTS.md`.

Where the current truss is Claude Code and the project has no `CLAUDE.md`
importing `AGENTS.md`, offer to create a one-line `CLAUDE.md` containing
`@AGENTS.md`. The human accepts or declines. Never write it unasked.

This is not a second managed block: no markers, no configuration, a pointer
at the block rather than a copy of it.

The offer is Claude-Code-only. On Codex, Antigravity CLI, and Kiro CLI the
file is inert in the same sense that it is not imported. Cursor Agent CLI
applies `CLAUDE.md` as a rule. GitHub Copilot CLI loads both `AGENTS.md`
and `CLAUDE.md`. Grok does not expand it.

## What delivery setup will not do

No skill or plugin install. No hook trust. No writes to `~/.claude`,
`~/.codex`, `~/.grok`, `~/.gemini`, `~/.pi`, `~/.zcode`, `~/.kiro`,
`~/.cursor`, or `~/.copilot`.
No custom Kiro agent creation or modification. No coordinator installation
or field. No separate control or release row. No enumeration or invocation of
project-owned workflow skills. No model catalogue.

Print verified install guidance only when the human explicitly asks for it.
