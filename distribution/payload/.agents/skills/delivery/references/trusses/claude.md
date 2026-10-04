# Claude Code

Orca agent ID: `claude`.
CLI executable: `claude`.
Launch classification: native — live `worker-start --help` names this agent under native `--model` support.
Permission default on the Orca-composed launch: `--dangerously-skip-permissions`.
Forbidden headless form: `claude -p`.

## Launch

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --agent claude --model "$DELIVERY_MODEL" --effort "$DELIVERY_EFFORT" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

Omit `--effort` when the tuple's Effort is `default`. `--effort` requires
`--model`; neither combines with `--terminal`. Prove the pin from the receipt:
`launch.requested` equals the tuple and `launch.effective` names the resolved
model.

Shared readiness and retry rules remain in `../../SKILL.md`; commands live in
`../../references/command-recipes.md`.
