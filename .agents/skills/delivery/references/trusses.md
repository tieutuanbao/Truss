# Truss launch mechanics

Use `../SKILL.md` § CLI identity and preflight for the canonical CLI
commands, consumer Git-root and HEAD checks, binary-identity boundary, and
no-fallback rule.

For installation setup, register the CLI in Orca desktop under
**Settings → Experimental → CLI** and enable **Orchestration**.

Before dispatch, confirm that the Orca Run/worktree selector resolves to the
validated consumer root. Record the exact worktree path and baseline
SHA. Released terminals do not prove child worktrees were removed; follow
`../SKILL.md` § Failure and recovery before cleanup or another Run.

Read only the launch reference for each resolved truss used in this run.
Shared readiness, permission-posture, dispatch completion, and retry rules
remain in `../SKILL.md`.

| Truss | Launch reference |
| --- | --- |
| Claude Code | [Claude Code](trusses/claude.md) |
| Codex CLI | [Codex CLI](trusses/codex.md) |
| Grok Build | [Grok Build](trusses/grok.md) |
| Antigravity CLI | [Antigravity CLI](trusses/antigravity.md) |
| Pi | [Pi](trusses/pi.md) |
| Zcode CLI | [Zcode CLI](trusses/zcode.md) |
| Kiro CLI | [Kiro CLI](trusses/kiro.md) |
| Cursor Agent CLI | [Cursor Agent CLI](trusses/cursor.md) |
| GitHub Copilot CLI | [GitHub Copilot CLI](trusses/copilot.md) |
