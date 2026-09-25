# Truss Product Model

Truss makes repository truth easier to retrieve and maintain.

## Principles

1. **Repository truth wins.** Product documents, decisions, plans, code, tests,
   CI, runtime evidence, and Git history are authoritative.
2. **Load the smallest useful context.** `AGENTS.md` is an entrypoint, not an
   encyclopedia.
3. **Process follows work shape.** Bounded work stays bounded; coordinated or
   recoverable work gets one durable plan.
4. **Material choices stay human-owned.** Missing product policy stops mutation.
5. **Behavior proves completion.** Workflow records and self-reports do not
   replace executable or observable evidence.
6. **Consumer applications own application operation.** Generic Truss files
   cannot supply stack-specific runtimes, credentials, logs, or fixtures.
7. **Truss maintains only its core.** `truss` safely installs and updates
   managed guidance without becoming a task control plane.

## Installed Core

The core provides:

- a small agent entrypoint;
- workflow and documentation maps;
- product, decision, and execution-plan locations;
- templates for durable work and application operation;
- an invariant-encoding pattern and request-triggered skill; and
- explicit-only onboarding, proposal-audit, and improvement skills.

It provides no fabricated product domains or validation commands.

## Explicit Add-Ons

Two independent opt-in payloads extend the core; neither is installed by
default, and neither activates without an explicit request:

- **Engineering wisdom**: an advisory skill for engineering judgment.
- **Delivery**: a control protocol that takes one change through an approved
  design contract, isolated implementation tasks, independent tester-debugger
  acceptance, and release on the Orca execution plane. It is invoked per change;
  it is not a standing lifecycle, task database, or background process, and the
  repository remains the system of record for everything it produces.

## Evidence

Release claims are bounded to fresh installation, repository navigation and
authority behavior, and safe updater lifecycle. Consumer runtime experiments
may improve guidance, but they do not become universal capability claims.
