# OpenCode

Orca agent ID: `opencode`.
Forbidden headless form: `opencode run`.

Check live `orchestration worker-start --help` for native model coverage. When
coverage is absent, compose the interactive pin on the minimal TUI:

`opencode mini --model <provider/model[#variant]> --prompt <first message>`

The default `opencode` TUI takes no `--model` (observed v2.0.22: passing it
prints help and exits), so the pin rides `mini`. The variant is part of the
selector — `provider/model#variant`; v2 has no separate `--variant` or
`--effort` flag — and the status bar names `model [variant] · provider`, so
the readiness read proves the pin. An unknown variant is accepted at launch
and fails model resolution only when a message is sent: the error is reported
inside the TUI and the dispatch stays unsettled, so read the terminal rather
than assuming a stuck worker. `--prompt` sends its message at launch; a
launch without `--prompt` waits for the usual dispatch input.

Permission posture comes from opencode's own configuration (`permission` in
`opencode debug config`); never add `--auto`, which auto-approves permissions
that are not explicitly denied. `mini` requires a TTY: piped stdout exits
with `opencode mini requires a TTY stdout` before any TUI appears.

Prove the composed TUI is ready before dispatch with `--terminal`, and prove
the model from its status bar or `launch.effective`.
Shared readiness and retry rules remain in `../../SKILL.md`.
