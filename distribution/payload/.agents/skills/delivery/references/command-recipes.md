# Delivery command recipes

Executable command layer for [Execution](execution.md). Resolve the Orca executable by
following the installed `orca-cli` skill exactly; do not use a guessed default
or silently substitute another binary. Its precedence is `ORCA_CLI_COMMAND`;
else `orca-dev` when `ORCA_DEV_REPO_ROOT` is set; else `orca-ide` on Linux
outside managed terminals, never bare `orca` there; else `orca`. If the
execution context does not establish which case applies, stop and ask. Assign
the resolved executable once to `DELIVERY_ORCA_CLI` and use it throughout.

## Canonical variables and tuple mapping

Resolve values from the named sources. `default` is a policy value and must
map to omitted flags, never to the literal argv string `default`.

```bash
: "${DELIVERY_ORCA_CLI:?resolve using the installed orca-cli skill}"
DELIVERY_SKILL_DIR=""                # absolute installed Delivery skill directory
DELIVERY_APPROVED_ENVELOPE=""         # immutable approved envelope path
DELIVERY_ROLE=""                      # dispatched role from the approved task
DELIVERY_ROOT="$(git rev-parse --show-toplevel)"
DELIVERY_HEAD="$(git -C "$DELIVERY_ROOT" rev-parse HEAD)"
DELIVERY_WORKTREE_SELECTOR="path:$DELIVERY_ROOT"
DELIVERY_RUN_ID=""                  # run-create result.run.id
DELIVERY_TASK_ID=""                 # unique task_ identifier from task-create receipt
DELIVERY_DISPATCH_ID=""             # worker-start result.dispatchId (ctx_*)
DELIVERY_DELIVERY_ID=""             # check result.deliveryId (delivery_*), not msg_*
DELIVERY_MESSAGE_ID=""              # check result.messages[].id (msg_*)
DELIVERY_REQUEST_ID=""              # error.data.orchestrationRequestId (UUID)
DELIVERY_TERMINAL_HANDLE=""         # terminal create result.terminal.handle
DELIVERY_TERMINAL_TITLE=""          # concise role label
DELIVERY_TASK_TITLE=""              # concise Task title
DELIVERY_AGENT_ID=""                # exact ID from references/trusses.md
DELIVERY_MODEL=""                   # exact AGENTS.md Model cell
DELIVERY_EFFORT=""                  # exact AGENTS.md Effort cell
DELIVERY_PROMPT_FILE=""             # in-worktree full prompt path
DELIVERY_REPORT_PATH=""             # in-worktree handoff path
DELIVERY_RECEIPT_FILE=""            # exact JSON stdout receipt
DELIVERY_COMPOSED_ARGV=""           # safely quoted command from adapter array
DELIVERY_PERMISSION_SOURCE=""       # source/posture named by adapter
DELIVERY_MODEL_ARGS=()               # native; empty for Model=default
DELIVERY_EFFORT_ARGS=()              # native; adapter-specific
DELIVERY_AGENT_ARGV=()               # composed executable argv before quoting
```

| Prefix | Kind | Returned by | Consumed by |
| --- | --- | --- | --- |
| `run_` | Run | `run-create` result.run.id | `--run`, `run-use --id` |
| `task_` | Task | `task-create` receipt | `worker-start --task` |
| `ctx_` | Dispatch | `worker-start` result.dispatchId | worker lifecycle `--dispatch` |
| `msg_` | Message | `check.result.messages[].id` | `reply --id` |
| `delivery_` | Delivery batch | `check.result.deliveryId` | `check --ack` |
| `term_` | Terminal | `terminal create` receipt | terminal verbs, `worker-start --terminal` |
| UUID | Mutation request | `orchestrationRequestId` | `request-show --request`, `--retry-request` |

For Orca-native pins, construct optional args as arrays. This matrix is valid
only for agents whose live help says `--effort` requires `--model`:

```bash
DELIVERY_MODEL_ARGS=()
DELIVERY_EFFORT_ARGS=()
if [ "$DELIVERY_MODEL" != default ]; then DELIVERY_MODEL_ARGS=(--model "$DELIVERY_MODEL"); fi
if [ "$DELIVERY_EFFORT" != default ]; then DELIVERY_EFFORT_ARGS=(--effort "$DELIVERY_EFFORT"); fi
if [ "$DELIVERY_MODEL" = default ] && [ "$DELIVERY_EFFORT" != default ]; then
  echo "UNSUPPORTED_TUPLE: native effort requires a pinned model" >&2
  exit 2
fi
```

For composed adapters, follow their per-truss truth table. Do not apply the
native `--effort` mapping to Pi (`--thinking`), OpenCode (variant in Model,
Effort must be `default`), Cursor (no effort flag), or another agent with a
different CLI. Every adapter must state the permission default/source; never
infer it from another truss. To build a safely quoted `--command` string from
the adapter's argv, quote every argv element with Bash `%q`:

```bash
shell_quote_argv() {
  local result="" item quoted
  for item in "$@"; do printf -v quoted '%q' "$item"; result+="${result:+ }$quoted"; done
  printf '%s' "$result"
}
DELIVERY_COMPOSED_ARGV="$(shell_quote_argv "${DELIVERY_AGENT_ARGV[@]}")"
```

## Preflight and co-installed skills

```bash
command -v "$DELIVERY_ORCA_CLI"
"$DELIVERY_ORCA_CLI" --version
"$DELIVERY_ORCA_CLI" status --json
"$DELIVERY_ORCA_CLI" orchestration run-list --json
DELIVERY_BA_SKILL="$DELIVERY_ROOT/.agents/skills/business-analyst/SKILL.md"
test -r "$DELIVERY_BA_SKILL"
DELIVERY_BA_DIR="$(dirname "$DELIVERY_BA_SKILL")"
test -r "$DELIVERY_BA_DIR/references/requirement-quality.md"
```

The co-installed BA skill is repository-local, not an `orca skills get` topic;
its references resolve relative to its directory.

## Run lifecycle

```bash
"$DELIVERY_ORCA_CLI" orchestration run-create --objective "<objective>" --json
"$DELIVERY_ORCA_CLI" orchestration run-current --json
"$DELIVERY_ORCA_CLI" orchestration run-list --json
"$DELIVERY_ORCA_CLI" orchestration run-use --id "$DELIVERY_RUN_ID" --json
```

Extract Run ID from `result.run.id`; `run-use` takes `--id`, not `--run`.

## Prompt file to self-contained Task

Orchestration requires a Task spec with **Target, Change, Constraints,
Ownership, Observable acceptance**. Live help defines the exact interface as
`task-create --spec <text>`. Keep the complete approved prompt in an in-worktree
file. Read it in the coordinator process and pass the bytes as one
argv element using `subprocess.run` (never shell-interpolate file content and
never replace the spec with a pointer-only instruction). If it exceeds OS
argument limits and this Orca build has no structured file/stdin transport,
stop with `UNSUPPORTED_PROMPT_TRANSPORT`.

```bash
python3 - "$DELIVERY_ORCA_CLI" "$DELIVERY_PROMPT_FILE" "$DELIVERY_TASK_TITLE" "$DELIVERY_RUN_ID" "$DELIVERY_RECEIPT_FILE" <<'PY'
import json, subprocess, sys
cli, prompt_path, title, run_id, receipt_path = sys.argv[1:]
with open(prompt_path, encoding="utf-8") as source:
    spec = source.read()
required = ("Target:", "Change:", "Constraints:", "Ownership:", "Observable acceptance:")
missing = [field for field in required if field not in spec]
if missing:
    raise SystemExit("Task spec missing fields: " + ", ".join(missing))
args = [cli, "orchestration", "task-create", "--spec", spec,
        "--task-title", title, "--run", run_id, "--json"]
result = subprocess.run(args, text=True, capture_output=True)
with open(receipt_path, "w", encoding="utf-8") as receipt:
    receipt.write(result.stdout)
    receipt.write(result.stderr)
if result.returncode:
    raise SystemExit(result.returncode)
data = json.loads(result.stdout)
def find_task_ids(value):
    if isinstance(value, dict):
        for child in value.values():
            yield from find_task_ids(child)
    elif isinstance(value, list):
        for child in value:
            yield from find_task_ids(child)
    elif isinstance(value, str) and value.startswith("task_"):
        yield value
ids = sorted(set(find_task_ids(data.get("result", data))))
if len(ids) != 1:
    raise SystemExit(f"expected exactly one task_ ID in receipt, got {ids!r}")
print(ids[0])
PY
```

Capture stdout as `DELIVERY_TASK_ID` and require prefix `task_`. Do not stage
the prompt. Repository-hosted runs remove it after worker return;
consumer-local runs retain it in the approved private run path.

## Immediate pre-dispatch check

Immediately before every start, retry or low-level dispatch, run:

```bash
python3 "$DELIVERY_SKILL_DIR/scripts/check-role-tuple.py" --agents "$DELIVERY_ROOT/AGENTS.md" --approved "$DELIVERY_APPROVED_ENVELOPE" --role "$DELIVERY_ROLE"
```

Continue only on exit 0. JSON records the matched normalized tuple; exit 2 names
the invalid item, authority and next action. This is read-only, uncached and does
not prove readiness or substitute for the approval receipt.

## Native worker launch

Only for an adapter marked `native` and an agent literally named under native
`--model` support by live help. Populate the optional arrays according to the
adapter's default mapping:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --agent "$DELIVERY_AGENT_ID" "${DELIVERY_MODEL_ARGS[@]}" "${DELIVERY_EFFORT_ARGS[@]}" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

Compare `launch.requested` and `launch.effective` in the receipt.

## Composed worker launch

Only for an adapter marked `composed`. The adapter builds
`DELIVERY_AGENT_ARGV` from the tuple and its truth table, then safely quotes it
as `DELIVERY_COMPOSED_ARGV` using the shell-quoted argv helper above. State
the permission default/source before launch.
Never send the task with an agent CLI `--prompt` before the supervised start;
`worker-start` injects the authoritative Task/preamble.

```bash
"$DELIVERY_ORCA_CLI" terminal create --worktree "$DELIVERY_WORKTREE_SELECTOR" --title "$DELIVERY_TERMINAL_TITLE" --command "$DELIVERY_COMPOSED_ARGV" --json
"$DELIVERY_ORCA_CLI" terminal wait --terminal "$DELIVERY_TERMINAL_HANDLE" --for tui-idle --timeout-ms 60000 --json
```

Extract handle from `result.terminal.handle`. Continue only if the exact
installed CLI receipt says `satisfied: true` and the visible TUI proves the
pin. A trust, login, or model picker is not readiness.

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --terminal "$DELIVERY_TERMINAL_HANDLE" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

Never pass `--agent`, `--model`, or `--effort` on this `--terminal` path.

## Completion wait and acknowledgement

Wait for a batch:

```bash
"$DELIVERY_ORCA_CLI" orchestration check --wait --types "worker_done,escalation,question" --timeout-ms 900000 --json
```

Process **every** returned message first: reply to questions, validate each
`worker_done` against its Dispatch, and decide ownership for each settled
terminal. Only then ack `result.deliveryId` (`delivery_...`), never an
individual `msg_...` ID:

```bash
"$DELIVERY_ORCA_CLI" orchestration check --ack "$DELIVERY_DELIVERY_ID" --json
```

A timeout is transport state: wait again or inspect the worker before judging
its state.

## Evidence and settlement

Inspect evidence:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-show --dispatch "$DELIVERY_DISPATCH_ID" --json
"$DELIVERY_ORCA_CLI" orchestration worker-read --dispatch "$DELIVERY_DISPATCH_ID" --source auto --json
```

After settlement, choose **exactly one** ownership action:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-release --dispatch "$DELIVERY_DISPATCH_ID" --json
# OR, when retaining the settled terminal was requested:
"$DELIVERY_ORCA_CLI" orchestration worker-retain --dispatch "$DELIVERY_DISPATCH_ID" --json
```

`worker-stop`/`worker-abandon` need positive recovery evidence. Do not close
until the reclaimable fleet query is empty:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-list --run "$DELIVERY_RUN_ID" --terminal-state reclaimable --json
```

## Retry and request recovery

Save the exact original recipe as a Bash argv array before executing it. Keep
stdout JSON and stderr separate, preserve exit status, and never run an
incomplete command to manufacture a request ID:

```bash
set -o pipefail
DELIVERY_ORIGINAL_ARGV=("$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --agent "$DELIVERY_AGENT_ID" "${DELIVERY_MODEL_ARGS[@]}" "${DELIVERY_EFFORT_ARGS[@]}" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json)
set +e
"${DELIVERY_ORIGINAL_ARGV[@]}" >"$DELIVERY_RECEIPT_FILE" 2>"$DELIVERY_RECEIPT_FILE.stderr"
DELIVERY_STATUS=$?
set -e
DELIVERY_REQUEST_ID="$(python3 - "$DELIVERY_RECEIPT_FILE" <<'PY'
import json, sys
with open(sys.argv[1], encoding="utf-8") as receipt:
    data = json.load(receipt)
error = data.get("error", {})
mutation = data.get("result", {}).get("mutation", {})
print(error.get("data", {}).get("orchestrationRequestId") or mutation.get("requestId") or "")
PY
)"
test -n "$DELIVERY_REQUEST_ID"
"$DELIVERY_ORCA_CLI" orchestration request-show --request "$DELIVERY_REQUEST_ID" --json
```

Use the ID only when present; `absent` is not proof that no mutation happened.
Replay only the same original argv with the receipt-supported
`--retry-request "$DELIVERY_REQUEST_ID"`. Retry a failed/stopped dispatch once
with the original Task, `--retry-of` Dispatch ID (`ctx_...`), and explicit
placement:

```bash
"$DELIVERY_ORCA_CLI" orchestration worker-start --task "$DELIVERY_TASK_ID" --retry-of "$DELIVERY_DISPATCH_ID" --worktree "$DELIVERY_WORKTREE_SELECTOR" --terminal "$DELIVERY_TERMINAL_HANDLE" --task-title "$DELIVERY_TASK_TITLE" --run "$DELIVERY_RUN_ID" --json
```

For fresh native placement, replace `--terminal` with the adapter's exact
`--agent`/pin arrays. Never retry while the plane considers the dispatch live.

## Launch decision

Normalize the Truss cell through `references/trusses.md`; follow the adapter's
`native`, `composed`, or `unresolved` classification. Native support requires
literal presence under live help's native model-support sentence; presence in
`--agent` examples is not enough. `unresolved` stops with
`UNSUPPORTED_LAUNCH`; never guess.
