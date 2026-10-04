# Delivery command recipes

The command layer of `../../SKILL.md`. Every dispatch, wait, acknowledgement,
and recovery command in a delivery run is copied from this file with the
placeholders filled from receipts. Commands are never composed from memory,
from prose, or from live help alone: live `--help` owns flags this file does
not name, and a launch this file cannot express stops the delivery instead of
being improvised.

## Canonical variables

Set once per run; every recipe reads them. The comment names the receipt that
supplies each value.

```bash
DELIVERY_ORCA_CLI="${DELIVERY_ORCA_CLI:-orca-ide}"
DELIVERY_ROOT="$(git rev-parse --show-toplevel)"            # consumer Git root
DELIVERY_HEAD="$(git -C "$DELIVERY_ROOT" rev-parse HEAD)"   # baseline SHA
DELIVERY_WORKTREE_SELECTOR="path:$DELIVERY_ROOT"            # or id:<repoId>::<path>
DELIVERY_RUN_ID=""            # run_<id>    run-create / run-list / run-show
DELIVERY_TASK_ID=""           # task_<id>   task-create / worker-start / task-list
DELIVERY_DISPATCH_ID=""       # ctx_<id>    worker-start / worker-show / worker-list
DELIVERY_DELIVERY_ID=""       # delivery_<id>  the check batch, NOT a msg_ id
DELIVERY_MESSAGE_ID=""        # msg_<id>    one message inside a check batch
DELIVERY_REQUEST_ID=""        # UUID        error.data.orchestrationRequestId
DELIVERY_TERMINAL_HANDLE=""   # term_<id>   terminal create / start receipts
DELIVERY_TERMINAL_TITLE=""    # short role label, e.g. "BA pi tao-router"
DELIVERY_TASK_TITLE=""        # concise orchestration task title
DELIVERY_PROMPT_FILE=""       # in-worktree prompt path, see § Prompt and task transport
DELIVERY_REPORT_PATH=""       # in-worktree handoff path, travels as payload.reportPath
DELIVERY_MODEL=""             # AGENTS.md tuple; empty when the Model cell is `default`
DELIVERY_EFFORT=""            # AGENTS.md tuple; empty when the Effort cell is `default`
```

## Identifier taxonomy

| Prefix | Kind | Returned by | Consumed by |
| --- | --- | --- | --- |
| `run_` | Run | `run-create`, `run-list` | `--run`, `run-use --id`, `run-show` |
| `task_` | Task | `task-create`, `worker-start` receipt, `task-list` | `worker-start --task`, `--retry-of` requires `--task` |
| `ctx_` | Dispatch | `worker-start` receipt (`dispatchId`), `worker-show`, `worker-list` | `worker-show/read/release/retain/stop/abandon --dispatch`, `dispatch --task --to` |
| `msg_` | Message | one row of a `check` batch | `reply --id` |
| `delivery_` | Delivery batch | `check` result `deliveryId` | `check --ack` |
| `term_` | Terminal handle | `terminal create`, start receipts | `terminal` verbs, `worker-start --terminal` |
| UUID | Request | `error.data.orchestrationRequestId`, `mutation.requestId` | `request-show --request`, `--retry-request` |

Copy identifiers from JSON with extraction commands, never by retyping; see
§ Retry and recovery.

## Preflight

```bash
command -v "$DELIVERY_ORCA_CLI"
"$DELIVERY_ORCA_CLI" --version
"$DELIVERY_ORCA_CLI" status --json
"$DELIVERY_ORCA_CLI" orchestration run-list --json
```

The co-installed business-analyst skill resolves from the consumer root, not
from the execution plane. It is not an `orca skills get` topic; its cited
references resolve relative to the skill directory.

```bash
DELIVERY_BA_SKILL="$DELIVERY_ROOT/.agents/skills/business-analyst/SKILL.md"
test -r "$DELIVERY_BA_SKILL"
```

## Run lifecycle

```bash
"$DELIVERY_ORCA_CLI" orchestration run-create --objective "<objective>" --json
"$DELIVERY_ORCA_CLI" orchestration run-current --json
"$DELIVERY_ORCA_CLI" orchestration run-list --json
"$DELIVERY_ORCA_CLI" orchestration run-use --id "$DELIVERY_RUN_ID" --json
```

`run-use` rebinds a new coordinator terminal to an existing Run when resuming;
it never rebinds a terminal that already owns a current Run.

## Prompt and task transport

The prompt is an in-worktree file, never a shell argument, and never
interpolated into one. The task spec is a short pointer that names the prompt
file; the worker reads the file from the candidate worktree.

```bash
"$DELIVERY_ORCA_CLI" orchestration task-create --spec "Read the task prompt at $DELIVERY_PROMPT_FILE_RELATIVE inside the candidate worktree and execute it exactly. Write the handoff to $DELIVERY_REPORT_PATH_RELATIVE and return its path as payload.reportPath." --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

`DELIVERY_PROMPT_FILE_RELATIVE` and `DELIVERY_REPORT_PATH_RELATIVE` are the
worktree-relative forms of the two paths. Do not stage the prompt file; in a
repository-hosted run, delete it after the worker returns.

## Native worker launch

Only for a truss whose reference states `Launch classification: native` — an
agent the live `worker-start --help` literally names under native `--model`
support. A single line, so the receipt is one command:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --agent "$DELIVERY_AGENT_ID" --model "$DELIVERY_MODEL" --effort "$DELIVERY_EFFORT" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

Omit `--effort` when the tuple's Effort is `default`. `--effort` requires
`--model`; neither combines with `--terminal`. Prove the pin from the receipt:
`launch.requested` equals the tuple and `launch.effective` names the resolved
model.

## Composed worker launch

For a truss whose reference states `Launch classification: composed` — Orca
refuses launch-time model selection for the agent, so the pin travels on the
terminal argv from that truss reference. Step one, carrying the pins:

```bash
"$DELIVERY_ORCA_CLI" terminal create --worktree "$DELIVERY_WORKTREE_SELECTOR" --title "$DELIVERY_TERMINAL_TITLE" --command "$DELIVERY_COMPOSED_ARGV" --json
```

`DELIVERY_COMPOSED_ARGV` is copied verbatim from the resolved truss reference
(for example `pi --model 'tao-router/thinking' --thinking 'high' --approve`).
Take the handle from `.result.terminal.handle` into `DELIVERY_TERMINAL_HANDLE`.
Never launch with `--prompt`: dispatch input arrives from the supervised start.

Step two, readiness:

```bash
"$DELIVERY_ORCA_CLI" terminal wait --terminal "$DELIVERY_TERMINAL_HANDLE" --for tui-idle --timeout-ms 60000 --json
```

Continue only on `satisfied: true`, and prove the pin from the rendered banner
or status line before dispatch. A trust, login, or model picker is not ready.

Step three, the supervised start. Never add `--agent`, `--model`, or `--effort`
on this path — the terminal argv already carries those pins, and Orca rejects
the combination:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --terminal "$DELIVERY_TERMINAL_HANDLE" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

## Completion wait and acknowledgement

```bash
"$DELIVERY_ORCA_CLI" orchestration check --wait --types "worker_done,escalation,question" --timeout-ms 900000 --json
```

The batch carries `result.deliveryId` — that is `DELIVERY_DELIVERY_ID` — and
`result.messages[]`, whose `id` values are `msg_` identifiers. Acknowledge the
batch by its delivery id; acknowledging a `msg_` id is a category error and
redelivers nothing:

```bash
"$DELIVERY_ORCA_CLI" orchestration check --ack "$DELIVERY_DELIVERY_ID" --json
```

A timeout is a transport outcome: re-enter the wait or read the terminal
before concluding anything about the worker.

## Evidence reads

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-show --dispatch "$DELIVERY_DISPATCH_ID" --json
"$DELIVERY_ORCA_CLI" orchestration worker-read --dispatch "$DELIVERY_DISPATCH_ID" --source auto --json
```

## Settlement

After settlement or positive exit evidence, choose exactly one:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-release --dispatch "$DELIVERY_DISPATCH_ID" --json
"$DELIVERY_ORCA_CLI" orchestration worker-retain --dispatch "$DELIVERY_DISPATCH_ID" --json
```

`worker-stop` and `worker-abandon` are recovery actions with the same
`--dispatch` argument; they are chosen from failure evidence, not from absence.
Close the run only when the fleet owes nothing:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-list --run "$DELIVERY_RUN_ID" --terminal-state reclaimable --json
```

## Retry and recovery

Inspect an unknown mutation before any replay:

```bash
"$DELIVERY_ORCA_CLI" orchestration request-show --request "$DELIVERY_REQUEST_ID" --json
```

Extract the request id from the failed command's own JSON receipt instead of
retyping a UUID:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --json 2>&1 | tee "$DELIVERY_RECEIPT_FILE"
DELIVERY_REQUEST_ID="$(grep -oE '"orchestrationRequestId"[^"]*"[^"]*"' "$DELIVERY_RECEIPT_FILE" | head -1 | sed 's/.*"\([^"]*\)"$/\1/')"
test -n "$DELIVERY_REQUEST_ID"
```

Replay the same mutation through the same identity with `--retry-request
"$DELIVERY_REQUEST_ID"`. A failed or stopped dispatch is retried once with the
original Task and explicit placement — nothing is inherited:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --retry-of "$DELIVERY_DISPATCH_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --terminal "$DELIVERY_TERMINAL_HANDLE" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

A second identical failure on the same agent and worktree escalates to the
human; it is never worked around with a different agent, an omitted pin, or a
direct path.

## Launch decision tree

For the resolved tuple `role + truss + model + effort`:

1. Normalize the Truss cell through the table in `references/trusses.md`. A
   cell that matches no row stops the dispatch.
2. Read the resolved truss's launch reference. It states exactly one launch
   classification:
   - `native` — use § Native worker launch.
   - `composed` — use § Composed worker launch with that reference's argv.
   - `unresolved` — stop and report `UNSUPPORTED_LAUNCH` with the observed
     tuple; never guess a native `--model` call and never invent composed argv.
3. Native support is read literally: only an agent the live
   `worker-start --help` names in its native `--model` support sentence takes a
   native model pin. Presence among the `--agent` examples is not model-pin
   support.
