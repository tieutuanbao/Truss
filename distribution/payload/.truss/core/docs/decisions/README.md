# Decisions

Decision records preserve lasting product, architecture, data ownership,
security, compatibility, and validation choices that future work must inherit.

Use `.truss/core/docs/templates/decision.md`. Task-local implementation choices remain in
the active execution plan and do not require a separate decision.

An installed consumer begins with no fabricated decisions. Add local decision
documents under `.truss/authority/decisions/` as real choices are accepted, and
index them in `.truss/authority/decisions/README.md`; the installed payload is
never a write target.
