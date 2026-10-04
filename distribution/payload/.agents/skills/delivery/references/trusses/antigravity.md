# Antigravity CLI

Orca agent ID: `antigravity`.
CLI executable: `agy`.
Launch classification: native — live `worker-start --help` names this agent under native `--model` support; a composed fallback exists for a server that cannot pin.
`worker-start --agent agy` returns `agent_unconfigured` and creates no terminal: the agent id is `antigravity`, never `agy`.
Permission posture: native launch uses the Orca-registered agent preference; `--dangerously-skip-permissions` applies to the composed fallback argv only.
Forbidden headless forms: `agy -p` and `agy --print`.

## Launch (native)

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --agent antigravity "${DELIVERY_MODEL_ARGS[@]}" "${DELIVERY_EFFORT_ARGS[@]}" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

Omit `--effort` when the tuple's Effort is `default` (`low|medium|high`
otherwise). Compare `launch.requested` with `launch.effective` on the receipt.

## Launch (composed fallback)

Use the composed path only when the selected server cannot pin the native
launch. Compose the pinned terminal from the truss argv, prove readiness, then
dispatch with `--terminal` only — never re-adding the pins:

```bash
DELIVERY_AGENT_ARGV=(agy --dangerously-skip-permissions)
[ "$DELIVERY_MODEL" = default ] || DELIVERY_AGENT_ARGV+=(--model "$DELIVERY_MODEL")
[ "$DELIVERY_EFFORT" = default ] || DELIVERY_AGENT_ARGV+=(--effort "$DELIVERY_EFFORT")
DELIVERY_COMPOSED_ARGV="$(shell_quote_argv "${DELIVERY_AGENT_ARGV[@]}")"
"$DELIVERY_ORCA_CLI" terminal create --worktree "$DELIVERY_WORKTREE_SELECTOR" --title "$DELIVERY_TERMINAL_TITLE" --command "$DELIVERY_COMPOSED_ARGV" --json
"$DELIVERY_ORCA_CLI" terminal wait --terminal "$DELIVERY_TERMINAL_HANDLE" --for tui-idle --timeout-ms 60000 --json
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --terminal "$DELIVERY_TERMINAL_HANDLE" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

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

Shared readiness and retry rules remain in `../../SKILL.md`; commands live in
`../../references/command-recipes.md`.
