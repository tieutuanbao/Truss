# Delivery execution

Read for this phase only. Paths written in backticks are relative to the skill
root; Markdown links are relative to this file. Shared routing and authority:
[Delivery](../SKILL.md). Follow applicable repository instructions first.

## Execution envelope

Before mutation, Control resolves deployment preferences against the live
truss surface, validates the consumer repository worktree's exact Git root and
baseline, starts Orca and verifies its required capabilities, records the
dirty baseline and exact-path ownership, and creates a feature branch when
starting on the default branch. The Orca Run must select that validated
worktree.

The candidate worktree defaults to the consumer checkout Control is running
in. A separate checkout — including an Orca-managed worktree under the
application's own workspace root — is neither required nor a default, and
Control never relocates the working copy on its own judgement. Control may use
one only when the approved envelope explicitly authorizes it: the design
contract names the candidate path, and the owner approves that path at gate 1
before any dispatch. Absent that authorization, deliver in the consumer
checkout.

Concurrent writers require separately approved isolated worktrees. This is a
Delivery ownership policy stricter than Orca's placement capability. Only when
the approved envelope names a worktree, a branch, and a base for each writer
may two tasks run at the same time; coupled work that shares a checkout or a
path is serialized in one worktree, one writer at a time. No two tasks in one
wave own the same path. The project-manager integrates accepted task commits
into the candidate before fresh exact-HEAD integration acceptance; integration
is not a concurrent write.

Every delivered shape freezes the same safeguards in its approved envelope:

- Prerequisites: every artifact the run must consume before candidate mutation,
  its current owner, the evidence that establishes it, and whether the approved
  task may create, repair, validate, or only consume it. A required create or
  repair action not granted in owned scope is `NEEDS_REPLAN`; path relevance or
  tool capability does not grant that authority. Include execution-plane
  identity evidence: CLI path, version, selected server/environment, capability
  evidence, and the live surface that advertised each capability.
- Scope: owned paths, forbidden scope, and protected pre-existing dirty paths.
- Proof: acceptance criteria, counterexample or permitted manual inspection,
  focused instruments, and applicable gates.
- Git target: the candidate worktree and its exact Git root, branch, baseline,
  base, remote, and pull-request target; state when publishing is not
  authorized. The candidate worktree is the consumer checkout unless the
  approved envelope explicitly authorizes a separate, Orca-managed checkout.
  State each concurrent writer's worktree, branch, and base when parallel work
  is authorized.
- Deployment: the complete role-keyed tuple (`role + truss + model + effort`),
  launch classification and exact adapter recipe, frozen from the managed
  block in the approved envelope. Record execution mode (`worker-start`
  supervised or low-level unsupervised Dispatch). Low-level unsupervised mode
  requires an explicit owner-approved envelope decision; otherwise it is
  forbidden.
- Authority: granted branch, owned-path commit, gate, push, and pull-request
  actions; local delivery explicitly excludes push and pull-request authority.
- Never authorized: merge, force-push, stash, reset, clean or other cleanup,
  and edits outside owned scope.

Shape changes representation and acceptance depth, not safeguards:

- Bounded: approved in-chat design and compact envelope; Implementer
  self-verification, plus a risk review only when Architect requires it.
- Architectural: durable business analysis and decision record at their
  authority routes plus a transient plan carrying the envelope; those authority
  artifacts are committed for a repository-hosted run and repository-local for an
  approved consumer-local run; Implementer self-verification, risk-routed task
  review, and release-triggered integration acceptance.

The repository workflow's durable-memory requirement still applies when
work spans sessions or needs recovery; neither shape duplicates progress.

Delivery stages and commits only contract-owned paths. It never stashes,
resets, cleans, or silently absorbs the user's existing changes. If a path
carries protected baseline changes and Delivery must also modify it, Control
pauses and presents the overlap: the user either moves or commits their
changes, grants ownership of that path to this run, or splits the path out of
scope. Delivery proceeds on the unpause decision; it does not combine
ownership silently.

## Orca is mandatory

Orca is the required dispatch and supervision plane. Mandatory identifies how
workers are launched and supervised, not a different repository in which the
candidate is implemented. Candidate mutation remains in the candidate worktree
recorded in the execution envelope — by default the consumer checkout — and no
Orca capability, convenience, or default workspace path relocates it.

Orca launches and supervises fresh native truss TUIs with the resolved truss,
model, and effort. Orchestration is a required Orca capability. `$delivery`
starts and preflights Orca before execution. If the CLI is missing, the runtime
cannot start, or a required capability is absent, Delivery stops — there is no
direct dispatch and no headless fallback of any kind.

### CLI identity and preflight

Use the configured execution-plane CLI, not a binary name guessed from the
OS. On Linux the official desktop installation registers `orca-ide`, because
`orca` is commonly the GNOME screen reader. Verify the configured CLI:

```bash
: "${DELIVERY_ORCA_CLI:?resolve using the installed orca-cli skill and command-recipes.md}"
command -v "$DELIVERY_ORCA_CLI"
"$DELIVERY_ORCA_CLI" --version
"$DELIVERY_ORCA_CLI" status --json
"$DELIVERY_ORCA_CLI" orchestration run-list --json
```

Before any worker dispatch, Control must also validate the consumer boundary:

```bash
DELIVERY_ROOT="$(git rev-parse --show-toplevel)"
DELIVERY_HEAD="$(git -C "$DELIVERY_ROOT" rev-parse HEAD)"
```

The co-installed `$business-analyst` skill resolves from the consumer root,
not from the execution plane: `"$DELIVERY_ROOT/.agents/skills/business-analyst/SKILL.md"`
must be readable, and its cited references resolve relative to that skill
directory. It is not an `orca skills get` topic.

A delivery requires a valid `HEAD`. If the consumer repository is an unborn
branch, stop before dispatch and ask for a baseline commit; do not stage
Truss-managed files, nested repositories, or unrelated user files. Resolve
the requested repository and worktree to this exact Git root and baseline
before creating a Run. Do not infer the target from a nested directory named
`source`, an existing current terminal, a stale Orca worktree registration, or
an Orca-managed checkout path the envelope did not authorize. The delivery
objective must name the consumer-relative path, not an absolute path copied
from a different repository.

A valid runtime does not prove worker readiness. The first worker dispatch
must be treated as a readiness probe: inspect the actual terminal output when
it fails. A start that does not reach readiness names the stage it failed at
in `failedStage`, its reason in `lastError`, and any terminal it created in
`residualResources`; read all three before choosing a recovery. A `lastError`
that names an Orca classification — a blocked trust prompt, for example —
classifies the failure and is not proof that the agent launched. If the
terminal shows a Claude, Antigravity, or other agent trust prompt, trust that
exact agent in that exact worktree; do not change a global Codex trust setting
or enable a bypass. Do not dispatch the next role until the probe reaches
agent readiness.

For a truss whose model cannot be pinned by `worker-start`, bootstrap trust
before creating the dispatch. Launch its configured interactive argv in an
ordinary Orca terminal, settle any exact-worktree trust prompt, then launch a
fresh terminal with the same pinned argv. Dispatch with `--terminal` only after
`terminal wait --for tui-idle` succeeds and the terminal banner names the
configured model. A trust-cleared terminal whose banner names another model is
not ready for that role.

For a composed launch, cite the terminal handle, requested model/effort argv,
and the readiness read showing the active model in the existing dispatch
handoff or Control report. Distinguish requested settings from settings the
TUI actually exposes; a model banner alone does not verify effective effort.
Null launch fields on a reused-terminal receipt prove neither an unpinned
launch nor the requested pin.

Before the first dispatch, resolve the configured execution-plane CLI and use
that same executable for the whole run. Record its resolved path and version
and the selected server/environment in the approved envelope. Read the live
`skills get orchestration` guide and relevant command help, including
`orchestration worker-start --help` and help for lifecycle or recovery actions
the run may need. Record the required capabilities, the live surfaces that
advertised them, and every requested role deployment value in the envelope.
Help is syntax/capability discovery only: the first applicable start/readiness
probe and its `launch.requested` versus `launch.effective` establish whether
the selected server and agent can serve the request. Missing required
capability or unproved readiness stops the delivery.

For Zcode, read `references/trusses/zcode.md` before launch. It selects native
`worker-start` when live help advertises support and uses its custom-terminal
procedure only as the conditional fallback; neither a desktop launcher nor a
headless prompt is a worker substitute.

If the configured CLI is
unavailable or is the GNOME accessibility application, stop and report the
installation boundary. Do not alias, shadow, or replace the system `orca`,
and do not fall back to direct worker execution.

### Pre-dispatch role-tuple check

Immediately before every dispatch attempt — first start, `--retry-of` retry,
and low-level dispatch — Control must read `AGENTS.md` from disk, not session
context, and compare the target role's complete normalized tuple
`role + truss + model + effort` with the tuple frozen in the approved envelope.
A read performed only at run start does not satisfy this check.

| Observed disk state | Required outcome |
| --- | --- |
| Exactly one block, one well-formed header-compatible table, exactly one target-role row, all cells resolvable, full tuple == frozen tuple | continue this dispatch with the frozen values |
| Any single field differs (role, truss, model, or effort) | stop before dispatch; report observed vs approved; dispatch with neither value; next-delivery eligibility only |
| No `delivery:begin` delimiter (block absent) | stop and report |
| No `delivery:end` delimiter (block unterminated) | stop and report |
| More than one `begin`, more than one `end`, or `end` before `begin` | stop and report |
| Malformed table, header, or row | stop and report |
| Target role row missing | stop and report |
| Target role row duplicated | stop and report |
| Role not in Delivery's nine-role set | stop and report |
| Managed block still contains retired `tester-debugger` | stop with `DELIVERY_ROLE_CONFIG_MIGRATION_REQUIRED` and route to `$delivery-setup` |
| Unresolvable value in any cell | stop and report |

The run is never silently continued on a stale value or switched to the new one.
The dispatch attempt stops, the discrepancy is reported, and only a later
delivery's envelope may adopt the changed tuple. This is a direct read-and-
compare — no new digest, database, cached parse, or migration. Run the read-only
stdlib helper against the current `AGENTS.md` and the immutable approved envelope:

```bash
python3 "$DELIVERY_SKILL_DIR/scripts/check-role-tuple.py" \
  --agents "$DELIVERY_ROOT/AGENTS.md" \
  --approved "$DELIVERY_APPROVED_ENVELOPE" --role "$DELIVERY_ROLE"
```

`DELIVERY_SKILL_DIR` is this installed skill's directory. The envelope contains
one frozen `Role | Truss | Model | Effort` table with the nine configured roles;
its rows are approval content, not a second mutable configuration. The helper
rereads both files on every invocation, normalizes only exact documented aliases,
and returns JSON on success or exit 2 with the violating item and next action.
A missing file, malformed table, retired role or tuple mismatch blocks dispatch.
A successful comparison proves configuration identity only, not user approval,
live capability or worker readiness. `delivery-setup` remains the sole writer of the managed block; Delivery is a
strict consumer.

### Launching a worker

Write the complete prompt to an untracked file **inside the worktree**.
Never place prompt bytes in a shell command string or interpolate file content
through shell syntax. When Orca exposes only `--spec <text>`, the coordinator
may read the file and pass its bytes as one structured argv element through a
process API (for example Python `subprocess.run`), after validating the
self-contained Task fields; do not replace the Task spec with a pointer-only
instruction. If the prompt exceeds OS argument limits and no structured
file/stdin transport exists, stop with `UNSUPPORTED_PROMPT_TRANSPORT`. A path outside the workspace can trigger a
second permission surface some trusses still prompt for even when tool
approval is skipped. Do not stage that file. In a repository-hosted run, delete
it after the worker returns: Control owns that dispatch artifact, not `git
clean`. In a consumer-local run the dispatch prompt and the handoff report live
under `.truss/delivery/runs/<run-key>/` beside the approved envelope and the
plan, and Control retains them until the owner deletes them explicitly: delivery
deletes no run artifact on its own. The handoff is likewise a file in the
worktree; its path travels as `payload.reportPath` and the message body stays
short. Prompt bytes are never interpolated into a shell command. When an Orca
command exposes only `--spec <text>`, use the structured-process transport in
`references/command-recipes.md`; the Task spec remains self-contained and
contains Target, Change, Constraints, Ownership, and Observable acceptance.
This forbids shell interpolation, not a single process argv element.

Before every launch, retry, or low-level dispatch, complete § Pre-dispatch
role-tuple check against the current bytes on disk.

The dispatch prompt carries the approved task, its scope, and required
evidence as a self-contained Task spec. It does not define the role
dispositions or the conditions for reaching one — those belong to this skill,
and a prompt that restates them narrows or contradicts them. Where the design contract states an
acceptance row — its instrument, its counterexample, and what was
observed — the prompt carries that row as written rather than a
restatement of it.

The `worker-start` receipt records `launch.requested` and `launch.effective`;
it does not establish that the worker can serve the request or that it
cannot. Launch a real interactive truss TUI for the role, with the model
and effort pinned from `AGENTS.md`. A visible shell running a headless
truss is not a TUI;
use `references/trusses.md` to select and read only the resolved truss's
launch reference before composing its argv. The reference states exactly one
launch classification, and that classification picks the recipe: `native`
dispatches through `worker-start --agent` with the model pin; `composed`
creates the pinned terminal from the reference's exact argv, proves readiness,
and dispatches with `--terminal` only; `unresolved` stops before dispatch with
`UNSUPPORTED_LAUNCH`. Every dispatch, wait, acknowledgement, and recovery
command is copied from `references/command-recipes.md` with placeholders
filled from receipts — argv is never composed from memory or from live help
alone.

Use low-level task, dispatch, injection, and returned-preamble actions only
where the live guide says the normal supervised start cannot express the
launch; follow that guide for the current task, dispatch, injection, preamble,
and supervision semantics.

**The tuple is named and provable on every dispatch.** A worker left on a
truss default is an unpinned environment: it lives in the truss's own config,
it changes without announcing itself, and the dispatch that relies on it
looks identical to one that pinned the same value deliberately. The pin
travels exactly once per launch, where the truss's classification puts it: a
native start carries `--model` and `--effort` on `worker-start`; a composed
start carries them on the terminal-create argv, and its `worker-start
--terminal` call carries neither. Every dispatch receipt or handoff then
records the full tuple (`launch.requested`/`launch.effective`, or the
composed argv plus its readiness proof). Repeating the flags on a
`--terminal` dispatch is a launch error, not a stronger pin.

When composing the TUI launch argv yourself, carry the execution plane's
configured permission default for that agent onto the composed argv;
composing argv is not a request for a different permission posture. Do not
add a sandbox the project did not pin.

`check --wait` on `worker_done,escalation,question` is the completion wait,
repeated past heartbeats until a settling message arrives for that dispatch —
a heartbeat ends one wait but settles nothing.
The worker reports once with `worker_done` and an `--outcome`.
Completion comes from the worker's own `worker_done`;
do not infer it from reading the worker's terminal.
`worker-read` is the bounded evidence read. After settlement or positive exit
evidence, choose and record exactly one applicable retain, release, stop, or
abandon outcome — absence is not exit evidence. When release is chosen, request
`worker-release` and inspect its result; success does not always mean the
terminal closed. Reused or external terminals may be retained without
process action. Control may close an exact retained terminal only after
verifying that this run created and still owns it, no work is active there,
and required evidence has been recovered or its gap reported. Do not close
user-owned, taken-over, or uncertain terminals.

Each delivery creates a new Run with an objective by using the live documented
creation verb — currently `orchestration run-create --objective` — then confirms
the coordinator's current Run with the live documented inspection verbs,
currently `run-current`/`run-list`, and refuses implicit reuse of another Run;
live guide/help owns the exact grammar. Control handles every
message in a returned batch, then acknowledges that batch with
`check --ack <deliveryId>` using its returned ID and the same Run and
coordinator. Otherwise the plane redelivers it. A wait's type filter controls
wakeup, not which messages belong to the returned batch.

Use Run-scoped fleet liveness, currently `worker-list` with its
`projection.liveness`, as the agent-level verdict. Use `worker-show` only for
its terminal/PTY observation (`observation.status`); never infer fleet liveness
from terminal-only observation.

When a mutation result is unknown, inspect the request record before any replay
— currently `request-show` — and preserve the original request identity through
the live retry mechanism, currently `--retry-request`.

A dispatch that does not reach `ready` is diagnosed by reading its terminal
and handling what is actually there. A retry names the failed task with
`--task` and the failed attempt with `--retry-of`; `--spec` would create a new
task instead of retrying this one, and `--retry-of` inherits no placement, so
the intended `--worktree` and exactly one of `--agent` (a fresh terminal) or
`--terminal` (the pinned terminal) are repeated explicitly. The pinned
terminal is reused only when that read shows the worker is not already
progressing; a `failed` receipt is not that showing. A `dispatched` receipt is
not evidence the worker is alive any more than a `failed` receipt is evidence
it is dead. A wait timeout is likewise a transport outcome, not a worker
outcome: re-enter the wait or read the terminal before concluding anything
about the worker. Retry is refused while the plane still considers the
dispatch live, whether or not the worker still is; the live terminal is
re-engaged instead. `--agent`, `--model`, and `--effort` all fail alongside
`--terminal`, because it reuses the agent it was launched with; reusing the
pinned terminal is therefore not an exception to naming the model and effort
on every dispatch, since that terminal was launched pinned. Control does not
route by an enumerated vendor dialog; `agent_prompt_blocked` and
`agent_prompt_stalled` do not distinguish separate recoveries.

### Specialists, not a consultation role

There is no consultation role. When a question — a domain judgement, a design
input, or a blocker — is better answered by a dedicated read-only dispatch,
Control routes it to the existing nine-role specialist that holds the relevant
expertise, using that specialist's deployment preference: an `architect` for a
technical judgement, a `detailed-designer` for a structural design judgement,
a `ba` for a product-intent question, a `tester` for a read-only testability
judgement, or a `visual-engineering` specialist for a visual judgement. Debugger
is excluded from this route because it requires explicit user authorization. The
specialist may reproduce, inspect, and report a diagnosis or expertise packet,
but it does not edit the candidate, commit, launch workers, or expand scope.
This is an exception, not a phase or mandatory round trip. Without a suitable
specialist, Control may answer from repository evidence.

### Escalate rather than guess

Stop and ask the human when: a result maps to no route or more than one; the
worker **failed** rather than returned a stop status — a non-zero exit with no
result, an exhausted quota, an authentication error — which is not `BLOCKED`
and must not be treated as one; Orca is unavailable or a required capability
is absent; an action needs authority policy reserves to the human; or the
same worker fails twice on the same input. Say what you know, what you tried,
and what the options are. Do not pick one. When Implementer exhausts one or two
approved remediation attempts on the same failing test, Control may recommend
Debugger, replan, another explicitly authorized repair, or stopping, but must not
dispatch Debugger automatically.
