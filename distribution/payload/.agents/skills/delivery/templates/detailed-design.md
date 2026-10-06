# Detailed design and audit template

One combined authoring template. The `detailed-designer` role produces three
durable outputs from it, and the role owns all three: `design.md`, a separate
durable `audit.md`, and `skeleton/` holding the placeholder-body skeleton.
`audit.md` is a distinct artifact, not a section inside `design.md`, because a
readiness claim must remain reviewable independently of the design it judges.

Write them to the repository-authority path named by the approved envelope,
normally `.truss/authority/design/<design-key>/`, so structural authority
survives the run without competing canonical copies. For a repository-hosted run,
the approved baseline commit carries the exact authority path and SHA-256 of
each output. For an approved consumer-local run, the package stays
repository-local at that authority path: it is never staged, committed, or
force-added, and the accepting dispatch carries each absolute candidate-local
path and SHA-256. If placement, persistence, or retrieval authority is missing
or ambiguous, return `NEEDS_INPUT` before writing.

This template records design and proof, not business analysis and not task
decomposition. `templates/business-analysis.md` owns product authority,
`templates/decision-record.md` owns high-level technical authority consumed by
this package but not executed from, and `templates/plan.md` owns task
decomposition and mapped proof. The `architect` decides high-level contracts;
this role decides structure; `planner` decomposes work; `implement` codes
behavior and returns structural ambiguity here.

A copy of this template with its placeholders intact is not a completed design
package and must not be handed to `planner`. The readiness value is exactly
`NOT_READY` or `READY_FOR_PLANNING`. An absent audit, absent dispatch, partial
output, unknown state, hash mismatch, or incomplete proof is `NOT_READY`.

---

# Detailed design — <change>

## Status and authority

Name the consumed decision record and its identity, the architect output this
design realizes, the baseline it was produced against, and the role that
authored it. Record material assumptions as assumptions, not as decisions.
Nothing here may restate or reopen the decision record's high-level contracts.

## Design categories

Every category below must carry either a concrete decision referencing the
`design.md` content that satisfies it, or an evidence-backed non-applicability
rationale. A bare `N/A` cannot conceal an applicable category. Module and file
structure, interfaces, inputs, outputs, and dependency order are resolved here,
not deferred to implementation.

| Category | Decision | `design.md` reference | Applicability evidence |
| --- | --- | --- | --- |
| Modules and file structure | | | |
| API signatures | | | |
| Inputs and outputs | | | |
| Interfaces | | | |
| Dependency order and sequence | | | |
| Data structures | | | |
| Enums and discriminants | | | |
| State machines and state behavior | | | |
| Error handling and error codes | | | |

## Non-applicability record

For each category marked non-applicable, state why no decision is required and
cite the evidence that supports it. A category with no entry here is not
resolved.

| Category | Rationale | Supporting evidence |
| --- | --- | --- |

## Structural ambiguity register

Every material structural ambiguity, its competing interpretations, the
conflicting evidence, and its resolution. An unresolved material ambiguity
blocks `READY_FOR_PLANNING`; there is no "ready with material TODOs". An
implementer that finds a structural ambiguity pauses and returns it through
Control to this role — it is not resolved in implementation.

| Ambiguity | Competing interpretations | Evidence | Resolution |
| --- | --- | --- | --- |

---

# Skeleton inventory — `skeleton/`

The skeleton embodies the actual intended file and module structure and
signatures. A prose list or an unrelated compiling example is not sufficient.
Each indexed item states its target path, source authority, implemented
structure, intentionally unimplemented behavior, validation command, expected
outcome, and placeholder-body inspection method.

| Target path | Source authority | Implemented structure | Intentionally unimplemented | Validation command | Expected outcome | Placeholder inspection |
| --- | --- | --- | --- | --- | --- | --- |
| | | | | | | |

## Structural proof

Materialize the indexed skeleton bytes with only explicitly referenced baseline
dependencies in a disposable directory, then run the repository's applicable
compile, type, or syntax instrument. Record the tool versions, construction
provenance, every command, exit status, and the exact tested digests. Missing
tooling, undeclared dependencies, broken imports, or broken signatures leave
the package not ready.

| Field | Value |
| --- | --- |
| Tool and version | |
| Disposable directory and construction provenance | |
| Commands and exit status | |
| Tested digests | |

A standalone self-contained fixture can be checked with
`rustc --edition=2021 --crate-type=lib --emit=metadata`. A skeleton dependent on
crate context uses a disposable baseline overlay and the appropriate locked
Cargo check instead. Do not claim a standalone fixture proves the real crate's
dependency wiring.

## Placeholder-boundary proof

Inspect all new function and method bodies, initializers, macros, callbacks,
and embedded executable fragments. They may declare shape and explicit
placeholders, but must not implement business behavior. Compilation alone,
keyword presence, and unverified worker assertions cannot satisfy this result.
Record the inspection method and its result per indexed item.

| Target path | Inspection method | Result |
| --- | --- | --- |

Canonical new Rust callable bodies are exactly
`todo!("detailed-design placeholder")`; declarations without bodies are allowed
where the language requires them. Shape-defining fields, variants, and type
relationships are real. Domain algorithms, default-valued returns pretending to
be placeholders, I/O, state changes, and domain-computing initializers are not.
A `todo!` somewhere in a behavior-filled function is insufficient, and a body
containing a type-correct real implementation passes structural validation while
failing placeholder inspection — record both outcomes.

For this repository's Markdown guidance product the document skeleton is real
Markdown with the intended sections, tables, field contracts, and explicit
unfilled narrative slots, plus well-formed YAML metadata where applicable.
Repository-native contract checks validate that structure; a successful text
read or `git diff --check` is not syntax or structure proof. Applicable Rust
test or helper changes still require a real Rust skeleton and structural proof.
Where no repository-native instrument exists for a consumer technology, bind
that repository's existing compiler, type, or syntax command and its
language-aware check, or record a named expert inspection. There is no universal
"all languages compile" claim and no automatic pass for an unsupported language.

---

# Design audit — `audit.md`

Role-owned and self-audited. Independent review and acceptance remain separate
`tester` sessions when risk routing or release requires them, and are not replaced by this audit. Every entry
references the exact audited bytes by SHA-256, records the command or method
used, and reports its result. An audit that is not bound to the current package
identity is `NOT_READY`.

## Readiness

`Readiness: READY_FOR_PLANNING | NOT_READY`

The value is exactly one of those two. Record the conjunction that produced it.

## Package identity

| Output | Path | SHA-256 |
| --- | --- | --- |
| `design.md` | | |
| `skeleton/` per-item | | |

After finalizing `audit.md`, compute its full-file SHA-256 and record its path
and digest in the existing approval envelope or authorized consuming handoff,
outside `audit.md`. Do not embed the audit's own digest in its bytes or invent
an excluded-field hash. Changing the audit invalidates that external identity;
recompute it and follow the existing approval boundary for material changes.

## Category coverage

Substantive coverage is checked, not headings alone. Every category in the
design table is referenced here with its audit entry or its evidence-backed
non-applicability rationale.

| Category | Audit entry | Complete |
| --- | --- | --- |
| Modules and file structure | | |
| API signatures | | |
| Inputs and outputs | | |
| Interfaces | | |
| Dependency order and sequence | | |
| Data structures | | |
| Enums and discriminants | | |
| State machines and state behavior | | |
| Error handling and error codes | | |

## Ambiguity review

Every material structural ambiguity is resolved, with no contradictory
contracts and no unresolved alternatives affecting implementation.

| Ambiguity | Resolution | Contradictory contract remaining |
| --- | --- | --- |

## Proof results

| Proof part | Commands and methods | Exit status / result | Digests tested |
| --- | --- | --- | --- |
| Structural proof | | | |
| Placeholder-boundary proof | | | |

## Sequencing evidence

The completed architect output precedes this dispatch, and this dispatch
precedes planner decomposition.

| Field | Value |
| --- | --- |
| Architect output identity | |
| Detailed-designer dispatch identity | |
| Planner dispatch (not yet run) | |

## Limitations

State what this audit did not observe: what was not reproduced, not read, or
not decidable mechanically. Named expert inspection is recorded here with its
reader and its limit.
