# GitHub Copilot CLI

Orca agent ID: `copilot`.
CLI executable: `copilot`.
Launch classification: unresolved — live `worker-start --help` names no native `--model` support for this agent, and no composed launch argv is verified here.
Permission default observed on interactive launches: `--allow-all`.
Forbidden headless forms: `copilot -p` and `copilot --prompt`.

A delivery whose approved envelope pins this truss stops before dispatch and
reports `UNSUPPORTED_LAUNCH` with the observed tuple. Do not guess a launch-
time model call for an agent the live help does not name, and do not invent a
composed argv: a pinned dispatch here requires documenting and verifying a
recipe in this file first, per [Release and recovery](../release-and-recovery.md) § Changing this skill.

Model and effort discovery for setup only: there is no non-interactive model
listing; effort vocabulary from `copilot --help`. Shared readiness and retry
rules remain in [Execution](../execution.md).
