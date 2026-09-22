# Pi

Orca agent ID: `pi`.
Permission default: `--approve`.
Forbidden headless forms: `pi -p` and `pi --print`.

Orca cannot pin this agent's model. For a non-default pin compose:

`pi --model <provider/model> --thinking <level> --approve`

Prove TUI readiness before dispatch with `--terminal`.
`--model` and `--effort` cannot combine with `--terminal`.
Shared readiness and retry rules remain in `../../SKILL.md`.
