# 0006 Private Accepted-Envelope Binding

Date: 2026-09-25

## Status

Accepted

## Context

0005 accepts a consumer-local installation in which delivery plans and delivery
process artifacts are never committed. That collides with the current delivery
contract, which requires committing `templates/business-analysis.md`, the
approved decision record, and the transient plan
(`.agents/skills/delivery/SKILL.md:122-126`) and places the Architectural
execution envelope inside that committed plan (`:223-225`).

A digest alone does not establish that an envelope was approved. It proves only
that two byte sequences match. Without an independent anchor, an artifact author
can change the envelope and the digest together, and the change looks
consistently approved.

Two candidate anchors were rejected as insufficient on their own:

- An Orca run record is not proven to provide durable retention or tamper
  protection, so it cannot be the only anchor.
- A committed digest-only receipt would publish the existence and count of
  delivery runs to the remote, which 0005 exists to avoid.

## Decision

1. For an explicitly approved consumer-local run, the **code baseline** is the
   exact pre-implementation commit. Business analysis, the approved decision
   record, the approved execution envelope, and the transient plan are private
   artifacts and must not be staged or committed.
2. Gate 1 approval binds, at the moment of approval: the candidate Git root,
   the baseline commit, the approved-envelope path, and the SHA-256 of an
   immutable approved-envelope snapshot.
3. The **approved envelope is an immutable snapshot**, stored separately from
   mutable per-run progress:

   ```text
   .truss/delivery-runs/<run-key>/
   ├── approved-envelope.md   # immutable; digest recorded in the receipt
   ├── plan.md                # mutable per-run progress
   ├── prompt-*.md            # dispatch prompts
   └── report-*.md            # handoff and reports
   ```

   Mutable progress never changes the approved digest. A change to scope or
   envelope produces a new snapshot, a new receipt, and a new approval.
4. The **approval receipt** is the primary anchor and is local-only:

   ```text
   .truss/authority/approvals/<run-key>.md
   ```

   It records the run-key, the candidate root, the baseline commit, the
   approved-envelope path and its SHA-256, the owner's approval note, and the
   approval date. It is never committed. A one-line echo of the digest is
   written to the Orca run record as a secondary copy only; the local receipt
   remains authoritative.
5. Independent acceptance is **fail-closed**. The accepting session obtains the
   approved-envelope snapshot and the receipt through an authorized private
   handoff, recomputes the digest, and binds its verdict to both the final
   candidate `HEAD` and the approved envelope identity. A missing, unreadable,
   mismatched, or unapproved snapshot or receipt blocks acceptance. The session
   must not accept on the author's assurance alone.
6. Retention: receipts and run artifacts are not auto-deleted and are not
   expired by any process. The owner deletes them explicitly. Delivery must not
   delete run artifacts on its own, because ignored artifacts are not protected
   from destruction and the previous instruction to delete the plan in the
   release commit no longer applies.
7. Until `.agents/skills/delivery/SKILL.md` and its templates carry this
   contract, Architectural delivery on a local-only candidate must **stop and
   report**, rather than commit private artifacts or silently drop the envelope
   from the baseline.

## Alternatives Considered

1. Orca run record as the only anchor. Rejected: retention and tamper
   protection are unproven, and Orca state can be lost or rebuilt.
2. A committed digest-only receipt. Rejected: it publishes the existence,
   timing, and count of delivery runs to the remote.
3. Hashing `plan.md` itself as the approval anchor. Rejected: the plan is
   mutable progress, so its digest changes legitimately on every update and the
   approval binding becomes meaningless.
4. A separate private Git repository for run artifacts. Rejected for now: it
   adds a second history and remote to maintain, and the owner's stated goal is
   local-only working memory.

## Consequences

Positive:

- A local-only candidate keeps its entire envelope and progress off the remote
  while approval remains independently checkable.
- The anchor survives Orca state loss and does not depend on unproven retention.
- Fail-closed acceptance makes a missing anchor a stop condition instead of a
  silent weakening.

Tradeoffs:

- The anchor does not survive loss of the machine holding it. Backup is the
  owner's responsibility; nothing verifies it.
- The accepting session needs an authorized private handoff channel. A clone or
  a fresh worktree does not carry ignored artifacts.
- `git clean -fdx` can delete both the run artifacts and the receipt.
- Retention is manual and indefinite, so private artifacts accumulate.

## Follow-Up

- Update `.agents/skills/delivery/SKILL.md` (Architectural artifacts, the
  acceptance baseline, the release step that deletes the plan) and
  `.agents/skills/delivery/templates/{plan,business-analysis,decision-record}.md`.
- Update `.truss-core/docs/WORKFLOW.md` and `.truss-core/docs/plans/README.md`,
  plus the distributed copy at
  `crates/truss/assets/.truss-core/docs/plans/README.md`.
- The artifact paths above move to `.truss/delivery-runs/` and
  `.truss/authority/approvals/`; until 0007 lands, they apply at their declared
  paths and the move is part of the source/installed separation.
- Add a contract check that rejects acceptance when the snapshot digest, the
  receipt, or the handoff is missing or mismatched.
