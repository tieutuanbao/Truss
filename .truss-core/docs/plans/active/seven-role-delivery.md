# Seven-role delivery migration

Authority: ../decisions/0004-seven-role-delivery.md (under docs/decisions).
Run: run_eeaecf6b6bfa. Candidate is original consumer checkout.
Baseline: 68a4b34723d47daf7a2d7d4c442297297d5ef120.

## Outcome and approach

Replace legacy role routing with seven approved roles, substantive BA handoff,
detailed parallel-capable planner contract and independent tester sessions.
One coherent implementation task; avoid concurrent edits to coupled protocol.
Fresh tester-debugger performs acceptance after implementation. Local only.

## Recovery checkpoint

Planning completed: task_e405d5e7ea6d / ctx_5b8101b4077b.
Planning report: .delivery-role-plan-report.md (temporary dispatch evidence).
Planner did not change candidate. Implementer and acceptance still pending.
Use Orca dispatch state before resuming; do not start duplicate live writers.

## Proof and risk

Run focused role-contract positive/negative fixtures, shell syntax, diff check,
and scripts/validate-premerge.sh on committed payload. Independently inspect
session separation, parallel isolation, custom-pin migration and preserved
approval/recovery boundaries. Text checks cannot prove future agent compliance.
No hooks, CI settings, global configuration or publishing changes authorized.

## Completion

Pending implementation, executable validation, independent exact-HEAD acceptance.
Move to completed only after validation. Keep durable results here; transient
per-dispatch task state belongs to Orca, not duplicate progress tables.
