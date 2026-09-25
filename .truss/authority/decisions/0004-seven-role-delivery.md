# 0004 — Seven-role delivery

Status: Accepted by repository owner in conversation on 2026-09-24.
Source revision: 68a4b34723d47daf7a2d7d4c442297297d5ef120.

Owner requested rebuilding delivery around project-manager, ba, architect,
planner, implement, visual-engineering, and tester-debugger, then explicitly
approved separate tester sessions and removal of the review role.

## Contract

Project-manager is current interactive Control, owning coordination, approved
DevOps, integration and release. BA produces concrete goals, actors, business
flows/rules/exceptions, scope, traceable requirements and acceptance for planner.
Architect owns technical decisions and interfaces. Planner maps their outputs
into detailed independent tasks with inputs, outputs, path ownership, dependency
DAG, verification, parallel waves, integration and recovery. Implement writes
code and documentation. Visual-engineering advises and builds UI as needed.
Tester-debugger tests and fixes, or independently accepts in a separate fresh
read-only session. A candidate author, fixer, planner or advisor cannot accept
that candidate. Remove separate review, plan-review, plan and consult routing;
review remains an activity, never an eighth role.

Concurrent writers require separately approved isolated worktrees; coupled work
is serialized. PM integrates before fresh exact-HEAD integration acceptance.
Preserve human authority gates, protected dirt, prerequisites, no implicit
publishing, runtime preflight and revision-bound evidence. Existing activation
and product-authority decisions remain unchanged.

## Local execution envelope

Candidate: /media/bitscat/data/Workspace/Bitscat/AI_Workflow/source.
Baseline above; main is starting branch. Create delivery/seven-role-delivery.
Local only; no push, PR, default-branch merge, reset, stash, clean or global config.
This migration is one coherent serialized implementation task in this checkout;
parallel capability is documented, not exercised using unapproved extra checkouts.

Owned scope: delivery and delivery-setup skills, their YAML and templates,
delivery install manifest, AGENTS delivery block, README, WORKFLOW, TRUSS,
installation-profiles template inventory, focused role-contract test and native
premerge entry point, plus manifest-count expectations in
crates/truss/tests/cli_lifecycle.rs (17 to 18 for the added BA template).
This companion test update was identified by the native gate during execution;
it changes no production Rust or installer logic and preserves the approved
payload acceptance scope. Control owns this
record and durable execution plan. Dispatch prompt/report files remain untracked.

Reuse existing preference choices: ba=Pi tao-router/explore medium;
architect=Pi tao-router/thinking high; planner=Pi tao-router/thinking-high high;
implement and visual-engineering=Pi tao-router/code-writer medium;
tester-debugger=Pi tao-router/code-reviewer high. PM denotes current session,
not a launch. These are migrated preferences, not availability guarantees.

## Acceptance

- Exactly seven role rows; no active legacy routing. Reject substituted or
  duplicated roles in isolated negative fixtures.
- BA template includes substantive business fields and requirement traceability;
  planner includes task ownership, dependencies, proof and integration fields.
- Tester fix and acceptance sessions are distinct; same-session self-acceptance
  and stale-HEAD acceptance are explicitly forbidden.
- Parallel task contract rejects shared-checkout concurrent writers and unowned
  edits; no new runtime scheduling engine is introduced.
- Preserve custom legacy pins through documented migration; PM never dispatched.
- New template ships through delivery manifest; native installation rehearsal
  and premerge gates pass on committed payload.

Structural tests prove documentation shape and payload coverage, not future
agent obedience. Independent tester inspects adversarial workflow scenarios
and reproduces native proof; owner remains final human acceptance authority.
