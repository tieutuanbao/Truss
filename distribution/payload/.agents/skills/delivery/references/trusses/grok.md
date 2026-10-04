# Grok Build

Orca agent ID: `grok`.
CLI executable: `grok`.
Launch classification: unresolved — live `worker-start --help` names no native `--model` support for this agent, and no composed launch argv is verified here.
Permission default observed on interactive launches: `--permission-mode bypassPermissions`.
Forbidden headless form: `grok --prompt-file`.

A delivery whose approved envelope pins this truss stops before dispatch and
reports `UNSUPPORTED_LAUNCH` with the observed tuple. Do not guess a launch-
time model call for an agent the live help does not name, and do not invent a
composed argv: a pinned dispatch here requires documenting and verifying a
recipe in this file first, per `../../SKILL.md` § Changing this skill.

Model and effort discovery for setup only: `grok models`; effort vocabulary
from the installed CLI's help. Shared readiness and retry rules remain in
`../../SKILL.md`.
