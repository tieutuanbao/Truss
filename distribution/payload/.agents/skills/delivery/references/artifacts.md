# Delivery artifacts

Read for this phase only. Paths written in backticks are relative to the skill
root; Markdown links are relative to this file. Shared routing and authority:
[Delivery](../SKILL.md). Follow applicable repository instructions first.

Classify every Delivery artifact by purpose and lifecycle: content required
after run closure is repository authority; content used only to dispatch,
communicate, hand off, bind an approval, or track ephemeral run state is run
coordination. Filename alone is not decisive. Business analysis belongs under
`.truss/authority/product/`; decision records under
`.truss/authority/decisions/`; detailed design and its audit under
`.truss/authority/design/<design-key>/`; and durable execution plans under
`.truss/authority/plans/active/`, then `.truss/authority/plans/completed/`. The
transient plan, approved envelope, approval receipt, dispatch prompts, worker
handoffs, run
state, gate output, and operational maintenance evidence belong under
`.truss/delivery/`. A mixed-purpose artifact is split between a run-local
artifact and the owning authority record; missing or ambiguous classification,
persistence, or retrieval authority returns `NEEDS_INPUT`.

Reuse the owning record before creating another file: before creating an
artifact, identify its purpose, its owning record, and its run or dispatch
identity, and create a separate file only for a distinct purpose, a distinct
dispatch attempt, or an immutable approval or evidence snapshot. Name new
dispatch artifacts `<task-key>-<dispatch-key>-<kind>.<ext>`, distinguishing a
prompt, a handoff, and each receipt type, and link each approval snapshot and its
digest from the existing run record. Do not overwrite immutable evidence, and do
not create a second canonical copy of an authority record. Naming and placement
never grant staging, retention, or deletion authority: a conflict with a
repository's accepted evidence policy returns `NEEDS_INPUT`.

For a repository-hosted run, commit approved durable authority in the baseline
before implementation. If an ignore rule matches an approved authority path, use
explicit path-scoped `git add -f -- <path>` only when the approved envelope
grants staging and commit authority. Run coordination uses
`.truss/delivery/runs/<run-key>/` and
`.truss/delivery/approvals/<run-key>.md`; a transient plan is committed only
where the approved baseline requires it and is removed before release after
durable content has moved to its authority owner.

For an approved consumer-local run (decision `0006`), the code baseline is the
exact pre-implementation commit. Durable authority stays repository-local under
`.truss/authority/` and is never staged, committed, or force-added. Run
coordination stays private under `.truss/delivery/runs/<run-key>/`, with the
approved envelope as an immutable `approved-envelope.md` snapshot separate from
mutable `plan.md`, and the approval receipt at
`.truss/delivery/approvals/<run-key>.md`. Mutable progress never changes the
approved digest; a scope or envelope change produces a new snapshot, a new
receipt, and a new approval. The local receipt stays authoritative, and a
one-line digest echo in the Orca run record is secondary evidence only.

Repository-hosted acceptance checks each authority artifact with
`git cat-file -e <approved-baseline-commit>:<authority-path>` and
`git show <approved-baseline-commit>:<authority-path> | sha256sum`. Consumer-local
acceptance receives each absolute candidate-local authority path and SHA-256
through an authorized Orca handoff, then runs `test -r <absolute-path>` and
`sha256sum <absolute-path>`. A missing path, unreadable bytes, digest mismatch,
wrong candidate root, or absent authorization returns `NEEDS_INPUT` and blocks
acceptance. The approval envelope and receipt follow the private handoff route
required by decision `0006`; a missing, unreadable, mismatched, or unapproved
snapshot or receipt also blocks acceptance. Delivery never deletes these run
artifacts on its own.

Before dispatching any work in a consumer-local run, Control verifies that the
candidate excludes `.truss/`, `.truss/core/`, the installed skill directories,
and the entrypoint files through `.git/info/exclude`, and records that check in
the envelope prerequisites. A local-only candidate that cannot establish those
rules stops instead of mutating the candidate.

The transient delivery plan is a per-run control artifact, not a durable
repository record. It does not live in `.truss/authority/plans/active/`. When the
work also needs memory that outlives the run — multi-session recovery, decisions
future work must inherit — that memory belongs in the durable decision record
and, for cross-session working memory, one execution plan under
`.truss/authority/plans/active/` owned by the repository workflow. Never keep
the same canonical content in both places: the transient plan holds per-run task
state, the durable record holds what survives the run.

### Acceptance

The planner supplies, for every task, explicit acceptance criteria and test
cases mapped to those criteria. Each criterion states an observable pass
condition. Each test case states its preconditions, action or exact command,
expected result, and mapped criterion. The set must be complete enough that an
implementer can build the task and an independent reviewer can judge it without
creating new requirements.

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
