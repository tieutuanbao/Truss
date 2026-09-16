#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
for command in cargo git rg; do command -v "$command" >/dev/null 2>&1 || { echo "pre-merge validation requires: $command" >&2; exit 1; }; done
bash -n scripts/*.sh tests/*.sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
tests/source-contract.sh
git diff --check
echo "pre-merge source contract passed"
