# Documentation Map

Start with the smallest authoritative surface.

## Current Product

- `WORKFLOW.md`: request shape, planning, judgment, operation, validation, and
  completion.
- `product/`: current product behavior and installation contract.
- `decisions/`: lasting choices future work must inherit.
- `plans/`: one durable working-memory document for work that needs it.
- [`patterns/encoding-invariants.md`](patterns/encoding-invariants.md): turn
  accepted architecture, reliability, security, and quality rules into native
  mechanical validation.
- `templates/`: optional decision, plan, runbook, communication, and
  Truss-improvement structures.

## Consumer-Owned Truth

The consumer's README, product documents, architecture, code, tests, CI,
runtime signals, and application behavior remain authoritative. Truss does
not overwrite those with upstream product assumptions.

A project's own authority — its decisions, execution plans, product documents,
and its recorded communication selection — lives under `.truss/authority/`.
`.truss/core/**` is the installed Truss payload: read it, never write project
content into it.

## Source Repository

An installed consumer holds none of this section. These paths exist only in a
Truss source checkout, and reading them is a source-repository activity, not an
installed one.

- `ARCHITECTURE.md` and `TRUSS.md`: Truss's own architecture and product
  authority, describing the tool rather than the consumer's product. They are
  never installed into `.truss/core/**`.
- Root `README.md`: product overview, installation, maintenance, and
  development.
- `crates/truss/`: safe core installer/updater.
- `scripts/`: platform bootstrap, release, and validation entrypoints.
- `tests/`: behavior ownership and repository contract.

Completed plans may be removed from the current tree when decisions, code,
tests, and Git history preserve their lasting result.
