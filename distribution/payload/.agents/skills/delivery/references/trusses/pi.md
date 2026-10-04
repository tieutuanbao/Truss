# Pi

Orca agent ID: `pi`.
CLI executable: `pi`.
Launch classification: composed — Orca refuses launch-time model selection for this agent (`Agent pi does not support launch-time model selection`); the pin travels on the terminal argv.
Permission default on the composed argv: `--approve`.
Forbidden headless forms: `pi -p` and `pi --print`.

## Launch

```bash
"$DELIVERY_ORCA_CLI" terminal create --worktree "$DELIVERY_WORKTREE_SELECTOR" --title "$DELIVERY_TERMINAL_TITLE" --command "pi --model '$DELIVERY_MODEL' --thinking '$DELIVERY_EFFORT' --approve" --json
```

`DELIVERY_MODEL` is the tuple's Model cell (for example
`tao-router/thinking`); `DELIVERY_EFFORT` maps to Pi's `--thinking` level
(`off|minimal|low|medium|high|xhigh|max`), not to an Orca effort flag. When the
tuple's Model is `default`, omit the `--model`/`--thinking` pair entirely.
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

Shared readiness and retry rules remain in `../../SKILL.md`; commands live in
`../../references/command-recipes.md`.
