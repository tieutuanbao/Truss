# OpenCode

Orca agent ID: `opencode`.
CLI executable: `opencode`.
Launch classification: composed — Orca refuses launch-time model selection for this agent; the pin travels on the terminal argv of the minimal TUI.
Forbidden headless form: `opencode run`.

Check live `orchestration worker-start --help` for native model coverage. When
coverage is absent, compose the interactive pin on the minimal TUI:

```bash
DELIVERY_AGENT_ARGV=(opencode mini)
[ "$DELIVERY_MODEL" = default ] || DELIVERY_AGENT_ARGV+=(--model "$DELIVERY_MODEL")
DELIVERY_COMPOSED_ARGV="$(shell_quote_argv "${DELIVERY_AGENT_ARGV[@]}")"
"$DELIVERY_ORCA_CLI" terminal create --worktree "$DELIVERY_WORKTREE_SELECTOR" --title "$DELIVERY_TERMINAL_TITLE" --command "$DELIVERY_COMPOSED_ARGV" --json
```

`DELIVERY_MODEL` is the tuple's Model cell, `provider/model` with an optional
`#variant` (for example `openai/gpt-5.2#high`); a chosen variant travels inside
that selector, never as a separate effort flag. `Effort` must be `default`;
otherwise stop with `UNSUPPORTED_TUPLE` because this agent has no effort flag.
The argv builder omits `--model` for `Model=default`, never passing the literal
word. Never launch Delivery work with `--prompt`: the supervised
`worker-start` injection supplies the authoritative Task and preamble.

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
