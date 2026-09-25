# Private Envelope Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development
> (recommended) or executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The delivered delivery guidance carries the private accepted-envelope
contract of decision 0006, guarded by an executable structural check.

**Architecture:** Guidance-only change. No Rust, no installer, no payload file
added or removed. Three tasks each add a rule set to the existing structural
checker `tests/delivery-role-contract.sh`, watch it fail against the current
guidance, then update the guidance it guards, then commit both together.

**Tech Stack:** Bash + Python 3 checker harness already in the repository
(`tests/delivery-role-contract.sh`, 426 lines), Markdown guidance.

**Spec:** `.truss-core/docs/decisions/0006-private-accepted-envelope-binding.md`
and `.truss-core/docs/decisions/0007-source-and-installed-separation.md` §9.
The plan argues from those records; the executor reads both.

## Global Constraints

- No installed-state schema change, no `.truss-core/` rename, no Rust behavior
  change, no new payload file. `scripts/delivery-install-files.txt` keeps 18
  paths, so the manifest-count expectation in `crates/truss/tests/cli_lifecycle.rs`
  does not move.
- The two copies of the plans README stay byte-identical while both exist:
  `.truss-core/docs/plans/README.md` and
  `crates/truss/assets/.truss-core/docs/plans/README.md`. Embedding reads the
  `crates/truss/assets/` copy for this file
  (`crates/truss/src/infrastructure/embedded_distribution.rs:119`), while other
  payload files are read from the root tree; editing only one silently changes
  what consumers receive.
- The structural checker proves documentation shape, not future agent
  obedience. Every rule below is a presence or absence assertion on text. State
  that limit in the acceptance record; never present it as behavior proof.
- `.truss/` is a new local-only area and the installer does not yet exclude it.
  Until the source/installed separation lands, a consumer-local run must add
  `/.truss/` to the exclude file itself, and the guidance must say so.
- Every task ends with its own commit.

---

## File Structure

| File | Responsibility in this plan |
| --- | --- |
| `tests/delivery-role-contract.sh` | Structural checker. Gains an `envelope` subcommand with rules R8-R12 and their isolated negative fixtures. |
| `.agents/skills/delivery/SKILL.md` | Delivered delivery protocol. Gains the consumer-local branch: private artifacts, immutable snapshot, receipt, fail-closed acceptance, exclude prerequisite. |
| `.agents/skills/delivery/templates/plan.md` | Transient plan template. Gains the private path and the never-committed rule. |
| `.agents/skills/delivery/templates/business-analysis.md` | BA template. Gains the consumer-local visibility rule. |
| `.agents/skills/delivery/templates/decision-record.md` | Decision record template. Gains the consumer-local visibility rule. |
| `.truss-core/docs/WORKFLOW.md` | Repository workflow. Gains the private-artifact boundary. |
| `.truss-core/docs/plans/README.md` + `crates/truss/assets/.truss-core/docs/plans/README.md` | Durable-plan guidance, kept byte-identical. Gains the transient-path distinction. |

Rules use R8-R12; the checker currently uses R1, R2, R3, R6, R7.

---

## Task 1: The delivery skill carries the private-envelope contract

**Files:**
- Modify: `tests/delivery-role-contract.sh` (add `PATH` keys, `check_envelope`, dispatch, fixtures)
- Modify: `.agents/skills/delivery/SKILL.md`

**Interfaces:**
- Consumes: the harness helpers `check`, `pos`, `neg`, and the `read(key)` /
  `fail(rule, message)` functions inside the heredoc at
  `tests/delivery-role-contract.sh:80-352`.
- Produces: rule ids `delivery-role-contract R8` and
  `delivery-role-contract R9`; checker subcommand `envelope`; environment
  variables `CONTRACT_WORKFLOW`, `CONTRACT_PLANS_README`,
  `CONTRACT_ASSETS_PLANS_README`.

- [ ] **Step 1: Add the new checker paths, rules, and dispatch**

Inside the `contract.py` heredoc, extend the `PATH` dictionary (currently ends
with `"authority": ...`):

```python
    "workflow": os.environ.get(
        "CONTRACT_WORKFLOW", os.path.join(ROOT, ".truss-core/docs/WORKFLOW.md")),
    "plans_readme": os.environ.get(
        "CONTRACT_PLANS_README",
        os.path.join(ROOT, ".truss-core/docs/plans/README.md")),
    "assets_plans_readme": os.environ.get(
        "CONTRACT_ASSETS_PLANS_README",
        os.path.join(ROOT, "crates/truss/assets/.truss-core/docs/plans/README.md")),
```

Insert these rules immediately before `def main():`:

```python
PRIVATE_RUN_DIR = ".truss/delivery-runs/<run-key>/"
APPROVAL_RECEIPT = ".truss/authority/approvals/<run-key>.md"
SNAPSHOT_FILE = "approved-envelope.md"

# Decision 0006 terms the delivery skill must carry for a consumer-local run.
R8_REQUIRED = [
    PRIVATE_RUN_DIR,
    APPROVAL_RECEIPT,
    SNAPSHOT_FILE,
    "consumer-local",
    "must not be staged",
    "blocks acceptance",
    "info/exclude",
]

# The unconditional commitment of the transient plan, superseded for a
# consumer-local run by decision 0006.
R9_FORBIDDEN = "Commit them before\nimplementation begins"


def check_envelope():
    skill = read("delivery")
    if skill is None:
        return
    for token in R8_REQUIRED:
        if token not in skill:
            fail("delivery-role-contract R8",
                 "delivery SKILL.md does not name %r; decision 0006 requires the run "
                 "path, the approval receipt, the immutable snapshot, the exclude "
                 "prerequisite, and the fail-closed acceptance rule" % token)
    if R9_FORBIDDEN in skill:
        fail("delivery-role-contract R9",
             "delivery SKILL.md still commits the transient plan unconditionally; "
             "decision 0006 makes that conditional on a consumer-local run")
```

Dispatch it:

```python
    if command in ("envelope", "all"):
        check_envelope()
```

- [ ] **Step 2: Add the negative fixtures**

Append before the summary lines (`echo "== delivery-role-contract summary:`):

```bash
python3 - "$REPO/.agents/skills/delivery/SKILL.md" "$WORK/ng6-skill.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(
    text.replace(".truss/authority/approvals/<run-key>.md", "the approval receipt", 1))
PY
neg "a skill that drops the approval receipt path is rejected" "R8" \
  env CONTRACT_DELIVERY_SKILL="$WORK/ng6-skill.md" python3 "$CONTRACT" envelope

python3 - "$REPO/.agents/skills/delivery/SKILL.md" "$WORK/ng7-skill.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(
    text + "\nCommit them before\nimplementation begins — that commit is the acceptance baseline.\n")
PY
neg "a skill that commits the transient plan unconditionally is rejected" "R9" \
  env CONTRACT_DELIVERY_SKILL="$WORK/ng7-skill.md" python3 "$CONTRACT" envelope

pos "the repository delivery skill carries the consumer-local envelope contract" \
  python3 "$CONTRACT" envelope
```

- [ ] **Step 3: Run the checker and confirm the repo guidance is red**

Run: `bash tests/delivery-role-contract.sh`
Expected: FAIL carrying `delivery-role-contract R8` for each missing token, and
the two new `neg` fixtures report `ok`. Capture the exact output verbatim for the
acceptance record. If the `neg` fixtures report `FAIL`, the rule does not
discriminate: fix the rule before touching `SKILL.md`.

- [ ] **Step 4: Update the Architectural artifacts paragraph**

Replace this text in `.agents/skills/delivery/SKILL.md`:

```text
Architectural work uses `templates/decision-record.md` (durable, with the
requirements-traceability mapping), `templates/business-analysis.md` (durable
product analysis), and `templates/plan.md` (transient, deleted in the release
commit, before the release-binding integration acceptance). Commit them before
implementation begins — that commit is the acceptance baseline.
```

with:

```text
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
`.truss/delivery-runs/<run-key>/`, with the approved envelope as an immutable
`approved-envelope.md` snapshot separate from mutable `plan.md` progress. Gate 1
approval binds the candidate root, the baseline commit, the snapshot path, and
its SHA-256, recorded in the local-only receipt
`.truss/authority/approvals/<run-key>.md`. An independent accepting session
obtains the snapshot and the receipt through an authorized private handoff,
recomputes the digest, and binds its verdict to the final candidate HEAD and the
approved envelope identity; a missing, unreadable, mismatched, or unapproved
snapshot or receipt blocks acceptance. Delivery never deletes these artifacts on
its own.

Before dispatching any work in a consumer-local run, Control verifies that the
candidate excludes `.truss/`, `.truss-core/`, the installed skill directories,
and the entrypoint files through `.git/info/exclude`, and records that check in
the envelope prerequisites. A local-only candidate that cannot establish those
rules stops instead of mutating the candidate.
```

- [ ] **Step 5: Update the shape and release sections**

Replace, in the shape list:

```text
- Architectural: committed business analysis, decision record, and transient
  plan carrying the envelope; task acceptance per task, then integration
  acceptance.
```

with:

```text
- Architectural: committed business analysis, decision record, and transient
  plan carrying the envelope, or — for an approved consumer-local run — the same
  envelope as private artifacts bound by an approval receipt; task acceptance
  per task, then integration acceptance.
```

And in § Release, step 2, replace:

```text
2. Reconcile owning documentation, move anything durable out of the transient
   plan, and commit the complete candidate.
```

with:

```text
2. Reconcile owning documentation, move anything durable out of the transient
   plan, and commit the complete candidate. For a consumer-local run, nothing
   private is committed and no run artifact is deleted here: the transient plan
   stays on disk, because delivery does not delete run artifacts on its own.
```

- [ ] **Step 6: Run the checker and confirm green**

Run: `bash tests/delivery-role-contract.sh`
Expected: `== delivery-role-contract summary: N ok, 0 failed ==` with the two new
negative fixtures still `ok`.

- [ ] **Step 7: Commit**

```bash
git add tests/delivery-role-contract.sh .agents/skills/delivery/SKILL.md
git commit -m "feat(delivery): carry the private accepted-envelope contract"
```

---

## Task 2: The delivery templates carry the private-envelope contract

**Files:**
- Modify: `tests/delivery-role-contract.sh` (`R10` rules and fixtures)
- Modify: `.agents/skills/delivery/templates/plan.md`
- Modify: `.agents/skills/delivery/templates/business-analysis.md`
- Modify: `.agents/skills/delivery/templates/decision-record.md`

**Interfaces:**
- Consumes: rule ids and the `envelope` subcommand from Task 1.
- Produces: rule id `delivery-role-contract R10`.

- [ ] **Step 1: Add the R10 rules**

Append to `check_envelope` (before the closing of the function):

```python
    plan = read("plan")
    ba = read("ba")
    decision = read("decision")
    for key, text in (("plan", plan), ("ba", ba), ("decision", decision)):
        if text is None:
            return
    if PRIVATE_RUN_DIR not in plan or "never committed" not in plan:
        fail("delivery-role-contract R10",
             "templates/plan.md does not name %r and the never-committed rule; "
             "decision 0006 fixes the private run path" % PRIVATE_RUN_DIR)
    for key, text in (("ba", ba), ("decision", decision)):
        if "consumer-local" not in text or "never committed" not in text:
            fail("delivery-role-contract R10",
                 "templates/%s.md does not state the consumer-local "
                 "never-committed rule of decision 0006" % key)
```

- [ ] **Step 2: Add the negative fixtures**

```bash
python3 - "$REPO/.agents/skills/delivery/templates/plan.md" "$WORK/ng8-plan.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(text.replace(".truss/delivery-runs/", "the run directory"))
PY
neg "a plan template without the private run path is rejected" "R10" \
  env CONTRACT_PLAN_TEMPLATE="$WORK/ng8-plan.md" python3 "$CONTRACT" envelope

python3 - "$REPO/.agents/skills/delivery/templates/decision-record.md" "$WORK/ng9-decision.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(text.replace("never committed", "usually committed"))
PY
neg "a decision template that drops the never-committed rule is rejected" "R10" \
  env CONTRACT_DECISION_TEMPLATE="$WORK/ng9-decision.md" python3 "$CONTRACT" envelope
```

- [ ] **Step 3: Run the checker and confirm red**

Run: `bash tests/delivery-role-contract.sh`
Expected: FAIL carrying `delivery-role-contract R10` for the plan, BA, and
decision templates; the new fixtures `ok`. Capture verbatim.

- [ ] **Step 4: Update `templates/plan.md`**

Replace lines 3-5:

```text
Transient. Deleted in the release commit, before the release-binding review,
after anything durable has moved into the decision record or the documents
that own the changed paths.
```

with:

```text
Transient. For a repository-hosted run it is deleted in the release commit,
before the release-binding review, after anything durable has moved into the
decision record or the documents that own the changed paths. For an approved
consumer-local run it is never committed: it lives at
`.truss/delivery-runs/<run-key>/plan.md`, its immutable counterpart is
`approved-envelope.md` in the same directory, and delivery does not delete it.
```

- [ ] **Step 5: Update the BA and decision templates**

In `templates/business-analysis.md`, after the sentence ending
In `templates/business-analysis.md`, after the paragraph that ends
`must not be handed to the planner.`, insert:

```text
For an approved consumer-local run this analysis is a private artifact: it is
never committed and never staged. Write it under
`.truss/delivery-runs/<run-key>/` and obtain approval through the local receipt
`.truss/authority/approvals/<run-key>.md` instead of through a baseline commit.
```

In `templates/decision-record.md`, after the paragraph that ends
`before it exists.`, insert the same paragraph with `decision record` in place of
`analysis`.

- [ ] **Step 6: Run the checker and confirm green**

Run: `bash tests/delivery-role-contract.sh`
Expected: `0 failed`.

- [ ] **Step 7: Commit**

```bash
git add tests/delivery-role-contract.sh \
  .agents/skills/delivery/templates/plan.md \
  .agents/skills/delivery/templates/business-analysis.md \
  .agents/skills/delivery/templates/decision-record.md
git commit -m "feat(delivery): make the transient templates consumer-local aware"
```

---

## Task 3: The owning documents agree, and drift is guarded

**Files:**
- Modify: `tests/delivery-role-contract.sh` (`R11`, `R12` rules and fixtures)
- Modify: `.truss-core/docs/WORKFLOW.md`
- Modify: `.truss-core/docs/plans/README.md`
- Modify: `crates/truss/assets/.truss-core/docs/plans/README.md`

**Interfaces:**
- Consumes: `CONTRACT_WORKFLOW`, `CONTRACT_PLANS_README`,
  `CONTRACT_ASSETS_PLANS_README` from Task 1.
- Produces: rule ids `delivery-role-contract R11` and
  `delivery-role-contract R12`.

- [ ] **Step 1: Add the R11 and R12 rules**

Append to `check_envelope`:

```python
    workflow = read("workflow")
    plans_readme = read("plans_readme")
    assets_readme = read("assets_plans_readme")
    if workflow is None or plans_readme is None or assets_readme is None:
        return
    if PRIVATE_RUN_DIR not in workflow or "never committed" not in workflow:
        fail("delivery-role-contract R11",
             "WORKFLOW.md does not state the private run path and the "
             "never-committed rule of decision 0006")
    if PRIVATE_RUN_DIR not in plans_readme:
        fail("delivery-role-contract R11",
             "the plans README does not distinguish the transient private path "
             "%r from the durable plan location" % PRIVATE_RUN_DIR)
    if plans_readme != assets_readme:
        fail("delivery-role-contract R12",
             "the two plans README copies differ; embedding reads the "
             "crates/truss/assets copy, so divergence changes what consumers "
             "receive without either file looking wrong")
```

- [ ] **Step 2: Add the negative fixtures**

```bash
python3 - "$REPO/.truss-core/docs/WORKFLOW.md" "$WORK/ng10-workflow.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(text.replace("never committed", "committed with the candidate"))
PY
neg "a workflow that drops the never-committed rule is rejected" "R11" \
  env CONTRACT_WORKFLOW="$WORK/ng10-workflow.md" python3 "$CONTRACT" envelope

python3 - "$REPO/.truss-core/docs/plans/README.md" "$WORK/ng11-readme.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(text.replace("Execution plans", "Plan notes", 1))
PY
neg "a diverged plans README copy is rejected" "R12" \
  env CONTRACT_ASSETS_PLANS_README="$WORK/ng11-readme.md" python3 "$CONTRACT" envelope
```

- [ ] **Step 3: Run the checker and confirm red**

Run: `bash tests/delivery-role-contract.sh`
Expected: FAIL carrying `R11` for `WORKFLOW.md` and the plans README; the new
fixtures `ok`. Capture verbatim.

- [ ] **Step 4: Update `WORKFLOW.md`**

In the Delivered Change section, directly after the paragraph that ends
`` `.truss-core/docs/plans/active/`. ``, append:

```text
For an approved consumer-local run, the delivery control artifacts are private
and are never committed: the approved envelope and the transient plan live under
`.truss/delivery-runs/<run-key>/`, and the approval receipt lives at
`.truss/authority/approvals/<run-key>.md`. Nothing durable may be left only
there; it moves into this repository's decision record or an execution plan
before the run closes, exactly as for a repository-hosted run.
```

- [ ] **Step 5: Update both plans README copies identically**

In `.truss-core/docs/plans/README.md`, after the sentence
`Use `.truss-core/docs/templates/exec-plan.md` and place the file under `active/`.`
insert:

```text
A delivery run's transient control artifact is not a durable plan and is never
committed. For an approved consumer-local run it lives at
`.truss/delivery-runs/<run-key>/plan.md`, with the approved envelope beside it
as `approved-envelope.md`, and the approval receipt at
`.truss/authority/approvals/<run-key>.md`. Nothing that must outlive the run
stays there.
```

Then copy the result over the asset copy:

```bash
cp .truss-core/docs/plans/README.md crates/truss/assets/.truss-core/docs/plans/README.md
```

- [ ] **Step 6: Run the checker and the full repository gate**

```bash
bash tests/delivery-role-contract.sh
bash scripts/validate-premerge.sh
```

Expected: `0 failed` from the checker, then
`pre-merge validation passed` from the gate. The gate runs `cargo fmt`,
`cargo test --workspace --locked`, `cargo clippy -D warnings`, the manifest
existence loop, and `tests/s5-rehearse.sh`.

Record for the acceptance table: whether `cli_lifecycle` still expects 18
delivery manifest paths, and the verbatim summary line of each gate.

- [ ] **Step 7: Commit**

```bash
git add tests/delivery-role-contract.sh .truss-core/docs/WORKFLOW.md \
  .truss-core/docs/plans/README.md \
  crates/truss/assets/.truss-core/docs/plans/README.md
git commit -m "docs(workflow): record the private delivery artifact boundary"
```

---

## Acceptance

| Requirement | Instrument | Counterexample | Observed red |
| --- | --- | --- | --- |
| The delivery skill names the private run path, snapshot, receipt, exclude prerequisite, and fail-closed rule | `check_envelope` R8 inside `bash tests/delivery-role-contract.sh` | A copy of `SKILL.md` with the receipt path removed (`ng6`) | Task 1: `12 ok, 1 failed`, the failure listing every missing R8 token, while `ng6` reported `ok` |
| The delivery skill no longer commits the transient plan unconditionally | `check_envelope` R9 | A copy of `SKILL.md` with the old sentence re-appended (`ng7`) | Task 1: `ng7` reported `ok` in the same RED run |
| The three templates carry the private path and the never-committed rule | `check_envelope` R10 | `plan.md` with the path removed (`ng8`); `decision-record.md` with the rule reworded (`ng9`) | Task 2: `14 ok, 1 failed`, R10 firing on plan, BA, and decision, with `ng8` and `ng9` reporting `ok` |
| `WORKFLOW.md` and the plans README state the boundary | `check_envelope` R11 | `WORKFLOW.md` with the rule reworded (`ng10`) | Task 3: `16 ok, 1 failed`, R11 firing on `WORKFLOW.md` and the plans README, with `ng10` reporting `ok` |
| The two plans README copies cannot drift, measured on bytes | `check_envelope` R12 | A diverged asset copy (`ng11`), and a newline-only divergence (`ng12`) | Task 3: `ng11` reported `ok`. Byte measurement was added in the final fix wave (`18 ok, 0 failed`), because the first R12 compared decoded text and could not see a newline-only divergence |
| The payload is unchanged in shape | `cargo test --workspace --locked` (`cli_lifecycle` manifest count, `embedded_distribution`, `addon_payload_descriptor`) | A manifest line added or dropped | Final fix wave: `pre-merge validation passed`, rehearsal `56 ok, 0 failed`, manifest still 18 non-comment paths |

**Cannot be observed:** whether an agent actually refuses to stage a private
artifact, and whether an accepting session really obtains the receipt through a
private handoff. The instruments above read text. That gap is why the contract
also states the fail-closed rule in prose, and why integration acceptance must
read the diff for it.

---

## Stop conditions

- `cargo test` shows the delivery manifest count moving off 18: a file was added
  or removed despite the constraint. Stop and return `NEEDS_REPLAN`.
- The two plans README copies cannot be kept identical without a change outside
  this plan's file list.
- The checker's negative fixtures pass while the rule is present — the rule does
  not discriminate. Fix the rule; do not accept the row.
- PowerShell parity is not claimed anywhere by this plan; no installer or script
  behavior changes.

## Closure gates

```bash
# from the repository root
bash tests/delivery-role-contract.sh
bash scripts/validate-premerge.sh
git diff --check
```

## Self-review

- Spec coverage: 0006 items 1-6 map to Task 1 (skill) and Task 2 (templates);
  item 7 (stop until the guidance carries the contract) is discharged by the
  landing of these three tasks; the 0006 follow-up listing `WORKFLOW.md` and both
  plans README copies maps to Task 3. 0007 §9 supplies the paths. 0007's layout,
  resolver, provenance, and installer work is deliberately **not** here; it is a
  separate plan that depends on this one.
- Type consistency: rule ids R8-R12, subcommand `envelope`, and environment
  variables `CONTRACT_WORKFLOW`, `CONTRACT_PLANS_README`,
  `CONTRACT_ASSETS_PLANS_README` are used with the same names in every task.
- Placeholders: none. Every step carries the text or code it needs.

## Result

Completed 2026-09-25 on branch `refactor/private-envelope-contract`, commits
`702538f`, `9051d68`, `117ff14`, and the final-review fix wave `a738106`.

The delivered delivery guidance now carries the private accepted-envelope
contract of decision 0006: the skill, its three templates, `WORKFLOW.md`, and
both copies of the plans README. Rules R8-R12 in
`tests/delivery-role-contract.sh` guard the wording, including a byte-level
identity check on the two plans README copies.

Validation: `bash tests/delivery-role-contract.sh` reports `18 ok, 0 failed`;
`bash scripts/validate-premerge.sh` reports the same contract line, rehearsal
`56 ok, 0 failed`, and `pre-merge validation passed`; `git diff --check` and
`cmp` on the two README copies both exit 0.

Limitations: the instruments read text. They prove the guidance says the right
thing, not that an agent obeys it, and not that an accepting session obtains the
approval receipt through a private handoff. R9 matches the literal superseded
sentence only. One installer note surfaced during the fix wave: the payload must
be committed before `validate-premerge.sh` runs, because the installer refuses an
uncommitted add-on payload by design.

Follow-up: plans 2 and 3 of decision 0007 — distribution source separation, then
the repository migration (self-install in local-only mode, installer local-only
exclude mode, `.gitignore` cleanup, authority untracked).

## Next plan

`0007` implementation splits into two further plans, each with its own gate:

1. **Distribution source separation** — `distribution/` tree and layout marker,
   manifest-to-source resolver, embedding reads only canonical sources,
   provenance check against the recorded ref, gate updates.
2. **Repository migration** — self-install in local-only mode, installer
   local-only exclude mode with both installers reconciled, `.gitignore`
   cleanup, authority moved to `.truss/authority/` and untracked.
