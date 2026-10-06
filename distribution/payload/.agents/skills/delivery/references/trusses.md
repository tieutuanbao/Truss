# Truss launch mechanics

Use [Execution](execution.md) § CLI identity and preflight for the canonical CLI
commands, consumer Git-root and HEAD checks, binary-identity boundary, and
no-fallback rule.

For installation setup, register the CLI in Orca desktop under
**Settings → Experimental → CLI** and enable **Orchestration**.

Before dispatch, confirm that the Orca Run/worktree selector resolves to the
validated consumer root. Record the exact worktree path and baseline
SHA. Released terminals do not prove child worktrees were removed; follow
[Release and recovery](release-and-recovery.md) § Failure and recovery before cleanup or another Run.

Read only the launch reference for each resolved truss used in this run.
Shared readiness, permission-posture, dispatch completion, and retry rules
remain in [Execution](execution.md). Every dispatch, wait, acknowledgement, and recovery
command is copied from [`references/command-recipes.md`](command-recipes.md);
argv is never composed from memory or from live help alone.

## Truss normalization

Normalize the managed block's Truss cell through this table before any
dispatch. A cell that matches no row stops the dispatch; there is no fuzzy
match. Launch classification is stated by each reference and picks the recipe:
`native` dispatches through `worker-start --agent` with the model pin;
`composed` creates the pinned terminal from the reference's exact argv and
dispatches with `--terminal` only; `unresolved` stops with
`UNSUPPORTED_LAUNCH`.

| Truss (managed-block name) | Orca agent ID | CLI executable | Launch classification |
| --- | --- | --- | --- |
| Claude Code | `claude` | `claude` | native |
| Codex | `codex` | `codex` | native |
| Codex CLI | `codex` | `codex` | native |
| Cursor Agent CLI | `cursor` | `cursor-agent` | native |
| Antigravity CLI | `antigravity` | `agy` | native (composed fallback) |
| Pi | `pi` | `pi` | composed |
| OpenCode | `opencode` | `opencode` | composed |
| Zcode | `zcode` | resolved standalone client | native when advertised; else low-level |
| Grok Build | `grok` | `grok` | unresolved |
| Kiro CLI | `kiro` | `kiro-cli` | unresolved |
| GitHub Copilot CLI | `copilot` | `copilot` | unresolved |

Native support is read literally: only an agent the live
`worker-start --help` names in its native `--model` support sentence takes a
launch-time model pin. Presence among the `--agent` examples is not model-pin
support.

| Truss | Launch reference |
| --- | --- |
| Claude Code | [Claude Code](trusses/claude.md) |
| Codex | [Codex CLI](trusses/codex.md) |
| Codex CLI | [Codex CLI](trusses/codex.md) |
| Grok Build | [Grok Build](trusses/grok.md) |
| Kiro CLI | [Kiro CLI](trusses/kiro.md) |
| Antigravity CLI | [Antigravity CLI](trusses/antigravity.md) |
| Pi | [Pi](trusses/pi.md) |
| Zcode CLI | [Zcode CLI](trusses/zcode.md) |
| OpenCode | [OpenCode](trusses/opencode.md) |
| Cursor Agent CLI | [Cursor Agent CLI](trusses/cursor.md) |
| GitHub Copilot CLI | [GitHub Copilot CLI](trusses/copilot.md) |
