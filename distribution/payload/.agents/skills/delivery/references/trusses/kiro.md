# Kiro CLI

Orca agent ID: `kiro`.
CLI executable: `kiro-cli`.
Launch classification: unresolved — live `worker-start --help` names no native `--model` support for this agent, and no composed launch argv is verified here.
`worker-start --agent kiro-cli` returns `agent_unconfigured` and creates no terminal: the agent id is `kiro`, never `kiro-cli`.
Permission default: none. `--trust-all-tools` is forbidden on any launch argv.
Forbidden headless form: `kiro-cli chat --no-interactive`.

A delivery whose approved envelope pins this truss stops before dispatch and
reports `UNSUPPORTED_LAUNCH` with the observed tuple. Do not guess a launch-
time model call for an agent the live help does not name, and do not invent a
composed argv: a pinned dispatch here requires documenting and verifying a
recipe in this file first, per `../../SKILL.md` § Changing this skill.

Model and effort discovery for setup only: `kiro-cli chat --list-models
--format json`; effort vocabulary from `kiro-cli chat --help`. Shared
readiness and retry rules remain in `../../SKILL.md`.
