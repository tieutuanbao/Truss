<!-- TRUSS:BEGIN -->
## Truss

Start with the requested outcome and use the repository as the system of record.
Read `.truss/core/docs/WORKFLOW.md` and only relevant product, design, plan, code, and
validation material.

- Answers, explanations, reviews, diagnoses, plans, and status reports are
  read-only. Inspect only what is needed; change nothing.
- Apply the four Karpathy behavioral defaults in `.truss/core/docs/WORKFLOW.md`
  when writing, reviewing, or refactoring code; their scope and authority
  qualifications apply, and no separate invocation is required.
- For a bounded change, inspect affected behavior and proof, implement, and
  validate. No control-plane operation is required.
- Use one `.truss/authority/plans/active/` file when work spans sessions, coordinates
  contributors, has dependencies, or needs recovery. Move it to
  `.truss/authority/plans/completed/` only after validation.
- Follow the single communication standard in `.truss/core/docs/WORKFLOW.md`
  for every user-facing reply.
- Before editing, identify repository authority for each new externally
  observable policy. If materially different choices remain open, stop before
  edits; configurable defaults are not authority.
- For architecture, reliability, security, or quality invariant work, read
  `.truss/core/docs/patterns/encoding-invariants.md` and enforce only accepted rules.
- Report reusable agent friction. Change guidance, tools, runbooks, or validation
  for that purpose only when explicitly asked to use `$improve-truss`.
- Also pause when product intent remains ambiguous, recovery is difficult,
  validation is weakened, or authority is insufficient.
- Claim completion only with executable or observable evidence. Report outcome,
  changes, validation, and unresolved risks.

Truss owns no durable task database or orchestration lifecycle of its own.
Repository plans remain durable memory and authority. When an explicitly
installed workflow uses an execution plane, its Runs, Tasks, and Dispatches are
permitted transient execution state; they do not replace repository authority or
create a second durable source of truth.

That boundary records where durable authority is kept; it is not a finding that
a dispatched worker lacks authority. A task dispatched under an approved
contract is authorized, and its dispatch is the record of that authorization.
<!-- TRUSS:END -->
