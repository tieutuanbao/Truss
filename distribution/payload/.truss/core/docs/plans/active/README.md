# Active Execution Plans

Place one evolving plan in `.truss/authority/plans/active/` when work needs
durable memory. Use `.truss/core/docs/templates/exec-plan.md`, keep progress and
validation current, and move the plan to `.truss/authority/plans/completed/`
only after the result is verified. The installed payload is never a write
target.
