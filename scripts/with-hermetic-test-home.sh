#!/bin/sh
# Shared test-home boundary extracted from test-tui-hermetic.sh. Both that
# runner and Buildkite workspace tests use it; Rust toolchain homes stay real.
set -eu

if [ "$#" -eq 0 ]; then
  printf '%s\n' 'usage: with-hermetic-test-home.sh command [args...]' >&2
  exit 2
fi

real_cargo_home=${CARGO_HOME:-${HOME}/.cargo}
real_rustup_home=${RUSTUP_HOME:-${HOME}/.rustup}
rustc_bin=$(RUSTUP_HOME="$real_rustup_home" rustup which rustc)
toolchain_bin=${rustc_bin%/*}
test_home_root=$(mktemp -d "${TMPDIR:-/tmp}/codewhale-test-home.XXXXXX")
trap 'rm -rf -- "$test_home_root"' EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

mkdir -p "$test_home_root/home/.codewhale" "$test_home_root/xdg"
mkdir -p "$test_home_root/codex" "$test_home_root/grok" "$test_home_root/kimi-code"
mkdir -p "$test_home_root/kimi-share" "$test_home_root/claude"

# Use the isolated HOME default, while allowing each test to choose its own
# home or config fixture. Canonical overrides would shadow legacy fixtures.
unset CODEWHALE_HOME CODEWHALE_CONFIG_PATH DEEPSEEK_CONFIG_PATH DEEPSEEK_HOME

env \
  HOME="$test_home_root/home" \
  USERPROFILE="$test_home_root/home" \
  XDG_CONFIG_HOME="$test_home_root/xdg" \
  CODEX_HOME="$test_home_root/codex" \
  GROK_HOME="$test_home_root/grok" \
  GROK_AUTH_PATH="$test_home_root/grok/auth.json" \
  KIMI_CODE_HOME="$test_home_root/kimi-code" \
  KIMI_SHARE_DIR="$test_home_root/kimi-share" \
  CLAUDE_CONFIG_DIR="$test_home_root/claude" \
  DEEPSEEK_API_KEY= \
  OPENAI_API_KEY= \
  ANTHROPIC_API_KEY= \
  XAI_API_KEY= \
  GROK_API_KEY= \
  MOONSHOT_API_KEY= \
  KIMI_API_KEY= \
  XIAOMI_MIMO_API_KEY= \
  XIAOMI_MIMO_TOKEN_PLAN_API_KEY= \
  MIMO_API_KEY= \
  MIMO_TOKEN_PLAN_API_KEY= \
  RUST_MIN_STACK="${RUST_MIN_STACK:-16777216}" \
  CARGO_HOME="$real_cargo_home" \
  RUSTUP_HOME="$real_rustup_home" \
  PATH="$toolchain_bin:$PATH" \
  "$@"
