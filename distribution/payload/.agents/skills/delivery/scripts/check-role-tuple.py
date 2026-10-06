#!/usr/bin/env python3
"""Read-only comparison of current Delivery pins with the approved envelope."""

import argparse
import json
from pathlib import Path
import re
import sys


ROLES = {
    "project-manager", "ba", "architect", "detailed-designer", "planner",
    "implement", "visual-engineering", "tester", "debugger",
}
AUTHORITY = "delivery/references/execution.md, Pre-dispatch role-tuple check"


def read(path):
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise ValueError(f"{path}: unreadable input: {error}") from error


def cells(line):
    if not line.strip().startswith("|") or not line.strip().endswith("|"):
        return []
    return [cell.strip().strip("`") for cell in line.strip()[1:-1].split("|")]


def truss_names():
    path = Path(__file__).resolve().parent.parent / "references/trusses.md"
    names = {}
    for line in read(path).splitlines():
        row = cells(line)
        if len(row) == 4 and row[1] in {
            "claude", "codex", "cursor", "antigravity", "pi", "opencode",
            "zcode", "grok", "kiro", "copilot",
        }:
            if row[0] in names:
                raise ValueError(f"{path}: duplicate Truss name {row[0]!r}")
            names[row[0]] = row[1]
    return names


def tuples(path, names, managed=False):
    text = read(path)
    if managed:
        begin, end = "<!-- delivery:begin -->", "<!-- delivery:end -->"
        if text.count(begin) != 1 or text.count(end) != 1 or text.index(end) < text.index(begin):
            raise ValueError(f"{path}: managed block must have one ordered begin/end pair")
        text = text.split(begin, 1)[1].split(end, 1)[0]
    lines = text.splitlines()
    # Validate role identity throughout the record, including rows separated
    # from the canonical table by blank lines or explanatory text.
    roles = [row[0] for line in lines if (row := cells(line))]
    if "tester-debugger" in roles:
        raise ValueError(f"DELIVERY_ROLE_CONFIG_MIGRATION_REQUIRED: {path}: retired tester-debugger; use delivery-setup")
    for role in sorted(ROLES):
        if roles.count(role) > 1:
            raise ValueError(f"{path}: duplicate role {role!r}")
    headers = [i for i, line in enumerate(lines) if cells(line) == ["Role", "Truss", "Model", "Effort"]]
    if len(headers) != 1:
        raise ValueError(f"{path}: exactly one Role/Truss/Model/Effort table is required")
    start = headers[0] + 1
    if start >= len(lines) or len(cells(lines[start])) != 4 or not all(
        re.fullmatch(r":?-{3,}:?", cell) for cell in cells(lines[start])
    ):
        raise ValueError(f"{path}: malformed role table separator")
    found = {}
    table_lines = {headers[0], start}
    for index, line in enumerate(lines[start + 1:], start + 1):
        if not line.strip().startswith("|"):
            break
        table_lines.add(index)
        row = cells(line)
        if len(row) != 4:
            raise ValueError(f"{path}: malformed role table row {line!r}")
        role, truss, model, effort = row
        if role not in ROLES:
            raise ValueError(f"{path}: unknown role {role!r}")
        if role in found:
            raise ValueError(f"{path}: duplicate role {role!r}")
        if any(not value or "…" in value or "..." in value or "<" in value or ">" in value for value in row):
            raise ValueError(f"{path}: {role}: unresolvable role tuple {row!r}")
        if role == "project-manager":
            if (truss, model, effort) != ("current", "current", "current"):
                raise ValueError(f"{path}: project-manager must remain current/current/current")
            normalized = "current"
        else:
            if truss not in names:
                raise ValueError(f"{path}: {role}: unrecognized Truss {truss!r}; use an exact documented name")
            if model == "current" or effort == "current":
                raise ValueError(f"{path}: {role}: unresolvable worker model or effort")
            normalized = names[truss]
        found[role] = {"role": role, "truss": normalized, "model": model, "effort": effort}
    if managed and any(
        line.strip().startswith("|") and index not in table_lines
        for index, line in enumerate(lines)
    ):
        raise ValueError(f"{path}: malformed role table row outside the configured table")
    missing = sorted(ROLES - found.keys())
    if missing:
        raise ValueError(f"{path}: missing role rows: {', '.join(missing)}")
    return found


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--agents", type=Path, required=True)
    parser.add_argument("--approved", type=Path, required=True)
    parser.add_argument("--role", required=True)
    args = parser.parse_args()
    try:
        if args.role not in ROLES or args.role == "project-manager":
            raise ValueError(f"target role {args.role!r} is not a dispatched Delivery role")
        names = truss_names()
        observed = tuples(args.agents, names, managed=True)[args.role]
        approved = tuples(args.approved, names)[args.role]
        if observed != approved:
            raise ValueError(
                f"DELIVERY_ROLE_TUPLE_CHANGED: {args.agents}: {args.role}: "
                f"observed {observed!r}; approved {approved!r} in {args.approved}; "
                "stop this dispatch; only a later delivery may adopt changed pins"
            )
    except ValueError as error:
        print(
            f"DELIVERY_PREFLIGHT_BLOCKED: {error}. Authority: {AUTHORITY}. "
            "Resolve configuration with delivery-setup or repair the approved envelope "
            "under its existing approval boundary; do not dispatch.",
            file=sys.stderr,
        )
        return 2
    print(json.dumps({"tuple": observed, "agents": str(args.agents), "approved": str(args.approved)}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
