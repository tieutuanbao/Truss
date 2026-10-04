# OpenCode

Orca agent ID: `opencode`.
CLI executable: `opencode`.
Launch classification: composed — Orca refuses launch-time model selection for this agent; the pin travels on the terminal argv of the minimal TUI.
Forbidden headless form: `opencode run`.

Check live `orchestration worker-start --help` for native model coverage. When
coverage is absent, compose the interactive pin on the minimal TUI:

```bash
"$DELIVERY_ORCA_CLI" terminal create --worktree "$DELIVERY_WORKTREE_SELECTOR" --title "$DELIVERY_TERMINAL_TITLE" --command "opencode mini --model '$DELIVERY_MODEL'" --json
```

`DELIVERY_MODEL` is the tuple's Model cell, `provider/model` with an optional
`#variant` (for example `openai/gpt-5.2#high`); a chosen variant travels inside
that selector, never as a separate effort flag. When the tuple's Model is
`default`, launch `opencode mini` with no model flag. Take the handle from
`.result.terminal.handle`. Appending `--prompt <first message>` sends that
message at launch; without it the launch waits for the usual dispatch input.

The default `opencode` TUI takes no model flag (observed v2.0.22: passing one
prints help and exits), so the pin rides `mini`. The variant is part of the
selector — this agent has no separate effort flag anywhere — and the status
bar names `model [variant] · provider`, so the readiness read proves the pin.
An unknown variant is accepted at launch and fails model resolution only when
a message is sent: the error is reported inside the TUI and the dispatch stays
unsettled, so read the terminal rather than assuming a stuck worker.

## Readiness

```bash
"$DELIVERY_ORCA_CLI" terminal wait --terminal "$DELIVERY_TERMINAL_HANDLE" --for tui-idle --timeout-ms 60000 --json
```

Continue only on `satisfied: true`, then prove the model from the status bar
or `launch.effective`.

## Dispatch

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --terminal "$DELIVERY_TERMINAL_HANDLE" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

Never add `--agent`, `--model`, or `--effort` to this call: the terminal argv
already carries those pins, and observed 1.4.217 help says such flags cannot
combine with `--terminal`.

Permission posture comes from opencode's own `permissions` rules, shown
resolved by `opencode debug config`; the default base policy allows shell and
edits and asks for external-directory and `.env` reads. A matching `ask`
stalls an unattended worker at a TUI dialog, and the pass is configuration,
not keystrokes: give shell and edit `allow` rules in opencode's project or
user configuration before launch, and verify before dispatch that neither
asks. `mini` takes no permission flag: `--auto` exists only on the default
TUI, and a flag `mini` does not accept makes it print help and exit. `mini`
requires a TTY: piped stdout exits with `opencode mini requires a TTY stdout`
before any TUI appears.

Shared readiness and retry rules remain in `../../SKILL.md`; commands live in
`../../references/command-recipes.md`.
