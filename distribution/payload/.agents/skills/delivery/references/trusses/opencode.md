# OpenCode

Orca agent ID: `opencode`.
Forbidden headless form: `opencode run`.

Check live `orchestration worker-start --help` for native model coverage. When
coverage is absent, compose a non-default pin:

`opencode --model <provider/model>`

The interactive command accepts `--model`, and its banner names the resolved
model, so readiness proves the pin. It has no effort flag: `--variant` exists
only on `opencode run`, which is the forbidden headless form, and reasoning
effort comes from the resolved model or its configured variant. Write `default`
for Effort and let the model carry the effort, because an argv flag that
opencode does not accept makes it print help and exit instead of starting a TUI.

Permission posture comes from opencode's own configuration (`permission` in
`opencode debug config`); never add `--auto`, which auto-approves permissions
that are not explicitly denied.

A model that refuses a request reports the error inside the TUI and leaves the
dispatch unsettled, so read the terminal rather than assuming a stuck worker.

Prove the composed TUI is ready before dispatch with `--terminal`, and prove
the model from its banner or `launch.effective`. Observed 1.4.217 help says
`--model` and `--effort` cannot combine with `--terminal`.
Shared readiness and retry rules remain in `../../SKILL.md`.
