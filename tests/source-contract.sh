#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
for forbidden in "repository"-"truss" "hoang""nb24" "de""ly" "De""ly" "Truss ""v0" "protocol ""v1" "truss"-"cli"; do
  if rg -n -i --hidden --glob '!target/**' --glob '!tests/source-contract.sh' -- "$forbidden" .; then
    echo "forbidden legacy identity: $forbidden" >&2
    exit 1
  fi
done
while IFS= read -r path; do
  [[ -z "$path" || "$path" == \#* ]] || [[ -f "$path" ]] || { echo "manifest path missing: $path" >&2; exit 1; }
done < scripts/truss-install-files.txt
bash -n scripts/install-truss.sh scripts/validate-premerge.sh
echo "source contract passed"
