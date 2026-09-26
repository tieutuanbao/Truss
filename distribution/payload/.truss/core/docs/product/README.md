# Product Docs

This directory contains current consumer-product behavior derived from real
accepted intent. Truss deliberately ships no fake product domains.

When a user provides a product specification, derive smaller living documents
under `.truss/authority/product/` instead of keeping one growing specification
as the operating manual. Name files after actual product domains, such as
`overview.md`, `billing.md`, `permissions.md`, or `api-conventions.md`. The
installed payload is never a write target.

## Current Product Contract

No consumer-specific product contract is shipped in this generic directory.
The Truss contract lives in the root README, current workflow and
architecture documents, lasting decisions, implementation, and executable
tests.

## Update Rule

When behavior changes:

1. Update the affected product document when the expected behavior changed.
2. Update the active execution plan when complex work uses one.
3. Add a lasting decision only when future work must inherit a consequential
   product, architecture, data, security, compatibility, or validation choice.
4. Add or update executable proof that exercises the behavior.

Bounded changes do not require a parallel lifecycle record.
