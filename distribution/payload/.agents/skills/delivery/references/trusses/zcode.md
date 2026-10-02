# Zcode CLI

Orca agent ID: resolve from live `orchestration worker-start --help` before
launch.
Permission default: `--mode yolo`.
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
surface with `tui --mode yolo --cwd <exact-worktree>`, then require
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

When live help advertises the `zcode` agent ID, use native
`worker-start --agent zcode` as the default path, require readiness, and
compare `launch.requested` with `launch.effective`. Use the low-level path only
when the live guide identifies a genuine expressiveness gap:

1. Create the task with `orchestration task-create`.
2. Dispatch it to the ready terminal without `--inject`, requesting the
   returned preamble.
3. Send that exact preamble as one text payload to the terminal and submit it.
   Pass it as a structured process argument; never interpolate it into a shell
   command.
4. Supervise the returned dispatch ID and require its `worker_done` outcome.
