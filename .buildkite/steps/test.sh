#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
# shellcheck source=/dev/null
. .buildkite/steps/common.sh

# nextest profile `ci` lives in .config/nextest.toml alongside the test-group
# bounds that serialize the binary-spawning integration suites.
if ! command -v cargo-nextest >/dev/null 2>&1; then
  cargo install cargo-nextest --locked --version 0.9.* || cargo install cargo-nextest --locked
fi

echo "--- workspace tests"
cargo nextest run --workspace --all-features --locked --profile ci

echo "--- doctests"
cargo test --workspace --all-features --locked --doc
