#!/usr/bin/env bash
# Shared setup for Buildkite steps. Sourced, not executed.
set -euo pipefail

# Hosted Linux agents are bare containers; macOS agents ship Xcode and brew.
# Only install what the workspace actually links against (dbus, via the
# keyring/secret-store path) so a step failure is about the code, not apt.
if [ "$(uname -s)" = "Linux" ]; then
  SUDO=""
  command -v sudo >/dev/null 2>&1 && SUDO="sudo"
  for i in 1 2 3 4 5; do
    $SUDO apt-get update && break
    echo "apt-get update failed (attempt $i); retrying in 15s" >&2
    sleep 15
  done
  $SUDO apt-get install -y --no-install-recommends \
    ca-certificates curl pkg-config libdbus-1-dev build-essential
fi

# rust-toolchain.toml pins `stable`; rustup honours it on first cargo call.
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable
fi
# shellcheck disable=SC1091
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

# Export toolchain homes explicitly so the unprivileged re-exec in test.sh can
# inherit them; rustup's default HOME-relative paths do not survive a user swap.
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export PATH="$CARGO_HOME/bin:$PATH"

cargo --version
rustc --version
