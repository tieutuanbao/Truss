# Truss Improvement: Delivery candidate-worktree default

Date: 2026-09-24

## Status

Completed

## Representative Job

Run `$delivery` for a bounded change in the consumer repository
`/media/bitscat/data/Workspace/Bitscat/MeBe`, with role pins for Codex from the
consumer's `AGENTS.md` delivery block, and have the Control session decide where
the candidate is implemented.

- Trajectory: `/home/bitscat/.pi/agent/sessions/--media-bitscat-data-Workspace-Bitscat-MeBe--/2026-09-24T06-33-24-359Z_01a0d21e-6c86-7068-a96a-ab996b3ad3e3.jsonl`
- Consumer revision at the time: `main` at `e2f938765e85bbc07d6ecdeeb779cc38eca9ca81` (delivery baseline recorded in that run's envelope).
- External state: Orca CLI `orca-ide` 1.4.210; Orca-managed worktrees live under
  `/home/bitscat/orca/workspaces/<project>/<name>` (`orca worktree create --help`).
- Authority: owner asked for the delivery in that session; owner invoked
  `$improve-truss` here for this bounded experiment.
- Stop condition for this experiment: one applied intervention plus a fresh
  equivalent rerun.

## Baseline

Owner request in the MeBe session: "Hãy delivery với tên model và role như
AGENTS.md đã ghi".

Observed failure: Control created a new Orca-managed checkout with
`orca worktree create --repo path:/media/bitscat/data/Workspace/Bitscat/MeBe
--name so-bau-slice-1 --base-branch main`, then ran the whole delivery there, at
`/home/bitscat/orca/workspaces/MeBe/so-bau-slice-1`. The consumer checkout stayed
on `main` without the application code.

Control presented that relocation as a hard rule of the skill: "Quy trình
`$delivery` ... có một quy định cứng: nơi thi hành phải là Orca, và Orca tạo
**một bản checkout riêng cho mỗi lần delivery**, gọi là worktree, trên một nhánh
riêng. Đây là quy định của quy trình, không phải lựa chọn của tôi." Its dispatch
handoff recorded `Repository:` and `Worktree:` as the Orca path.

Human steering and recovery: the owner asked why the delivery was not done in the
current directory, then asked which document line says "nơi thi hành phải là
Orca, và Orca tạo một bản checkout riêng cho mỗi lần delivery", then closed the
run manually with "Hợp nhất so-bau-slice-1 vào main và xóa worktree".

Discriminating control measurement, taken on the unmodified installed copy at
`/media/bitscat/data/Workspace/Bitscat/MeBe/.agents/skills/delivery/SKILL.md`
(byte-identical to the source before this change, `md5 16aefcfc03f0c6966c9f2c8f6a6cb210`):
a fresh agent asked whether relocation is Control's decision answered
"**Control's own decision** — envelope is Control-resolved" and confirmed no
worktree-creation gate exists, citing the envelope paragraph and the absence of
any authorization rule. The old text therefore licensed exactly the behaviour
that occurred; the misreport as a "quy định cứng" is also a reasoning defect
under that text.

Earlier probes in this investigation asked only "where does the work live" and
all three answered the consumer Git root. They did not exercise the relocation
decision, which is why the failure looked unreproducible until the MeBe
trajectory was located.

## Earliest Gap

Context and authority, in `.agents/skills/delivery/SKILL.md` § Execution
envelope. The envelope required a "Git target: worktree, ..." but never named the
candidate worktree's default, never said relocation is the owner's decision, and
never listed it among the stop conditions. Read together with § Orca is mandatory
("Orca is the required execution plane"), a Control session could resolve
"worktree" to Orca's own managed checkout and treat that choice as mandated.
The knowledge existed only as an inference from `SKILL.md:227` ("Resolve the
requested repository and worktree to this exact Git root and baseline"), which
does not distinguish the consumer checkout from a new Orca-managed one.

Not the gap: Orca's own behaviour. Orca-managed worktrees under
`/home/bitscat/orca/workspaces/...` are ordinary git linked worktrees of the same
repository, so the failure is not "a different repository"; it is an
unauthorized relocation of the working copy.

## Correct Owner

Truss source: `.agents/skills/delivery/SKILL.md`, § Execution envelope and
§ Orca is mandatory. `references/trusses.md:10-12` already required the Run
selector to resolve to the validated consumer root and needed no change.

## Intervention

```text
If the envelope names the candidate worktree default (the consumer checkout) and
reserves relocation to owner authorization at gate 1, then a fresh Control
session will deliver in the consumer checkout unless the approved envelope names
another path, because the section that makes Orca mandatory also states that its
mandatory role is dispatch and supervision, not candidate location.
```

Applied, in `.agents/skills/delivery/SKILL.md` (+34/−14):

1. § Execution envelope, opening paragraph: validates the consumer worktree's
   exact Git root and baseline, and requires the Orca Run to select that worktree.
2. § Execution envelope, new paragraph: the candidate worktree defaults to the
   consumer checkout; a separate checkout, including an Orca-managed worktree
   under the application's own workspace root, is neither required nor a default;
   Control never relocates on its own judgement; relocation requires the approved
   envelope to authorize it, with the path named in the design contract and
   approved at gate 1 before any dispatch.
3. Git target bullet: "the candidate worktree and its exact Git root, branch,
   baseline, base, remote, and pull-request target", plus the consumer-checkout
   default.
4. § Orca is mandatory: heading retained; the body now states Orca is the required
   dispatch and supervision plane, that mandatory does not name a different
   repository, and that no Orca capability, convenience, or default workspace
   path relocates the candidate. The no-direct-dispatch and no-headless-fallback
   sentences are preserved.
5. Preflight warning: do not infer the target from "an Orca-managed checkout path
   the envelope did not authorize".
6. Failure and recovery table: a row for "The candidate location would move out of
   the consumer checkout" — not a Control decision; present it at gate 1 with the
   named path, or deliver in the consumer checkout.

Evidence that would weaken this intervention: a fresh agent that, under the old
text, refused to relocate without owner authorization or that reported the
mandate as unsettled rather than as a rule. The control measurement above shows
the old text said the opposite, so the intervention is not redundant.

Maintenance owner: this repository, `SKILL.md` § Execution envelope and
§ Orca is mandatory. Removal condition: remove if two fresh reruns show the
authorization gate changes no answer, or if a later mechanism (an envelope
schema, an installer-time check) enforces the same fact.

## Native Validation

`scripts/validate-premerge.sh`:

- With the payload uncommitted, the installers refuse by design — "the delivery
  add-on payload has uncommitted changes ...; commit or discard them first,
  because an immutable `--source-ref` must describe the bytes that are
  installed" — and `tests/s5-rehearse.sh` reports 37 ok, 19 failed. All 19
  failures trace to that refusal (missing `.truss-core/addons.json`), not to the
  edit.
- With the payload committed to a temporary commit, the same command exits 0:
  `cargo fmt --check`, `cargo test --workspace --locked`, `cargo clippy
  --workspace --all-targets --locked -D warnings`, the install-manifest path
  check, `git diff --check`, and the S5 rehearsal "56 ok, 0 failed" all pass.
  That commit was then removed with `git reset --soft HEAD~1`, so `main` HEAD is
  back at `91aa10f863151afe1f568365bf257eb53478fceb` and the change remains
  uncommitted in the working tree for the owner's promotion decision.

No installer manifest, decision record, or contract changed; no consumer
repository was modified.

## Fresh Rerun

Equivalent fresh agents, read-only, same probe wording, differing only in which
copy of the skill they read.

| Question | Control: unmodified installed copy | Rerun: this repository's copy |
| --- | --- | --- |
| Is a separate Orca-managed checkout required per delivery? | No; never stated | No; `SKILL.md:160-162` |
| Is relocation Control's decision or the owner's? | **Control's own decision**; no gate | **Owner must authorize**; `SKILL.md:163-165`, failure table `SKILL.md:599` |
| Does the skill stop without a separate worktree? | No | No; default is the consumer checkout (`SKILL.md:160`, `166`) |

A second rerun with the MeBe consumer scenario answered the candidate directory
as `/media/bitscat/data/Workspace/Bitscat/MeBe`, citing the new default and the
gate-1 authorization requirement.

Intervention status: available, retrieved, and relevant. The changed dimension is
the authorization gate and the removal of the "hard requirement" framing.
Measured limitation: the fresh agent under the old text already chose the
consumer checkout when asked directly for the directory, so the default-path
answer alone would not demonstrate improvement; the authorization answer did
change from "Control's own decision" to "owner must authorize".

## Decision

Keep.

Reason: the rerun exercised the intervention and changed the answer on the exact
dimension that failed (who may relocate the working copy), at a cost of a small,
contained wording change with no behavioural claim beyond that dimension. The
earlier three probes are retained above as the reason the default-path wording is
defended by reasoning rather than by a reproduced default-path failure.

## Result

The delivery skill now states where the candidate lives, makes the consumer
checkout the default, and reserves any relocation to the owner at gate 1, while
keeping Orca mandatory as the dispatch and supervision plane and keeping both
human gates and the no-headless-fallback rule unchanged.

Limitations: one consumer trajectory and a small number of fresh probes; the
intervention's effect on a full multi-task delivery run was not exercised. The
reported "quy định cứng" claim was a Control reasoning defect as well as a text
gap, and this change only removes the textual licence.

Follow-up, in order:

1. Done: the owner authorized commit and release. Commits `04cffc4`
   (skill), `edaae7a` (tag `truss-v0.1.15`), `68a4b34` (rehearsal alignment), all
   on `main` and pushed.
2. Open: consumer copies still carry the old skill; the MeBe copy is still
   `md5 16aefcfc03f0c6966c9f2c8f6a6cb210`. Update with `truss update` or a fresh
   install, then re-check the hash.
3. Done: `scripts/validate-premerge.sh` exits 0 at `68a4b34` with the S5
   rehearsal "56 ok, 0 failed".
4. Optional second fresh rerun on a real delivery request in a consumer project to
   confirm no relocation without authorization.

## Release

Release `truss-v0.1.15`, published 2026-09-24 as latest with assets
`truss-linux-x64` and `truss-linux-x64.sha256`.

- Tag target `edaae7a51cc590e9a7d92ccc80ac8ac40cb1dd5d`; downloaded asset
  verifies against its published sidecar and reports `truss 0.1.15`.
- The pointer at `main` resolves to `truss-v0.1.15`, so `truss update` and
  bootstrap see the new release.
- README release gate passed: the tagged checkout and the raw source base URL
  pinned to the same tag produced byte-identical add-on trees and identical
  `addons.json`, each add-on recording `source_ref=truss-v0.1.15`.

Two release-process defects surfaced while proving this release.

1. The previous arrangement — bump commit, tag on it, pointer commit after —
   cannot satisfy the README post-tag gate. At the tag, the pointer names the
   previous release, so a raw install pinned to the tag stops: with
   `truss-v0.1.14`, `raw .../truss-v0.1.14/scripts/truss-release-tag` declares
   `truss-v0.1.13` and the installer fails with "the raw source base URL pins
   truss-v0.1.14 but scripts/truss-release-tag declares 'truss-v0.1.13'".
   This release carries the pointer in the tagged commit instead, so the tag's
   tree declares itself; both halves of the gate then agree.
2. `tests/s5-rehearse.sh` expected the resolved HEAD commit SHA unconditionally,
   which is impossible on a release commit whose HEAD carries the declared tag.
   The expected ref now mirrors the installer's own rule in
   `resolve_local_addon_source_ref`: the release tag when a tag with that name
   points at HEAD, otherwise the exact HEAD SHA. Proof: the rehearsal is green
   with the tag at HEAD (run before the alignment commit) and green at the
   untagged commit `68a4b34` afterwards; `tests/` is not in any install
   manifest, so released artifacts are unaffected and no new release is needed
   for the alignment.

Retained artifacts: this record. Removed artifacts: the temporary validation
commit on `main` (removed; `HEAD` restored).
