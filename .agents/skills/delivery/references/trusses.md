# Truss launch mechanics

## Orca CLI preflight

Use the official Orca execution-plane CLI. On Linux it is normally
`orca-ide`, not `orca`; the latter commonly resolves to the GNOME Orca screen
reader. Register the CLI in Orca desktop under **Settings → Experimental →
CLI**, enable **Orchestration**, then verify:

```bash
DELIVERY_ORCA_CLI="${DELIVERY_ORCA_CLI:-orca-ide}"
command -v "$DELIVERY_ORCA_CLI"
"$DELIVERY_ORCA_CLI" --version
"$DELIVERY_ORCA_CLI" status --json
"$DELIVERY_ORCA_CLI" orchestration run-list --json
```

Do not treat the GNOME screen reader as the execution plane. Do not create an
`orca` alias or use a direct/headless fallback when `orca-ide` is unavailable.

Before dispatch, verify the requested consumer Git root has a valid `HEAD` and
that the Orca Run/worktree selector resolves to that same root. A released
terminal does not prove its child worktree was removed. Record the exact
worktree path and baseline SHA before proceeding. A worker startup label such
as `codex-trust-workspace` is not vendor evidence; inspect the terminal and
trust the agent actually shown there in the selected worktree.

Per-truss launch mechanics for `$delivery`'s worker dispatch: each
truss's Orca agent id, permission default, forbidden headless invocation
forms, and launch notes. `worker-start --agent` resolves these ids; the
binary names `agy`, `kiro-cli` and `cursor-agent` return `agent_unconfigured`
and create no terminal. `worker-start --model` supports Claude, Codex and
Cursor ids; `--effort` requires `--model`; neither combines with `--terminal`.

| Truss | Orca agent id | Permission default | Forbidden headless forms | Launch notes |
| --- | --- | --- | --- | --- |
| Claude Code | `claude` | `--dangerously-skip-permissions` | `claude -p` | `--model` pin honoured |
| Codex CLI | `codex` | `--dangerously-bypass-approvals-and-sandbox` | `codex exec` | |
| Grok Build | `grok` | `--permission-mode bypassPermissions` | `grok --prompt-file` | |
| Antigravity CLI | `antigravity` | `--dangerously-skip-permissions` | `agy -p`/`--print` | Orca cannot pin its model; compose `agy --model <slug> --effort <level> --dangerously-skip-permissions`, prove TUI readiness, then dispatch with `--terminal` |
| Pi | `pi` | `--approve` | `pi -p`/`--print` | Orca cannot pin its model; for a non-default pin compose `pi --model <provider/model> --thinking <level> --approve`, prove TUI readiness, then dispatch with `--terminal` |
| Zcode CLI | custom terminal | `--mode yolo` | `--prompt`/`-p`/`--print` | Resolve the standalone terminal client as described below, compose `<zcode-cli> tui --mode yolo --cwd <worktree>`, prove TUI readiness, then use the low-level custom-terminal dispatch flow below |
| Kiro CLI | `kiro` | none; `--trust-all-tools` is forbidden on the launch argv | `kiro-cli chat --no-interactive` | |
| Cursor Agent CLI | `cursor` | `--force` | `cursor-agent -p`/`--print` | `--model` pin honoured; omit `--effort` when Effort is `default` |
| GitHub Copilot CLI | `copilot` | `--allow-all` | `copilot -p`/`--prompt` | |

Workspace trust and tool approval are separate boundaries. On first use,
Antigravity may still ask whether the exact worktree is trusted even when
`--dangerously-skip-permissions` is present. Answer that prompt for the named
worktree, close or leave the bootstrap terminal outside orchestration, then
launch a fresh pinned terminal. Require `terminal wait --for tui-idle` to pass
and inspect its banner for the configured model before `worker-start
--terminal`; this prevents a stale trust classification and an unpinned
default-model dispatch from entering the Run.

## Zcode CLI resolution

Do not launch the Electron desktop application as a worker. Resolve a
standalone terminal client whose `version` command prints both its packaging
version and the embedded Zcode runtime version. A desktop application's
`resources/glm/zcode.cjs` is not sufficient: some desktop distributions omit
`@zcode/tui`, so `version` and `doctor` can pass while `tui` fails at startup.
Verify the candidate before launch:

```text
<resolved-zcode-cli> version
<resolved-zcode-cli> doctor --json
```

The doctor result must identify process name `zcode-cli`. Start its interactive
surface with `tui --mode yolo --cwd <exact-worktree>`, then require
`terminal wait --for tui-idle` and inspect the rendered screen. Readiness means
the editor is idle and a model is shown; a setup, login, or trust picker is not
ready. Settle that picker outside orchestration, close the bootstrap terminal,
and launch a fresh terminal before dispatch.

Zcode 0.16.x exposes no non-interactive model-list command and Orca cannot pin
its model, so accept only a `default` model/effort row and record the model
shown by the ready TUI. Never copy credentials into a repository or print them
in a log. If the terminal client needs access setup, complete it in Zcode's
user-scoped configuration and keep the resulting file private.

Until Orca advertises a native Zcode agent ID and status hooks, it rejects both
`worker-start --agent zcode` and `worker-start --terminal` for Zcode. Use the
low-level path:

1. Create the task with `orchestration task-create`.
2. Dispatch it to the ready terminal without `--inject`, requesting the
   returned preamble.
3. Send that exact preamble as one text payload to the terminal and submit it.
   Pass it as a structured process argument; never interpolate it into a shell
   command.
4. Supervise the returned dispatch ID and require its `worker_done` outcome.

If Orca later recognizes Zcode, prefer native `--inject` and the normal
supervised-worker lifecycle.
