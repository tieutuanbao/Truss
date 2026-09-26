---
name: delivery
description: Approved design, isolated seven-role implementation, independent tester-debugger acceptance, and exact-HEAD release via Orca. For Architectural work, public-contract changes, or explicitly requested deliveries. Ordinary bounded work uses repository workflow without delivery. Spikes investigate without starting a delivery run.
---

# Delivery

Delivery accepts a request that may still be vague, brings it to an approved
design contract, then runs isolated implementation, independent acceptance,
and release preparation on the Orca execution plane. It is a thin control
protocol, not an orchestrator, SDLC framework, or second source of Git state.

Read `AGENTS.md` for repository instructions and per-role truss/model/effort
pins. Follow repository-owned workflow, developer documentation, and native
validation entry points for required gates and artifact conventions; those
values need not be stored in `AGENTS.md`. Resolve the default branch from
repository policy or verified Git/forge metadata, not the current branch.
Record resolved paths and Git targets in the approved execution envelope.
If required authority remains absent or materially ambiguous, ask before
mutation; do not invent project policy.

For documentation or configuration work with no executable acceptance
instrument, use the manual inspection permitted under § Acceptance, naming
the responsible human reader and what that inspection cannot prove.
A missing gate command is not evidence that no gate is required, and manual
inspection does not waive an existing required gate.

## Two human gates

1. Approve the design contract before candidate mutation.
2. Accept the completed, independently accepted candidate: merge or publish a
   prepared pull request, or accept a local completion when the envelope
   authorizes no publish.

Delivery pauses outside those gates only for a scope or architecture change, a
destructive action, new authority, replan, or an unavailable required runtime.

## The control session

The current interactive session is the `project-manager`. It owns the approval
invariant, task boundaries, exception handling, dispatch supervision,
integration, recovery, and authorized release. It does not implement or fix
the candidate, and it does not accept a candidate it integrated or changed.
When the `ba`, `architect`, and `planner` roles are pinned it does not draft
the business analysis, the technical decisions, or the task plan either — it
supervises those dispatches and still owns the human gates. Delegation never
moves an approval: every gate stays with the project-manager.

Control does not prescribe question count, order, format, or skill-selection
precedence. Delivery owns the design outcome and approval boundary, not a
universal interview or planning method. The user, project, and truss
determine which design skills and native modes are active.

Native Plan Mode governs its enforced action constraints, question and plan
surfaces, artifact representation, and mode transitions. Compatible active
design skills may refine exploration and design methodology within those
constraints. When more than one applies, normal truss instruction and tool
precedence governs. One explicit approval satisfies Delivery's boundary for
the same design scope; a material scope change requires renewed approval.
Delivery does not select, activate, configure, emulate, or compose either
mechanism.

Neither one owns or can bypass the approval invariant: before candidate
mutation, Control must obtain explicit human approval of a design contract.
Plan Mode is defense in depth, not proof that requirements are clear or that
no mutation is possible.

Control surfaces unresolved uncertainty that could materially change intent,
acceptance, authority, public contract, architecture, or consequential risk.
The active design method may resolve other details from repository evidence
and convention, but material assumptions must be explicit.

## Shape: Spike, Bounded, Architectural

Use the smallest contract that safely holds the change. Risk may promote an
otherwise small change; diff size never demotes data-loss, security,
permission, or public-compatibility risk.

| Shape | Use | Artifact | Acceptance |
| --- | --- | --- | --- |
| Spike | Investigation only; no candidate is delivered | Approved probe and recommendation; no delivery run | none |
| Bounded | Small change, clear behaviour and ownership | Approved in-chat design and short execution envelope | one whole-change independent acceptance |
| Architectural | Multiple behaviours, public-contract change, architecture decision, or promoted risk | Approved decision record, task plan, execution envelope | task acceptance per task, then one integration acceptance |

An approved design contract states intent and success criteria; scope and
authority; affected public contract or architecture; consequential risks and
material assumptions; and a plausible counterexample or failure mode that
distinguishes correct behaviour from a present-but-wrong implementation. If no
executable instrument can discriminate the requirement, the contract names the
manual inspection and its limit.

### Roles

One session holds one role per run: a business analyst never implements or
accepts, an architect or planner never implements, an implementer never
accepts, and so on. `project-manager` is the current interactive session and is
never dispatched.

| Role | Owns | Dispatch |
| --- | --- | --- |
| `project-manager` | Coordination, required dev/ops preflight, approvals, dispatch, integration, recovery, authorized release | current session |
| `ba` | The concrete business analysis: goals, actors, flows, business rules, exceptions, scope, stable requirement IDs, acceptance and planner handoff | fresh dispatched worker |
| `architect` | Technical contracts, interfaces, boundaries, dependencies, risks, and the decision record mapping choices to business requirements | fresh dispatched worker |
| `planner` | The detailed task plan: inputs, outputs, exact path ownership, dependency DAG, commands, waves, integration and recovery | fresh dispatched worker |
| `implement` | Task-scoped code and documentation, tests, evidence, and commits | fresh dispatched worker |
| `visual-engineering` | UI/UX/visual advice and authorized UI implementation, including relevant code, docs, and tests | fresh dispatched worker when the task needs it |
| `tester-debugger` | Testing and diagnosis, authorized fixes, or independent acceptance in a separate fresh read-only session | fresh dispatched worker |

When the managed block pins `ba`, `architect`, and `planner`, Architectural work
is analyzed, decided, and planned by those fresh, separate sessions before
Control presents the design contract at gate 1. Bounded work may keep its
in-chat design, but Control may dispatch the same roles for it. Those sessions
never mutate the candidate: they produce or audit the contract, then their
dispatches end. Without those pins, Control drafts the contract itself and the
human gate is the only audit; missing BA, architect, or planner pins never move
their duties into the project-manager.

Code review is an activity inside `tester-debugger` acceptance, never a
separate role. A design audit, when one is needed, is a fresh non-author
`tester-debugger` session assessing acceptance and testability without editing.

Architectural work uses `templates/decision-record.md` (durable, with the
requirements-traceability mapping), `templates/business-analysis.md` (durable
product analysis), and `templates/plan.md` (transient, deleted in the release
commit, before the release-binding integration acceptance). For a
repository-hosted run, commit them before implementation begins — that commit is
the acceptance baseline.

For an approved consumer-local run (decision `0006`), the code baseline is the
exact pre-implementation commit, and business analysis, the approved decision
record, the approved execution envelope, and the transient plan are private
artifacts that must not be staged: they live under
`.truss/delivery/runs/<run-key>/`, with the approved envelope as an immutable
`approved-envelope.md` snapshot separate from mutable `plan.md` progress.
Mutable progress never changes the approved digest, and a change to scope or
envelope produces a new snapshot, a new receipt, and a new approval. Gate 1
approval binds the candidate root, the baseline commit, the snapshot path, and
its SHA-256, recorded in the local-only receipt
`.truss/delivery/approvals/<run-key>.md`, which carries the run-key, the
candidate root, the baseline commit, the approved-envelope path and its
SHA-256, the owner's approval note, and the approval date. The receipt is never
committed, and a one-line echo of the digest goes to the Orca run record as a
secondary copy only — the local receipt stays authoritative. An independent
accepting session obtains the snapshot and the receipt through an authorized
private handoff, recomputes the digest, and binds its verdict to the final
candidate HEAD and the approved envelope identity; a missing, unreadable,
mismatched, or unapproved snapshot or receipt blocks acceptance. Delivery never
deletes these artifacts on its own.

Before dispatching any work in a consumer-local run, Control verifies that the
candidate excludes `.truss/`, `.truss/core/`, the installed skill directories,
and the entrypoint files through `.git/info/exclude`, and records that check in
the envelope prerequisites. A local-only candidate that cannot establish those
rules stops instead of mutating the candidate.

The transient delivery plan is a per-run control artifact, not a durable
repository record. It does not live in `.truss/authority/plans/active/`. When the work
also needs memory that outlives the run — multi-session recovery, decisions
future work must inherit — that memory belongs in the durable decision record
and, for cross-session working memory, one execution plan under
`.truss/authority/plans/active/` owned by the repository workflow. Never keep the same
progress in both places: the transient plan holds per-run task state, the
durable record holds what survives the run.

### Acceptance

One acceptance table: each requirement, the instrument that proves it, the
plausible wrong implementation that instrument rejects, and where that
rejection was observed.

**An acceptance row is invalid until you have settled that its instrument
discriminates.** A row whose instrument passes both before and after the
change proves nothing and will be found during independent acceptance.
Baseline-red is insufficient by itself: an instrument observed red only because
the feature is absent says nothing about whether it can catch an implementation
that is present, runs, returns a pass, and is wrong.

Each row names a plausible wrong implementation its instrument rejects — one
that exists, runs, and returns a pass. "The feature is absent" does not
satisfy Counterexample. Where no counterexample exists, the row says so,
names the responsible human reader, and states what reading the diff cannot
prove.

Record what the available instruments cannot observe. Prefer the simplest
instrument that proves the contract. Documentation-only and configuration
rows may name reading or diff inspection as the instrument when no executable
one exists; they still state what that reading cannot prove.

**An acceptance row is also invalid until the executed instrument is shown to be
the approved one.** Naming a requirement or an instrument never satisfies a row
by itself: the accepting session compares what it executed against what the
approved contract named for that row, and reports both. A material substitution
is itself a contract mismatch and returns `CHANGES_REQUESTED` for Control to
reconcile; equivalence is demonstrated, never asserted. Treat as material any
change to fixture provenance, source artifact or version, environment or
platform, command cadence or repetition, the comparator, the attributes
observed, or the interface used to observe the result. Where an approved row
names artifact provenance, a version boundary, or a repetition count, the
executed instrument must supply exactly that.

Any claim about extent — an allowed scope, a count, a set of call sites —
states the command that produced it. Naming the command is not the
measurement: the command must have been run, and the claim reports its
output. Where the claim is a count or a scope, the instrument enumerates
rather than samples. A set comparison names both populations, any approved
exclusions, and the expected relation. A mismatch goes back to Control for
contract reconciliation; a dispatch prompt must not silently narrow the
approved requirement or its instrument.

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

Concurrent writers require separately approved isolated worktrees. Only when
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
  tool capability does not grant that authority.
- Scope: owned paths, forbidden scope, and protected pre-existing dirty paths.
- Proof: acceptance criteria, counterexample or permitted manual inspection,
  focused instruments, and applicable gates.
- Git target: the candidate worktree and its exact Git root, branch, baseline,
  base, remote, and pull-request target; state when publishing is not
  authorized. The candidate worktree is the consumer checkout unless the
  approved envelope explicitly authorizes a separate, Orca-managed checkout.
  State each concurrent writer's worktree, branch, and base when parallel work
  is authorized.
- Deployment: resolved truss, model, and effort for each dispatched role.
- Authority: granted branch, owned-path commit, gate, push, and pull-request
  actions; local delivery explicitly excludes push and pull-request authority.
- Never authorized: merge, force-push, stash, reset, clean or other cleanup,
  and edits outside owned scope.

Shape changes representation and acceptance depth, not safeguards:

- Bounded: approved in-chat design and compact envelope; no Architectural
  transient plan or separate task acceptance; one whole-change independent
  acceptance.
- Architectural: committed business analysis, decision record, and transient
  plan carrying the envelope, or — for an approved consumer-local run — the same
  envelope as private artifacts bound by an approval receipt; task acceptance
  per task, then integration acceptance.

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
DELIVERY_ORCA_CLI="${DELIVERY_ORCA_CLI:-orca-ide}"
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
it fails. Error labels such as `codex-trust-workspace` are Orca classifications,
not proof that Codex launched. If the terminal shows a Claude, Antigravity, or
other agent trust prompt, trust that exact agent in that exact worktree; do not
change a global Codex trust setting or enable a bypass. Do not dispatch the
next role until the probe reaches agent readiness.

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

For Zcode, read `references/trusses/zcode.md` before launch. Its standalone
TUI verification and custom-terminal dispatch procedure apply; neither a
desktop launcher nor a headless prompt is a worker substitute.

The preflight must verify the binary identity and required subcommands; a
successful `command -v orca` is not sufficient. If the configured CLI is
unavailable or is the GNOME accessibility application, stop and report the
installation boundary. Do not alias, shadow, or replace the system `orca`,
and do not fall back to direct worker execution.

### Launching a worker

Write the prompt to an untracked file **inside the worktree**. Never inline
it in a shell argument: prompts carry backticks, quotes and newlines, and a
shell argument mangles them. A path outside the workspace can trigger a
second permission surface some trusses still prompt for even when tool
approval is skipped. Do not stage that file. In a repository-hosted run, delete
it after the worker returns: Control owns that dispatch artifact, not `git
clean`. In a consumer-local run the dispatch prompt and the handoff report live
under `.truss/delivery/runs/<run-key>/` beside the approved envelope and the
plan, and Control retains them until the owner deletes them explicitly: delivery
deletes no run artifact on its own. The handoff is likewise a file in the
worktree; its path travels as `payload.reportPath` and the message body stays
short. `--spec` and `--body` are shell arguments, which this skill already
forbids for prompts.

The dispatch prompt carries the task, its scope and the evidence required.
It does not define the role dispositions or the conditions for reaching
one — those belong to this skill, and a prompt that restates them
narrows or contradicts them. Where the design contract states an
acceptance row — its instrument, its counterexample, and what was
observed — the prompt carries that row as written rather than a
restatement of it.

The `worker-start` receipt records `launch.requested` and `launch.effective`;
it does not establish that the worker can serve the request or that it
cannot. Launch a real interactive truss TUI for the role, with the model
and effort pinned from `AGENTS.md`. A visible shell running a headless
truss is not a TUI;
use `references/trusses.md` to select and read only the resolved truss's
launch reference before composing its argv.

**Name the model and effort on every dispatch.** A worker left on a truss
default is an unpinned environment: it lives in the truss's own config, it
changes without announcing itself, and the dispatch that relies on it looks
identical to one that pinned the same value deliberately. `--effort` on a
launch argv requires `--model`.

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
`worker-read` is the bounded evidence read. After settlement, request
`worker-release` and inspect its result; success does not always mean the
terminal closed. Reused or external terminals may be retained without
process action. Control may close an exact retained terminal only after
verifying that this run created and still owns it, no work is active there,
and required evidence has been recovered or its gap reported. Do not close
user-owned, taken-over, or uncertain terminals.

Each delivery opens its own Run on the execution plane rather than reusing
another's, so a stale report cannot settle a new wait. Control handles every
message in a returned batch, then acknowledges that batch with
`check --ack <deliveryId>` using its returned ID and the same Run and
coordinator. Otherwise the plane redelivers it. A wait's type filter controls
wakeup, not which messages belong to the returned batch.

A dispatch that does not reach `ready` is diagnosed by reading its terminal
and handling what is actually there. It is retried into that same terminal
with `--terminal` and `--retry-of` only when that read shows the worker is
not already progressing; a `failed` receipt is not that showing. A
`dispatched` receipt is not evidence the worker is alive any more than a
`failed` receipt is evidence it is dead. A wait timeout is likewise a
transport outcome, not a worker outcome: re-enter the wait or read the
terminal before concluding anything about the worker. Retry is refused while
the plane still considers the dispatch live, whether or not the worker still
is; the live terminal is re-engaged instead. `--model` and `--effort` cannot
combine with `--terminal`; that is not an exception to naming the model and
effort on every dispatch, because the terminal was launched pinned and the
retry reuses it rather than launching an unpinned one. Control does not route
by an enumerated vendor dialog; `agent_prompt_blocked` and
`agent_prompt_stalled` do not distinguish separate recoveries.

### Specialists, not a consultation role

There is no consultation role. When a question — a domain judgement, a design
input, or a blocker — is better answered by a dedicated read-only dispatch,
Control routes it to the existing seven-role specialist that holds the relevant
expertise, using that specialist's deployment preference: an `architect` for a
technical judgement, a `ba` for a product-intent question, a `tester-debugger`
for a read-only diagnosis, or a `visual-engineering` specialist for a visual
judgement. The specialist may reproduce, inspect, and report a diagnosis or
expertise packet, but it does not edit the candidate, commit, launch workers,
or expand scope. This is an exception, not a phase or mandatory round trip.
Without a suitable specialist, Control may answer from repository evidence.

### Escalate rather than guess

Stop and ask the human when: a result maps to no route or more than one; the
worker **failed** rather than returned a stop status — a non-zero exit with no
result, an exhausted quota, an authentication error — which is not `BLOCKED`
and must not be treated as one; Orca is unavailable or a required capability
is absent; an action needs authority policy reserves to the human; or the
same worker fails twice on the same input. Say what you know, what you tried,
and what the options are. Do not pick one.

## Implementation

Control creates a separate task only when that unit has its own test cycle
and an independent session could accept it while rejecting its neighbor.
Same-shaped mechanical changes are batched. Tightly coupled work stays one task
and one writer. Each independent task gets a fresh `implement` or
`visual-engineering` TUI.

An implementer reads the business analysis, the decision record, the plan, and
the baseline — not the design session's transcript. It owns only its task, runs
a focused acceptance instrument, and creates one task-scoped commit. For
behaviour with a deterministic executable test it uses TDD; the portable
invariant is smaller: observe a discriminating failure for the intended reason
before changing behaviour. A shell probe, parser fixture, or diff inspection
may be the correct instrument for configuration, documentation, generated
files, or environment-bound integration.

Concurrent implementers run only in separately approved isolated worktrees, one
writer per path, and coupled work is serialized. A writer never edits a path
another concurrent task owns.

The counterexample named in each acceptance row is observed red and cited.
That observation is not the behaviour's own absence: one is the feature
absent, the other is an implementation that is present, runs, returns a pass,
and is wrong. For a row using the permitted manual exception under
§ Acceptance, cite the completed inspection and its stated limit instead.

Implement the whole task before handing back. Stop and return `BLOCKED` or
`NEEDS_REPLAN` instead of a partial solution when the record contradicts the
code, the contract is ambiguous, work outside the task becomes necessary, an
existing test disproves an assumption, or the task cannot fit one session
even with normal context recovery. A task that exceeds one session's real
capacity is a decomposition failure; a worker context that fills mid-task is
ordinary recovery, handled through dispatch supervision — not a reason to
replan.

### Handoff

```text
Status:              DONE | BLOCKED | NEEDS_REPLAN
Session mode:        implementation | diagnose/fix | acceptance
Disposition:         ACCEPT | CHANGES_REQUESTED | BLOCKED (acceptance only)
Task:                approved task identifier or exact task heading
Truss:               name, model, effort, sandbox
Dispatch:            dispatch id
Repository:          exact Git root
Worktree:            exact worktree path
Branch:              branch name
Baseline:            approved baseline SHA
HEAD:                observed current HEAD SHA
Task commits:        task and remediation SHAs, or none with reason
Changed paths:       contract-owned paths changed by this task
Contract coverage:   each applicable acceptance row, quoted or identified by
                     its exact requirement
Proof fidelity:      per applicable row, in this order: `Approved instrument`,
                     `Executed instrument`, `Provenance`, `Substitutions`,
                     `Observability limits`. Write `Substitutions: none` only
                     after comparing the two instruments.
Verification:        commands, working directories, reported outcomes, and
                     evidence locators
Deviations from plan:
Residue:             remaining work or the check that returned empty
Git state:           observed status, including protected baseline changes
END OF HANDOFF
```

Write this handoff to the worktree file designated by Control and send its
path as `payload.reportPath`, as required under § Launching a worker; an
inline final message does not replace the file. `END OF HANDOFF` must be
the file's last line; a missing sentinel means the handoff may be truncated.
Resolve SHAs from Git, not from a planned commit.
Identify acceptance rows using existing identifiers or exact requirement
text; do not create another acceptance table or numbering system.

The worker identifies the commands it ran, their working directories,
reported outcomes, and available evidence locators. It does not transcribe
terminal output by hand or claim that its own account is independently
verified. If no recoverable locator is available, say so.

Control retrieves and cites available dispatch-bound evidence under
§ Evidence, including its stated limits. An accepting session still reproduces
the required instruments itself.

Under `Residue`, a claim of nothing left names the check that returned empty.
`Git state` distinguishes task changes from protected baseline changes;
a clean HEAD identity alone does not establish a clean working tree.

## Acceptance

Acceptance is testing and diagnosis performed by `tester-debugger`, and it is
the independent verdict on a candidate. Code review of the candidate is one of
its activities; it is never a separate role. The design contract was already
audited earlier — by the `architect` when pinned, otherwise by the human at
gate 1.

A `tester-debugger` task declares exactly one session mode:

- **diagnose/fix** reproduces a failure, finds the root cause, and makes an
  authorized fix. It owns a task-scoped commit like an implementer and returns
  the implementation handoff above, never `ACCEPT`. Any candidate edit
  invalidates every earlier acceptance and requires a different fresh
  acceptance session at the new HEAD.
- **acceptance** is a fresh, read-only session that reproduces the required
  instruments and returns a verdict. It never edits the candidate.

Acceptance independence is candidate independence: the accepting session did
not author, fix, plan, analyse, decide, advise, or integrate that candidate,
and does not edit it. The candidate author, fixer, planner, advisor, and the
project-manager that integrated it cannot accept it. Acceptance gets the
business analysis, the decision record (or Bounded design), the baseline, and
the diff. The phase adds no sandbox by default; `AGENTS.md` may pin one for a
concrete risk.

Acceptance depth is adaptive: Bounded work gets one independent whole-change
acceptance. Each Architectural task gets an independent acceptance. After all
tasks are integrated, a different fresh `tester-debugger` session performs one
integration acceptance of task interactions, complete-contract coverage,
deferred findings, candidate identity, and release readiness at the exact
final HEAD. Architectural task acceptance and final integration acceptance are
separate sessions. Only that final integration acceptance is release-binding
for Architectural work; Bounded work has no earlier task acceptance and no
duplicate integration acceptance.

No worker runs while acceptance of the same working tree runs. The working
tree and its gate surface are shared mutable state, and an accepting session
reproduces gates in that tree, so a concurrent edit makes another task's work
look like this one's result.

**Reproduce, do not accept.** Run the gates yourself. A claim you did not
reproduce is not evidence. The accepting session observes the counterexample
discriminate for itself — an implementation that is present, runs, returns a
pass, and is wrong. An instrument red only because the behaviour was
absent is not that observation. For a row using the permitted manual
exception under § Acceptance, verify that the named human's inspection was
completed and report its stated limit.

Classify findings: **Blocking** — contract failure, regression, data or
security risk. **Important** — missing required behaviour, test, or
reconciliation. **Minor** — useful, does not block. **Out of scope** —
recorded, not absorbed.

Return exactly one disposition: `ACCEPT`, `CHANGES_REQUESTED`, or `BLOCKED`.
State what acceptance did not verify — what it did not reproduce or read. A
contradiction you cannot resolve is `CHANGES_REQUESTED`. Do not open
remediation over wording when deterministic checks already prove the contract.

### Remediation

One pass is one remediation pass per finding, not per acceptance. For an
in-contract `CHANGES_REQUESTED` finding, the **original implementer**, or the
`tester-debugger` fix session that authored the change, verifies it, fixes the
root cause, reruns the affected instruments and closure gates, and writes a
separate remediation commit — this is the single original-party remediation.
A fix session never accepts its own fix. Control owns the plan and the decision
record for the whole run, including remediating findings inside them.
Amending them is not implementing the candidate. The fixed candidate is
accepted only by a **different fresh acceptance session at the new exact
HEAD**; the earlier verdict does not carry across the mutation. If that scoped
acceptance does not accept, Control routes to `REPLAN_OR_SPLIT` — it does not
start another repair loop. `BLOCKED` preserves the candidate and escalates the
unresolved dependency or authority question to Control separately from a
failed worker process; it is not remediated by the original implementer. A
fresh replacement acceptance session is used only when the original accepting
session is unavailable or contested, and for the Architectural integration
acceptance.

## Release

Control performs release with native Git and forge tools; release dispatches
no LLM worker and makes no post-acceptance candidate edit.

1. Complete implementation and, for Architectural work, its task acceptance.
2. Reconcile owning documentation, move anything durable out of the transient
   plan, and commit the complete candidate. For a consumer-local run, nothing
   private is committed and no run artifact is deleted here: the transient plan
   stays on disk, because delivery does not delete run artifacts on its own.
3. Run the focused instruments and project closure gates on exact HEAD.
4. If the envelope authorises publishing: push the feature branch and create
   or update a draft pull request, and run the applicable integration
   acceptance while remote checks run on that same HEAD.
5. Require the applicable integration acceptance's `ACCEPT` — plus required
   remote checks green when the envelope authorises publishing — bound to that
   exact HEAD.
6. Report for the second human gate: mark the pull request ready for human
   merge, or present the locally completed, accepted candidate for human
   acceptance.

Evidence is revision-bound: a verdict earned on one HEAD validates only that
HEAD. Any candidate mutation after the applicable integration acceptance
invalidates that verdict; Control reruns the affected gates and acceptance on
the new exact HEAD. Affected gates are those that can observe the change class;
a project may name that subset.

A local delivery ends at step 6 with the candidate committed on its branch,
gates green, and the integration acceptance accepted on that exact HEAD.
Publishing later repeats steps 4–6 on the current HEAD; it never reuses a
verdict from an earlier revision.

Delivery never merges, force-pushes, or publishes outside the approved
target and authority. If project policy cannot publish work in progress,
Delivery delays the push and pull request until the applicable acceptance
accepts.

### Maintenance log

Maintenance logging is machine-local and opt-in at `~/.truss/delivery-log`.
Never create the directory or file; a missing path is skipped silently.
After acceptance and all required checks are green, append one line only
when the path exists, using `references/maintenance-log.md`.
Never use this log for routing, recovery, or runtime decisions.
An append failure warns but does not invalidate or block release.

## Evidence

Evidence is a property of a dispatch, not a skill-owned journal. Control asks
Orca for the dispatch-bound command, output, and outcome. Orca owns transcript
selection and cursor mechanics. For each cited read, report its source,
exactness, completeness, and any fallback or clipping the response identifies.

When required evidence is unavailable, Control must name the missing item
and label the worker's account unverified. Neither a clipped terminal tail
nor worker_done proves an unseen command, output, or counterexample check.
Cite any independent reproduction separately, with its actor and revision;
it does not establish that the worker performed the claimed check. Keep
unresolved required evidence visible rather than declaring evidence complete.

Durable candidate and release facts come from Git, CI, and the pull-request
state. This skill duplicates none of those stores.

## Failure and recovery

Recovery uses Orca records, Git, CI, and pull-request state — never inferred
from an ambiguous, missing, or merely transport-level outcome.

A failed readiness probe does not authorize a blind retry. Confirm the exact
agent/worktree trust prompt, the consumer Git root, and the selected baseline
first. Released terminals may leave child worktrees or archived resources;
inspect Orca resource accounting before deciding whether cleanup or a new Run
is safe.

| Failure | Disposition |
| --- | --- |
| In-contract implementation defect | Original implementer or fix session remediates; a different fresh session accepts the new HEAD |
| Scoped remediation acceptance does not accept | `REPLAN_OR_SPLIT` |
| Scope or architecture must change | Return to the design gate |
| New authority or destructive action is required | Ask the human |
| Orca or a required capability is unavailable | Stop; no headless fallback |
| The candidate location would move out of the consumer checkout | Not a Control decision: present it as a gate 1 decision with the named path, or deliver in the consumer checkout when the envelope authorizes no relocation |
| Truss fails or evidence is insufficient | Preserve the candidate, report the native outcome and disposition |
| Dispatch wait times out or receipt is ambiguous | Treat as transport-unknown: re-enter the wait or read the terminal; retry only into the same pinned terminal with `--retry-of` when it is not progressing |
| Architect rejects the drafted contract | Control routes findings back to `ba`, `architect`, or `planner` when those roles are pinned; only without those pins may Control redraft in-session. Gate 1 is not presented until the contract is settled |
| Control session is interrupted | Resume from Git state, the durable decision record or execution plan, and Orca run records; re-verify a live dispatch before re-engaging it; never start a competing implementer or accepting session for work already in flight |
| Idempotent release step is interrupted | Verify Git and pull-request state, then resume |

## Changing this skill

Only when the same failure recurs under the current contract — one incident
is not policy. Before adding a rule, check whether a mechanism can enforce
the fact instead. A human decides whether to promote a proposal; this skill
never mutates itself, `AGENTS.md`, or project instructions from telemetry.

## Language

Repository artifacts are English. Conversation follows the user. Enum values,
paths, commands, branch names and SHAs are never translated.
