# Truss Improvement: Bind acceptance proof to the approved instrument

Date: 2026-09-26

## Status

Awaiting fresh rerun

## Representative Job

Architectural repository-hosted delivery of `truss migrate` in this Truss source
checkout (`run_7f2427a4e9ea`, branch `delivery/truss-migrate`, baseline
`f4d504b`, 13 commits, released as `truss-v0.2.1`). Transport: Orca supervised
worker terminals; roles `ba` (Pi `tao-router/explore`, medium), `architect` (Pi
`tao-router/thinking`, high), `planner` (Pi `tao-router/thinking-high`, high),
`implement` (Pi `tao-router/code-writer`, medium), `tester-debugger` (Pi
`tao-router/code-reviewer`, high). Authority: local only, no push until the owner
authorized release; owner approved the design contract and three later decisions.
Wall clock 14:55 → 21:23 (388 min).

## Baseline

Three defects were found only after an integration acceptance had already
returned `ACCEPT`:

1. **Permission bits lost** (`40e76b5`). `migrate --apply` published files through
   a byte copy that dropped Unix modes, so `.truss/core/bin/truss` became `664`
   and the retained backup lost the bit too, meaning a rollback would have
   restored bytes without the mode.
2. **Flaky gate** (`571bf82`).
   `crates/truss/tests/migration_lifecycle.rs::a_copied_running_binary_applies_and_retires_its_own_tree`
   failed about one run in six at default parallel threads with
   `ExecutableFileBusy`, so `scripts/validate-premerge.sh` was intermittently red.
3. **Stale bundled executable** (`c8712a4`, owner decision A). After migrating a
   real `truss-v0.1.16` installation with the 0.2.0 binary,
   `.truss/core/bin/truss` stayed 0.1.16 and reported `not_installed` on a
   layout-3 root; `truss update` cannot repair it because
   `crates/truss/src/application/self_update.rs` compares
   `env!("CARGO_PKG_VERSION")` of the running binary. This defect was found only
   because the owner asked whether a real old installation had been rehearsed.

Evidence for the escapes:

- The contract already named the instrument correctly:
  `.truss/delivery/runs/truss-migrate/business-analysis.md` carries `ASM-09`
  ("a complete legacy installation built from the `truss-v0.1.16` ref") and
  `REQ-037` ("...the 0.1.16 migration fixture...").
- The implementation delivered a substitute and labelled it as satisfying
  `ASM-09`: `legacy_repository()` installs with the current binary and
  `legacy_tree(state, Some("0.1.16"))` rewrites paths and stamps
  `core_version`, while the doc comment claims a genuine legacy consumer.
- An acceptance session certified the substitution in writing and cited the
  requirement: `report-acceptance-task2.md:51` — "Fixtures built by
  `install --json`, moving `.truss/core` → `.truss-core`, rewriting manifest
  paths ... and `core_version` to `0.1.16` (the lifecycle suite's own fixture
  recipe, ASM-09)." Integration acceptance #1 did the same by hand.
- Four independent acceptance sessions returned `ACCEPT` before defect 3 was
  found, all inheriting the same substituted fixture.
- Mode blindness is visible in the criteria themselves: `REQ-011`'s instrument is
  a per-file hash comparison and `REQ-012` demands `modified: 0, missing: 0`;
  neither observes permission bits, and the post-apply reads were run with the
  external `target/debug/truss`, so the dead installed file was never executed.
- Flake blindness: `report-acceptance-task2.md:32` records a single invocation of
  the lifecycle suite.

Human intervention required: the owner asked whether the real old-install
rehearsal had been done; Control then built it and discovered defect 3. Total
post-acceptance rework was 177 min of 388 min (46%): 30 + 90 + 57 min.

Known limitations: one observed trajectory, not a recurring pattern; the harness
cannot generally decide semantic equivalence of two fixtures; Windows apply and
several other boundaries remain unverified (see the completed migration plan).

## Earliest Gap

**Proof.** The requirement was strong enough; the executed proof object was not
bound to it. Implementation and acceptance silently substituted a materially
different fixture (provenance, payload version, bundled executable) and then
satisfied the row by citing the requirement name.

Escalated to Oracle (`advisor`, `tao-router/thinking-high`, read-only, fork
context). Oracle's finding: the deepest category is **proof substitution was not
bound to the approved instrument**, not candidate independence and not worker
capability. Candidate independence remains correct and necessary; broadening it
would be an overcorrection, because it was the *requirement* that was already
explicit while the *executed instrument* changed.

Oracle assigned three distinct earliest owners rather than one:

| Defect | Earliest gap | Earliest owner |
| --- | --- | --- |
| Stale bundled executable | Proof substitution: a synthetic current-version fixture replaced the real released artifact | Delivery acceptance protocol |
| Lost permission bits | Proof specification: byte hashes and status omitted filesystem metadata and execution of the installed entrypoint | BA/architect acceptance-table author |
| ETXTBSY flake | Proof sampling: one passing concurrent-suite invocation treated as stability evidence | Planner/acceptance instrument design |

Common ancestor: a proof row could be declared satisfied without showing that the
executed instrument matched the approved outcome and observability boundaries.
The existing skill already forbids narrowing an approved requirement, so this is
partly non-compliance with existing intent plus one missing narrow rule —
instrument substitution is itself a mismatch.

## Correct Owner

The Truss source, specifically the existing Delivery acceptance owner:
`.agents/skills/delivery/SKILL.md` §Acceptance, its handoff contract, the
canonical payload copy under `distribution/payload/.agents/skills/delivery/`,
and the existing contract test `tests/delivery-role-contract.sh`. No new role and
no parallel framework. Consumer repositories and the external environment are not
the owners; the owner's question that exposed defect 3 was a human recovery, not
a fix.

## Intervention

Stated as required:

```text
If the Delivery acceptance contract requires an explicit proof-fidelity
statement for every row (approved instrument, executed instrument, provenance or
starting state, substitutions, observability limits) and states that citing a
requirement id or instrument name never satisfies a row, then a fresh accepter
will report the substituted fixture instead of endorsing it on the
representative job, because the mismatch becomes a required field of the verdict
rather than an inference the accepter must make unprompted.
Evidence that would weaken this: a fresh accepter with the new fields present
still returns ACCEPT while executing a materially different instrument;
or accepters write `Substitutions: none` mechanically without inspecting
provenance.
Maintenance owner: Truss source delivery skill owner.
Removal condition: remove if a materially equivalent rerun shows the fields are
written mechanically without changing detection, or if the cost of the fields
exceeds the detection they buy.
```

Proposed change, one intervention only:

1. `.agents/skills/delivery/SKILL.md` §Acceptance — add the five fidelity fields
   and the rule: an accepter may not satisfy a requirement by citing its
   identifier or instrument name; it must compare the executed instrument with
   the approved one; any material substitution in fixture provenance, artifact
   version, environment, cadence, comparator, or observation interface is a
   contract mismatch and returns `CHANGES_REQUESTED` for Control reconciliation,
   with "equivalent" demonstrated rather than asserted.
2. The same text in the canonical payload copy
   `distribution/payload/.agents/skills/delivery/SKILL.md`, plus regeneration of
   `tests/payload-layout-digests.txt` because the payload bytes change.
3. `tests/delivery-role-contract.sh` — one new check that the skill text requires
   all five fields and the no-citation rule, with an observed-red negative probe
   that removes a field or the rule.

Honest ceiling: a repository check can only prove the contract *requires* the
fields. Acceptance reports are private run artifacts under `.truss/delivery/`
and are never committed, so no repository validator can inspect a real report
here. Deciding whether a disclosed substitution is material remains human or
agent judgement.

## Native Validation

Pending. To run after the intervention is authorized: `bash
tests/delivery-role-contract.sh` with an observed-red negative probe (removing a
field or the no-citation rule fails the new check), then the full
`bash scripts/validate-premerge.sh` at the changed HEAD, plus digest
regeneration verified by the payload-layout contract.

## Fresh Rerun

Pending and not yet authorized. Required design: a fresh accepter session,
equivalent task class, given an approved instrument that names an immutable old
release and a candidate suite that uses a relabelled current fixture while all
tests and gates pass. Success means the accepter retrieves the new rule,
identifies the provenance substitution, and returns `CHANGES_REQUESTED` without
human prompting. Record whether the intervention was available, retrieved, and
relevant, and whether human intervention dropped.


## Experiment Results (A/B/C, same question, same transport)

One neutral acceptance question, one fresh `tester-debugger` terminal each
(`tao-router/code-reviewer`, high), materially identical prompt; only the
contract text and Control's prompt conformance differ.

| Run | Contract | Control prompt | Wall clock | Verdict | Provenance substitution |
| --- | --- | --- | --- | --- | --- |
| A | before the intervention | enumerated handoff fields | 3 min 54 s | `ACCEPT` | disclosed only as "did not reinstall the truss-v0.1.16 tag"; REQ-037 satisfied by test names |
| B | after the intervention | enumerated handoff fields (stale) | 2 min 45 s | `CHANGES_REQUESTED` | not compared; rejected for missing per-row observed-red proof and a branch mismatch |
| C | after the intervention and its revision | names the format, never restates it | 3 min 14 s | `ACCEPT` | disclosed and dismissed as non-material |

Run A reproduces the escape on the released artifact: the accepter accepted
`REQ-001..REQ-038` at `caff853` while the committed fixture is a relabelled
current install, and it wrote "I did not independently reinstall the remote
`truss-v0.1.16` tag in this session" in the same report.

Run C wrote the new fields:

> `Proof fidelity:` ... `Substitutions: no material command substitution for the
> required suites or full gate; individual process fixtures are exercised inside
> the approved Rust suites rather than reimplemented ad hoc.` `Observability
> limits: synthetic fixtures do not prove every real legacy consumer shape.`

So the intervention converts a silent substitution into a disclosed one and
still returns `ACCEPT`. The recorded falsifier materialized: the accepter
inspected provenance, disclosed the substitution, and ruled it non-material. The
mechanism is that "the executed instrument" was read as the *command*
(`cargo test`), with the fixture treated as an internal detail of that command,
even though the approved row named the artifact and its version boundary.

Cost asymmetry that the numbers establish: an acceptance run costs about 3
minutes, while the original escape cost 57 minutes of rework plus an owner round
trip and came within one question of shipping a broken release. A real-instrument
rehearsal (install from the released tag, migrate, verify the bundled executable)
took about 4 minutes and detected the defect immediately.

### Consequence for the decision

The prose route has a measured ceiling: it buys disclosure, not detection,
because materiality remains a judgement made by the same accepter. Adding more
prose risks noise without changing verdicts. The competing, mechanical route is
to make the mandated artifact executable rather than merely named: require, for
every row that names artifact provenance or a version boundary, an instrument
that constructs that artifact, so there is nothing to substitute. That is a
different owner (the acceptance-table template and the migration suite's
fixture) and is a change to released verification, so it needs an owner decision
before it is attempted.


## Mechanical Intervention (second experiment)

Owner authorized the mechanical route. Implemented in commit `fc96895`:

1. `distribution/payload/.agents/skills/delivery/templates/business-analysis.md`
   §Acceptance now requires that a row naming artifact provenance name **a
   command that constructs that artifact**, "not the artifact's name", and states
   why: "bytes cannot show that the executed fixture is the approved one".
2. `tests/legacy-migration-rehearsal.sh` is that command for a released pre-0008
   installation. It installs the exact tag through that tag's own installer,
   refuses any ref that is not an immutable release tag, and **fails when the
   legacy artifact reports the current version**, so a relabelled current install
   can never satisfy it. It is deliberately outside `scripts/validate-premerge.sh`,
   which stays offline.
3. `R16` in `tests/delivery-role-contract.sh` binds the template rule to that
   script (existence, executability, the release-tag refusal, the version-skew
   guard) with two observed-red probes.

Measured:

| Instrument | Result | Wall clock |
| --- | --- | --- |
| `tests/legacy-migration-rehearsal.sh --tag truss-v0.1.16` | 24 ok, 0 failed; legacy 0.1.16 -> current 0.2.1 | **17 s** |
| same script with `--tag truss-v0.2.1` (anti-relabel probe) | 15 ok, 9 failed, exit 1, including "the legacy artifact reports the legacy version, not the current one" | 12 s |
| `tests/delivery-role-contract.sh` | 30 ok, 0 failed | — |
| `bash scripts/validate-premerge.sh` at `fc96895` | `pre-merge validation passed` | — |

### Run D — the agent-level measurement, and why it did not test the intervention

Fresh accepter, same neutral question, conformant prompt, contract containing the
template rule and the script.

| | |
| --- | --- |
| Wall clock | 3 min 20 s |
| Verdict | `ACCEPT` |
| `Proof fidelity` | present; `Substitutions: none for the required commands, working directory, fixture interfaces, or comparators` |
| Did it invoke the rehearsal? | **No** — `tests/legacy-migration-rehearsal.sh` appears nowhere in the report |

Per improve-thrift honesty rules this run must be scored as **not a test of the
intervention**: the intervention was *available* and partly *retrieved*, but it
was **not relevant** to the artifact under acceptance. `REQ-037` in
`.truss/delivery/runs/truss-migrate/business-analysis.md` was authored before the
template rule existed, so that row names an artifact and never names a
constructing command. The accepter took its instrument set from the commands the
plan lists, and no approved row or plan task named the script. The escape
therefore survives because the *contract* predates the rule, not because the
accepter ignored a rule that applied to it.

Rewriting the historical `REQ-037` row to name the script would be editing an
accepted record, which the harness forbids, and authoring a fresh BA solely to
make the experiment pass would be manufacturing evidence. The bounded conclusion
is therefore:

- The mechanical instrument is correct, cheap, and provably rejects the relabel
  class; it can be run in 17 seconds where the escape cost 57 minutes.
- Its effect on an acceptance verdict is **unproven**, pending a contract whose
  approved row actually names a constructing command — which the template now
  requires of every new contract.
- The next materially equivalent rerun is a future delivery whose BA is authored
  under the new rule; that run should be measured against the A/D baseline of
  `ACCEPT` with no rehearsal invoked.

## Decision

Keep the mechanical instrument, the template rule, and the prompt rule; record the agent-level effect as unproven pending a contract authored under the new rule.

The rule and `R15` are implemented, validated, and retained as a partial
improvement: every acceptance now discloses the approved and executed instrument
together, which is strictly more information than before. They are not kept as
the *fix* for the escape, because runs A and C show the same verdict class before
and after. Promote or remove the mechanical follow-up by owner decision; one
observed trajectory is still not a pattern, so the mechanical change should be
scoped as its own bounded experiment.

## Result

Diagnosis confirmed by measurement, intervention implemented and validated, and
the targeted outcome falsified.

- Files changed: `.agents/skills/delivery/SKILL.md`,
  `distribution/payload/.agents/skills/delivery/SKILL.md`,
  `.truss/core/base-addons/delivery/.agents/skills/delivery/SKILL.md`,
  `tests/delivery-role-contract.sh`, `tests/payload-layout-digests.txt`
  (commits `38e7e84` and `9282861` on branch `improve/acceptance-proof-fidelity`).
- Native validation: `tests/delivery-role-contract.sh` 27 ok / 0 failed with one
  positive and four observed-red negative probes; `tests/payload-layout-contract.sh`
  26 ok; `tests/s5-rehearse.sh` 75 ok; `bash scripts/validate-premerge.sh`
  printed `pre-merge validation passed` at `9282861`.
- What the intervention does buy: a mandatory, inspectable statement of the
  approved and executed instrument, provenance, substitutions, and observability
  limits; a ban on satisfying a row by naming a requirement or an instrument; and
  a ban on Control prompts that restate the handoff fields and silently drop new
  ones. Run B showed the second ban has real force.
- What it does not buy: detection. Run C returned the same `ACCEPT` as run A.
- Limitations: three runs of one question in one repository; the question was
  authored by the same session that proposed the intervention, so it may be
  easier than an average acceptance; `R15` proves only that the contract requires
  the fields, never that a report is truthful.
- Follow-up: the mechanical route is implemented (`fc96895`) but its
  agent-level effect is unproven, because run D accepted a contract that predates
  the template rule. Re-measure on the next delivery whose business analysis is
  authored under the new rule, naming `tests/legacy-migration-rehearsal.sh` (or
  its successor) in the task verification for any provenance row. Evidence that
  would weaken the keep: that delivery still accepts a provenance row without
  executing the named command.
