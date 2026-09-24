#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
for command in cargo git rg python3; do command -v "$command" >/dev/null 2>&1 || { echo "pre-merge validation requires: $command" >&2; exit 1; }; done
bash -n scripts/*.sh tests/*.sh
bash tests/delivery-role-contract.sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
while IFS= read -r manifest; do
  while IFS= read -r path; do
    [[ -z "$path" || "$path" == \#* ]] || [[ -f "$path" ]] || { echo "manifest path missing: $path ($manifest)" >&2; exit 1; }
  done < "$manifest"
done < <(find scripts -name '*-install-files.txt' | sort)
# Committed S5 rehearsal: installer delegation and platform parity. Offline and
# deterministic; it builds the CLI if needed, then exercises both installers
# against throwaway workspaces.
bash tests/s5-rehearse.sh
git diff --check
echo "pre-merge validation passed"
