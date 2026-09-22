# Claude Code

Orca agent ID: `claude`.
Permission default: `--dangerously-skip-permissions`.
Forbidden headless form: `claude -p`.

`worker-start --model` supports this agent and honours the model pin.
`--effort` requires `--model`; neither combines with `--terminal`.
Shared readiness and retry rules remain in `../../SKILL.md`.
