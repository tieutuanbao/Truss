#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
for command in cargo git rg python3; do command -v "$command" >/dev/null 2>&1 || { echo "pre-merge validation requires: $command" >&2; exit 1; }; done
bash -n scripts/*.sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
# The payload layout contract is a Cargo integration test
# (crates/truss/tests/payload_layout.rs), so `cargo test` runs it: it proves the
# layout marker, one-source resolution per destination, manifest membership
# both ways, the generator-input boundary, the reviewed byte baseline for every
# shipped payload file, entrypoint parity with scripts/, and the absence of the
# legacy mirror directory. Digest coverage is the drift guard because the binary
# embeds distribution/payload with include_bytes! at compile time.
git diff --check
echo "pre-merge validation passed"
