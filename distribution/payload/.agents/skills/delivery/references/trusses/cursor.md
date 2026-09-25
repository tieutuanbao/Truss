# Cursor Agent CLI

Orca agent ID: `cursor`.
`worker-start --agent cursor-agent` returns `agent_unconfigured` and creates no terminal.
Permission default: `--force`.
Forbidden headless forms: `cursor-agent -p` and `cursor-agent --print`.

`worker-start --model` supports this agent and honours the model pin.
Omit `--effort` when Effort is `default`.
Otherwise `--effort` requires `--model`; neither combines with `--terminal`.
Shared readiness and retry rules remain in `../../SKILL.md`.
