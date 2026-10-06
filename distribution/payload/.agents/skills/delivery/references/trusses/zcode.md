# Zcode CLI

Orca agent ID: resolve from live `orchestration worker-start --help` before launch.
CLI executable: resolve a standalone terminal client as below.
Launch classification: native when live help advertises the `zcode` agent ID; otherwise the low-level path below. No other launch may be improvised.
Permission posture: native launch uses the Orca-registered agent preference. The standalone TUI must preserve that posture; this recipe grants no yolo mode.
Forbidden headless forms: `--prompt`, `-p`, and `--print`.

## Zcode CLI resolution

Do not launch the Electron desktop application as a worker. Resolve a
standalone terminal client whose `version` command prints both its packaging
version and the embedded Zcode runtime version. A desktop application's
`resources/glm/zcode.cjs` is not sufficient: some desktop distributions omit
`@zcode/tui`, so `version` and `doctor` can pass while `tui` fails at startup.
Verify the candidate before launch:

```text
<resolved-zcode-cli> version
<resolved-zcode-cli> doctor --json
```

The doctor result must identify process name `zcode-cli`. Start its interactive
surface with `tui --cwd <exact-worktree>`, using only already-authorized
permission settings verified from live help. If those settings cannot preserve
the configured posture, stop with `NEEDS_INPUT`. Then require
`terminal wait --for tui-idle` and inspect the rendered screen. Readiness means
the editor is idle and a model is shown; a setup, login, or trust picker is not
ready. Settle that picker outside orchestration, close the bootstrap terminal,
and launch a fresh terminal before dispatch.

Zcode 0.16.x exposes no non-interactive model-list command. Check the current
live help for model-pin coverage; when it lacks coverage, accept only a
`default` model/effort row and record the model shown by the ready TUI. Never
copy credentials into a repository or print them in a log. If the terminal
client needs access setup, complete it in Zcode's user-scoped configuration and
keep the resulting file private.

## Launch (native, when live help advertises `zcode`)

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --agent zcode --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

Add `--model`/`--effort` only when the live help advertises native model
coverage for this agent and the tuple is non-default. Compare
`launch.requested` with `launch.effective` on the receipt.

## Launch (low-level, only on a live-guide expressiveness gap)

This is an **unsupervised low-level Dispatch**, not a supervised worker
resource. Use it only if the live orchestration guide documents this exact
gap and the approved envelope allows the weaker lifecycle. It creates no
worker resource row; worker stop/abandon cannot stop its terminal. If Delivery
requires supervised worker ownership, stop with `UNSUPPORTED_LAUNCH` until
normal `worker-start` supports this Zcode launch.

1. Create the Task using the self-contained Task recipe in
   `../command-recipes.md`.
2. On a ready terminal, use the documented low-level topology recipe:

   ```bash
   "$DELIVERY_ORCA_CLI" orchestration dispatch --task "$DELIVERY_TASK_ID" --to "$DELIVERY_TERMINAL_HANDLE" --inject --run "$DELIVERY_RUN_ID" --json
   ```

3. `--inject` delivers the authoritative preamble; do not duplicate it with a
   manual terminal send. Capture the returned Dispatch ID and follow the live
   orchestration worker contract for messages, `worker_done`, and outcome.
   Record the lane's unsupervised ownership/recovery limits in the envelope.

Shared readiness and retry rules remain in [Execution](../execution.md); commands live in
[Command recipes](../command-recipes.md).
