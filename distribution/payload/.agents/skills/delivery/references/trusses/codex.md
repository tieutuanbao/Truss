# Codex CLI

Orca agent ID: `codex`.
CLI executable: `codex`.
Launch classification: native — live `worker-start --help` names this agent under native `--model` support.
Permission posture: native launch uses the Orca-registered agent preference; `--dangerously-bypass-approvals-and-sandbox` applies only to a separately authorized composed CLI launch.
Forbidden headless form: `codex exec`.

## Launch

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --agent codex "${DELIVERY_MODEL_ARGS[@]}" "${DELIVERY_EFFORT_ARGS[@]}" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

Omit `--effort` when the tuple's Effort is `default`. `--effort` requires
`--model`; neither combines with `--terminal`. Prove the pin from the receipt:
`launch.requested` equals the tuple and `launch.effective` names the resolved
model.

Shared readiness and retry rules remain in `../../SKILL.md`; commands live in
`../../references/command-recipes.md`.
