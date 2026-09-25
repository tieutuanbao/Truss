#!/usr/bin/env bash
# Delivery seven-role contract — structural guard.
#
# One command, runnable from the repository root:
#
#   bash tests/delivery-role-contract.sh
#
# It proves the delivered documentation shape, not future agent obedience:
# the approved seven roles, the current-session project-manager, the retired
# role set's absence from active tables, the BA/architect/planner template
# fields, and the delivery manifest membership of the new template.
#
# Authority: .truss-core/docs/decisions/0004-seven-role-delivery.md.
# The check prints one line per observation plus a final summary and exits
# non-zero when any observation fails. Negative proof mutates isolated
# temporary copies; the candidate working tree is never modified.
#
# Requirements: bash, python3, grep. No new framework or dependency.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO" || exit 1

for command in python3 grep; do
  command -v "$command" >/dev/null 2>&1 || {
    echo "delivery-role-contract requires: $command" >&2
    exit 1
  }
done

WORK="$(mktemp -d "${TMPDIR:-/tmp}/delivery-role-contract.XXXXXX")"
OK=0
BAD=0

cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

check() { # label exit-code
  local label="$1" cond="$2"
  if [ "$cond" = "0" ]; then
    printf 'ok   %s\n' "$label"
    OK=$((OK + 1))
  else
    printf 'FAIL %s\n' "$label"
    BAD=$((BAD + 1))
  fi
}

pos() { # label command...
  local label="$1"
  shift
  local out st
  out="$("$@" 2>&1)"
  st=$?
  check "$label" "$st"
  if [ "$st" != 0 ]; then
    printf '%s\n' "$out" | sed 's/^/       /'
  fi
}

neg() { # label expected-rule command...
  local label="$1" rule="$2"
  shift 2
  local out st
  out="$("$@" 2>&1)"
  st=$?
  if [ "$st" != 0 ] && printf '%s\n' "$out" | grep -q "$rule"; then
    check "$label" 0
  else
    check "$label" 1
    printf '%s\n' "$out" | sed 's/^/       /'
  fi
}

# The checker runs against configurable paths so the same instrument can judge
# isolated negative fixtures under $WORK. Defaults are the candidate files.
cat > "$WORK/contract.py" <<'PY'
import os
import re
import sys

CANONICAL = [
    "project-manager",
    "ba",
    "architect",
    "planner",
    "implement",
    "visual-engineering",
    "tester-debugger",
]
DISPATCHED = CANONICAL[1:]
LEGACY = ["plan", "plan-review", "implement", "review", "consult"]
LEGACY_MAP = {
    "plan": ["planner"],
    "plan-review": ["architect"],
    "implement": ["implement", "visual-engineering"],
    "review": ["tester-debugger"],
    "consult": ["ba"],
}

ROOT = os.environ.get("CONTRACT_ROOT", os.getcwd())
PATH = {
    "agents": os.environ.get("CONTRACT_AGENTS", os.path.join(ROOT, "AGENTS.md")),
    "delivery": os.environ.get("CONTRACT_DELIVERY_SKILL",
                               os.path.join(ROOT, ".agents/skills/delivery/SKILL.md")),
    "setup": os.environ.get("CONTRACT_SETUP_SKILL",
                            os.path.join(ROOT, ".agents/skills/delivery-setup/SKILL.md")),
    "plan": os.environ.get("CONTRACT_PLAN_TEMPLATE",
                           os.path.join(ROOT, ".agents/skills/delivery/templates/plan.md")),
    "decision": os.environ.get("CONTRACT_DECISION_TEMPLATE",
                               os.path.join(ROOT, ".agents/skills/delivery/templates/decision-record.md")),
    "ba": os.environ.get("CONTRACT_BA_TEMPLATE",
                         os.path.join(ROOT, ".agents/skills/delivery/templates/business-analysis.md")),
    "manifest": os.environ.get("CONTRACT_MANIFEST",
                               os.path.join(ROOT, "scripts/delivery-install-files.txt")),
    "authority": os.path.join(ROOT, ".truss-core/docs/decisions/0004-seven-role-delivery.md"),
    "workflow": os.environ.get(
        "CONTRACT_WORKFLOW", os.path.join(ROOT, ".truss-core/docs/WORKFLOW.md")),
    "plans_readme": os.environ.get(
        "CONTRACT_PLANS_README",
        os.path.join(ROOT, ".truss-core/docs/plans/README.md")),
    "assets_plans_readme": os.environ.get(
        "CONTRACT_ASSETS_PLANS_README",
        os.path.join(ROOT, "crates/truss/assets/.truss-core/docs/plans/README.md")),
}

PROBLEMS = []


def fail(rule, message):
    PROBLEMS.append(rule)
    print("%s: %s" % (rule, message))


def read(key):
    path = PATH[key]
    try:
        with open(path, encoding="utf-8") as handle:
            return handle.read()
    except OSError as error:
        fail("delivery-role-contract R1",
             "cannot read %s (%s); the seven-role contract requires this file" % (path, error))
        return None


def section(text, heading):
    """Lines of the section whose heading exactly equals `heading`."""
    lines = text.splitlines()
    start = None
    depth = None
    for index, line in enumerate(lines):
        if line.strip() == heading:
            start = index + 1
            depth = len(line) - len(line.lstrip("#"))
            break
    if start is None:
        return None
    body = []
    for line in lines[start:]:
        if re.match(r"^#{1,%d} " % depth, line):
            break
        body.append(line)
    return body


ROLE_ROW = re.compile(r"^\|\s*`([a-z][a-z0-9-]*)`\s*\|")


def role_rows(lines):
    rows = []
    for line in lines or []:
        match = ROLE_ROW.match(line)
        if match:
            cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
            rows.append((match.group(1), cells))
    return rows


def describe(rows):
    return "[" + ", ".join(role for role, _ in rows) + "]"


def expect_role_set(rule, label, rows, expected):
    roles = [role for role, _ in rows]
    if len(set(roles)) != len(roles):
        seen = set()
        dupes = sorted({r for r in roles if r in seen or seen.add(r)})
        fail(rule, "%s lists duplicated roles %s; each role appears exactly once" % (label, dupes))
        return False
    if roles != expected:
        fail(rule, "%s defines roles %s but the approved set is %s (%s); fix the role table"
             % (label, describe(rows), expected, PATH["authority"]))
        return False
    return True


def check_roles():
    agents = read("agents")
    if agents is not None:
        block = re.search(r"<!-- delivery:begin -->(.*?)<!-- delivery:end -->", agents, re.S)
        if not block:
            fail("delivery-role-contract R1",
                 "AGENTS.md has no complete delivery managed block; prerequisite 0004 defines it")
        else:
            rows = role_rows(block.group(1).splitlines())
            if expect_role_set("delivery-role-contract R1", "AGENTS.md delivery block", rows, CANONICAL):
                pm = dict((r, c) for r, c in rows)["project-manager"]
                if [pm[1], pm[2], pm[3]] != ["current", "current", "current"]:
                    fail("delivery-role-contract R1",
                         "project-manager cells are %s; the project-manager is the current session and is "
                         "never dispatched, so all three preference cells are `current`" % [pm[1], pm[2], pm[3]])

    delivery = read("delivery")
    if delivery is not None:
        rows = role_rows(section(delivery, "### Roles"))
        if expect_role_set("delivery-role-contract R1", "delivery SKILL ### Roles", rows, CANONICAL):
            for role, cells in rows:
                dispatch = cells[2]
                if role == "project-manager":
                    if "current" not in dispatch:
                        fail("delivery-role-contract R1",
                             "delivery SKILL project-manager dispatch cell is %r; it must name the current session"
                             % dispatch)
                elif "fresh dispatched worker" not in dispatch:
                    fail("delivery-role-contract R1",
                         "delivery SKILL %s dispatch cell is %r; it must be a fresh dispatched worker"
                         % (role, dispatch))
        legacy_rows = [line for line in delivery.splitlines()
                       if re.match(r"^\|\s*`(?:plan|plan-review|review|consult)`\s*\|", line)]
        if legacy_rows:
            fail("delivery-role-contract R1",
                 "delivery SKILL still declares retired role rows: %s; review is an activity, not a role"
                 % legacy_rows)

    setup = read("setup")
    if setup is not None:
        rows = role_rows(section(setup, "## Seven roles"))
        if expect_role_set("delivery-role-contract R1", "delivery-setup SKILL Seven roles", rows, CANONICAL):
            for role, cells in rows:
                dispatched = cells[2].lower()
                if role == "project-manager":
                    if not dispatched.startswith("no"):
                        fail("delivery-role-contract R1",
                             "delivery-setup project-manager row says dispatched=%r; it is the current session"
                             % cells[2])
                elif not dispatched.startswith("yes"):
                    fail("delivery-role-contract R1",
                         "delivery-setup %s row says dispatched=%r; it is a dispatched preference" % (role, cells[2]))


def check_templates():
    plan = read("plan")
    if plan is not None:
        required = [
            ("**Inputs.**", "R3", "each task states the approved artifacts it consumes"),
            ("**Outputs.**", "R3", "each task states the artifacts and behaviour it produces"),
            ("**Owned paths.**", "R3", "each task states its exact owned paths"),
            ("**Dependencies.**", "R3", "each task states its dependency DAG"),
            ("**Worktree / branch / base.**", "R3", "each task states its isolated worktree, branch and base"),
            ("**Wave.**", "R3", "each task states its parallel wave"),
            ("**Acceptance owner.**", "R3", "each task names a fresh tester-debugger acceptance session"),
            ("## Integration, waves, and recovery", "R3", "the plan states integration order, waves and recovery"),
            ("**Integration order:**", "R3", "the plan states the integration order and integrator"),
            ("**Conflict and recovery:**", "R3", "the plan states conflict and recovery handling"),
        ]
        for needle, rule, why in required:
            if needle not in plan:
                fail("delivery-role-contract %s" % rule,
                     "plan template is missing %r; %s (authority: %s)"
                     % (needle, why, PATH["authority"]))

    decision = read("decision")
    if decision is not None and "## Requirements traceability" not in decision:
        fail("delivery-role-contract R3",
             "decision-record template lacks ## Requirements traceability; the architect maps technical "
             "choices to BA requirement IDs (authority: %s)" % PATH["authority"])

    ba = read("ba")
    if ba is not None:
        required = [
            "## Goals", "## Actors", "## Business flows", "## Business rules", "## Exceptions",
            "## Scope and non-goals", "## Requirements", "## Acceptance", "## Planner handoff",
        ]
        for needle in required:
            if needle not in ba:
                fail("delivery-role-contract R2",
                     "business-analysis template is missing %r; the BA output must carry substantive "
                     "business fields and requirement traceability (authority: %s)"
                     % (needle, PATH["authority"]))
        if "REQ-" not in ba and "REQ-001" not in ba:
            fail("delivery-role-contract R2",
                 "business-analysis template defines no stable requirement ID shape such as REQ-001; "
                 "requirement IDs are required for traceability")


def check_manifest():
    text = read("manifest")
    if text is None:
        return
    paths = [line.strip() for line in text.splitlines()
             if line.strip() and not line.strip().startswith("#")]
    template = ".agents/skills/delivery/templates/business-analysis.md"
    if template not in paths:
        fail("delivery-role-contract R7",
             "delivery manifest does not ship %s; the new template must install through the delivery "
             "manifest (authority: %s)" % (template, PATH["authority"]))
    for entry in paths:
        if not os.path.isfile(os.path.join(ROOT, entry)):
            fail("delivery-role-contract R7",
                 "delivery manifest names %s but that file does not exist; fix the manifest or add the file"
                 % entry)


def check_migration():
    setup = read("setup")
    if setup is None:
        return
    rows = role_rows(section(setup, "## Migrating a legacy block"))
    found = {}
    for role, cells in rows:
        if role in LEGACY:
            targets = re.findall(r"`([a-z][a-z0-9-]*)`", cells[1])
            found[role] = targets
    missing = [role for role in LEGACY if role not in found]
    if missing:
        fail("delivery-role-contract R6",
             "delivery-setup migration table does not map retired role(s) %s; a custom legacy pin must not "
             "be lost silently (authority: %s)" % (missing, PATH["authority"]))
    for role, targets in found.items():
        if not targets:
            fail("delivery-role-contract R6",
                 "delivery-setup migration row %r names no target role; map it to the approved roles" % role)
            continue
        for target in targets:
            if target not in CANONICAL:
                hint = " (retired role)" if target in LEGACY else ""
                fail("delivery-role-contract R6",
                     "delivery-setup maps retired role %r to unknown role %r%s; the approved roles are %s"
                     % (role, target, hint, CANONICAL))
        expected = LEGACY_MAP.get(role, [])
        if expected and targets != expected:
            fail("delivery-role-contract R6",
                 "delivery-setup maps retired role %r to %s but the approved migration is %s"
                 % (role, targets, expected))
    migration_text = "\n".join(section(setup, "## Migrating a legacy block") or [])
    if "`project-manager`" not in migration_text or "current" not in migration_text:
        fail("delivery-role-contract R6",
             "delivery-setup migration does not introduce the project-manager row with `current` cells")


PRIVATE_RUN_DIR = ".truss/delivery-runs/<run-key>/"
APPROVAL_RECEIPT = ".truss/authority/approvals/<run-key>.md"
SNAPSHOT_FILE = "approved-envelope.md"

# Decision 0006 terms the delivery skill must carry for a consumer-local run.
R8_REQUIRED = [
    PRIVATE_RUN_DIR,
    APPROVAL_RECEIPT,
    SNAPSHOT_FILE,
    "consumer-local",
    "must not be staged",
    "blocks acceptance",
    "info/exclude",
]

# The unconditional commitment of the transient plan, superseded for a
# consumer-local run by decision 0006.
R9_FORBIDDEN = "Commit them before\nimplementation begins"


def check_envelope():
    skill = read("delivery")
    if skill is None:
        return
    for token in R8_REQUIRED:
        if token not in skill:
            fail("delivery-role-contract R8",
                 "delivery SKILL.md does not name %r; decision 0006 requires the run "
                 "path, the approval receipt, the immutable snapshot, the exclude "
                 "prerequisite, and the fail-closed acceptance rule" % token)
    if R9_FORBIDDEN in skill:
        fail("delivery-role-contract R9",
             "delivery SKILL.md still commits the transient plan unconditionally; "
             "decision 0006 makes that conditional on a consumer-local run")
    plan = read("plan")
    ba = read("ba")
    decision = read("decision")
    for text in (plan, ba, decision):
        if text is None:
            return
    if PRIVATE_RUN_DIR not in plan or "never committed" not in plan:
        fail("delivery-role-contract R10",
             "templates/plan.md does not name %r and the never-committed rule; "
             "decision 0006 fixes the private run path" % PRIVATE_RUN_DIR)
    for path, text in ((".agents/skills/delivery/templates/business-analysis.md", ba),
                       (".agents/skills/delivery/templates/decision-record.md", decision)):
        if "consumer-local" not in text or "never committed" not in text:
            fail("delivery-role-contract R10",
                 "%s does not state the consumer-local never-committed rule of "
                 "decision 0006" % path)
    workflow = read("workflow")
    plans_readme = read("plans_readme")
    assets_readme = read("assets_plans_readme")
    if workflow is None or plans_readme is None or assets_readme is None:
        return
    if PRIVATE_RUN_DIR not in workflow or "never committed" not in workflow:
        fail("delivery-role-contract R11",
             "WORKFLOW.md does not state the private run path and the "
             "never-committed rule of decision 0006")
    if PRIVATE_RUN_DIR not in plans_readme:
        fail("delivery-role-contract R11",
             "the plans README does not distinguish the transient private path "
             "%r from the durable plan location" % PRIVATE_RUN_DIR)
    if plans_readme != assets_readme:
        fail("delivery-role-contract R12",
             "the two plans README copies differ; embedding reads the "
             "crates/truss/assets copy, so divergence changes what consumers "
             "receive without either file looking wrong")


def main():
    command = sys.argv[1] if len(sys.argv) > 1 else "all"
    if command in ("roles", "all"):
        check_roles()
    if command in ("templates", "all"):
        check_templates()
    if command in ("manifest", "all"):
        check_manifest()
    if command in ("migration", "all"):
        check_migration()
    if command in ("envelope", "all"):
        check_envelope()
    if PROBLEMS:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
PY

CONTRACT="$WORK/contract.py"

echo "delivery role contract: $(git rev-parse HEAD 2>/dev/null || echo 'no git HEAD')"
echo

# ------------------------------------------------------------- positive ----
pos "AGENTS delivery block, delivery role table, and setup role table define exactly the seven approved roles" \
  python3 "$CONTRACT" roles
pos "BA, plan, and decision templates carry the required business, task, and traceability fields" \
  python3 "$CONTRACT" templates
pos "the delivery manifest ships the new BA template and every declared path exists" \
  python3 "$CONTRACT" manifest
pos "the legacy five-role block maps onto the approved seven roles without losing a pin" \
  python3 "$CONTRACT" migration
check "scripts/validate-premerge.sh invokes this focused contract check" \
  "$(grep -q 'tests/delivery-role-contract.sh' scripts/validate-premerge.sh; echo $?)"
echo

# ------------------------------------------------------------- negative ----
# Present-but-wrong fixtures: still seven shipped-looking rows, wrong meaning.
python3 - "$REPO/AGENTS.md" "$WORK/ng1-agents.md" <<'PY'
import re
import sys
text = open(sys.argv[1], encoding="utf-8").read()
mutated = re.sub(r"(?m)^\| `ba` \|", "| `review` |", text, count=1)
open(sys.argv[2], "w", encoding="utf-8").write(mutated)
PY
neg 'a seven-row block whose `ba` row is renamed `review` is rejected' "R1" \
  env CONTRACT_AGENTS="$WORK/ng1-agents.md" python3 "$CONTRACT" roles

python3 - "$REPO/AGENTS.md" "$WORK/ng2-agents.md" <<'PY'
import re
import sys
text = open(sys.argv[1], encoding="utf-8").read()
pm = re.search(r"(?m)^\| `project-manager` \|.*$", text).group(0)
open(sys.argv[2], "w", encoding="utf-8").write(text.replace(pm, pm + "\n" + pm, 1))
PY
neg "a duplicated project-manager row is rejected" "R1" \
  env CONTRACT_AGENTS="$WORK/ng2-agents.md" python3 "$CONTRACT" roles

python3 - "$REPO/.agents/skills/delivery/SKILL.md" "$WORK/ng3-delivery.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(
    text + "\n| `consult` | a domain question | fresh dispatched worker |\n")
PY
neg "a reintroduced retired role row in the delivery skill is rejected" "R1" \
  env CONTRACT_DELIVERY_SKILL="$WORK/ng3-delivery.md" python3 "$CONTRACT" roles

grep -v 'business-analysis.md' "$REPO/scripts/delivery-install-files.txt" > "$WORK/ng4-manifest.txt"
neg "a manifest that drops the new BA template is rejected" "R7" \
  env CONTRACT_MANIFEST="$WORK/ng4-manifest.txt" python3 "$CONTRACT" manifest

python3 - "$REPO/.agents/skills/delivery-setup/SKILL.md" "$WORK/ng5-setup.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(
    text.replace("| `review` | `tester-debugger` |", "| `review` | `consult` |", 1))
PY
neg "a migration row that maps a retired role to a retired role is rejected" "R6" \
  env CONTRACT_SETUP_SKILL="$WORK/ng5-setup.md" python3 "$CONTRACT" migration

python3 - "$REPO/.agents/skills/delivery/SKILL.md" "$WORK/ng6-skill.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(
    text.replace(".truss/authority/approvals/<run-key>.md", "the approval receipt", 1))
PY
neg "a skill that drops the approval receipt path is rejected" "R8" \
  env CONTRACT_DELIVERY_SKILL="$WORK/ng6-skill.md" python3 "$CONTRACT" envelope

python3 - "$REPO/.agents/skills/delivery/SKILL.md" "$WORK/ng7-skill.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(
    text + "\nCommit them before\nimplementation begins — that commit is the acceptance baseline.\n")
PY
neg "a skill that commits the transient plan unconditionally is rejected" "R9" \
  env CONTRACT_DELIVERY_SKILL="$WORK/ng7-skill.md" python3 "$CONTRACT" envelope

python3 - "$REPO/.agents/skills/delivery/templates/plan.md" "$WORK/ng8-plan.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(
    text.replace(".truss/delivery-runs/", "the run directory"))
PY
neg "a plan template without the private run path is rejected" "R10" \
  env CONTRACT_PLAN_TEMPLATE="$WORK/ng8-plan.md" python3 "$CONTRACT" envelope

python3 - "$REPO/.agents/skills/delivery/templates/decision-record.md" "$WORK/ng9-decision.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(
    text.replace("never committed", "usually committed"))
PY
neg "a decision template that drops the never-committed rule is rejected" "R10" \
  env CONTRACT_DECISION_TEMPLATE="$WORK/ng9-decision.md" python3 "$CONTRACT" envelope

python3 - "$REPO/.truss-core/docs/WORKFLOW.md" "$WORK/ng10-workflow.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(text.replace("never committed", "committed with the candidate"))
PY
neg "a workflow that drops the never-committed rule is rejected" "R11" \
  env CONTRACT_WORKFLOW="$WORK/ng10-workflow.md" python3 "$CONTRACT" envelope

python3 - "$REPO/.truss-core/docs/plans/README.md" "$WORK/ng11-readme.md" <<'PY'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
open(sys.argv[2], "w", encoding="utf-8").write(text.replace("Execution plans", "Plan notes", 1))
PY
neg "a diverged plans README copy is rejected" "R12" \
  env CONTRACT_ASSETS_PLANS_README="$WORK/ng11-readme.md" python3 "$CONTRACT" envelope

pos "the repository delivery skill carries the consumer-local envelope contract" \
  python3 "$CONTRACT" envelope
echo

echo "== delivery-role-contract summary: $OK ok, $BAD failed =="
[ "$BAD" = 0 ]
