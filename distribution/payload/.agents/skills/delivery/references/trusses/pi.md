# Pi

Orca agent ID: `pi`.
CLI executable: `pi`.
Launch classification: composed — Orca refuses launch-time model selection for this agent (`Agent pi does not support launch-time model selection`); the pin travels on the terminal argv.
Project-local trust flag: `--approve` (Pi help: trust project-local files for this run); this is not a shell/tool permission bypass.
Forbidden headless forms: `pi -p` and `pi --print`.

## Launch

```bash
DELIVERY_AGENT_ARGV=(pi)
[ "$DELIVERY_MODEL" = default ] || DELIVERY_AGENT_ARGV+=(--model "$DELIVERY_MODEL")
[ "$DELIVERY_EFFORT" = default ] || DELIVERY_AGENT_ARGV+=(--thinking "$DELIVERY_EFFORT")
DELIVERY_AGENT_ARGV+=(--approve)
DELIVERY_COMPOSED_ARGV="$(shell_quote_argv "${DELIVERY_AGENT_ARGV[@]}")"
"$DELIVERY_ORCA_CLI" terminal create --worktree "$DELIVERY_WORKTREE_SELECTOR" --title "$DELIVERY_TERMINAL_TITLE" --command "$DELIVERY_COMPOSED_ARGV" --json
```

`DELIVERY_MODEL` is the tuple's Model cell (for example
`tao-router/thinking`); `DELIVERY_EFFORT` maps to Pi's `--thinking` level
(`off|minimal|low|medium|high|xhigh|max`), not to an Orca effort flag. The
argv construction above omits each flag independently when its cell is
`default`; no literal `default` reaches Pi. `--approve` only trusts
project-local files; it does not change shell/tool permission policy.
Take the handle from `.result.terminal.handle`. Never launch with `--prompt`:
dispatch input arrives from the supervised start.

## Readiness

```bash
"$DELIVERY_ORCA_CLI" terminal wait --terminal "$DELIVERY_TERMINAL_HANDLE" --for tui-idle --timeout-ms 60000 --json
```

Continue only on `satisfied: true`, then prove the pin from the rendered
banner or status line: Pi names the resolved model and thinking level. A
login, trust, or model picker is not ready.

## Dispatch

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --terminal "$DELIVERY_TERMINAL_HANDLE" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

Never add `--agent`, `--model`, or `--effort` to this call: the terminal argv
already carries those pins, and observed 1.4.217 help says such flags cannot
combine with `--terminal`. Repeating them here is a launch error, not a
stronger pin.

Model and effort discovery for setup: `pi --list-models` offers the resolved
`provider/model` pair; thinking levels come from `pi --help`.

Shared readiness and retry rules remain in [Execution](../execution.md); commands live in
[Command recipes](../command-recipes.md).
