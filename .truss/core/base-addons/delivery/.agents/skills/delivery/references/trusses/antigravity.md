# Antigravity CLI

Orca agent ID: `antigravity`.
`worker-start --agent agy` returns `agent_unconfigured` and creates no terminal.
Permission default: `--dangerously-skip-permissions`.
Forbidden headless forms: `agy -p` and `agy --print`.

Orca cannot pin this agent's model. Compose:

`agy --model <slug> --effort <level> --dangerously-skip-permissions`

Prove TUI readiness before dispatch with `--terminal`.
`--model` and `--effort` cannot combine with `--terminal`.

Workspace trust and tool approval are separate boundaries. On first use,
Antigravity may still ask whether the exact worktree is trusted even when
`--dangerously-skip-permissions` is present. Answer that prompt for the named
worktree, close or leave the bootstrap terminal outside orchestration, then
launch a fresh pinned terminal. Require `terminal wait --for tui-idle` to pass
and inspect its banner for the configured model before `worker-start
--terminal`; this prevents a stale trust classification and an unpinned
default-model dispatch from entering the Run.
