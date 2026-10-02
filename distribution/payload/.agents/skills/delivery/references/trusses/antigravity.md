# Antigravity CLI

Orca agent ID: `antigravity`.
`worker-start --agent agy` returns `agent_unconfigured` and creates no terminal.
Permission default: `--dangerously-skip-permissions`.
Forbidden headless forms: `agy -p` and `agy --print`.

Check live `orchestration worker-start --help` for model coverage. When it
advertises native coverage, use `worker-start --agent antigravity --model <id>
--effort <level>`, require readiness, and compare `launch.requested` with
`launch.effective`. Use the composed argv only when the selected server cannot
pin the native launch:

`agy --model <slug> --effort <level> --dangerously-skip-permissions`

For the composed path, prove TUI readiness before dispatch with `--terminal`.
Observed 1.4.217 help says `--model` and `--effort` cannot combine with
`--terminal`.

Workspace trust and tool approval are separate boundaries. On first use,
Antigravity may still ask whether the exact worktree is trusted even when
`--dangerously-skip-permissions` is present. Answer that prompt for the named
worktree, close or leave the bootstrap terminal outside orchestration, then
launch a fresh pinned terminal. Require `terminal wait --for tui-idle` to pass
and inspect its banner for the configured model before `worker-start
--terminal`; this prevents a stale trust classification and an unpinned
default-model dispatch from entering the Run.
