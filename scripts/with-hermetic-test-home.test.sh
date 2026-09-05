#!/bin/sh
# Exercise the real boundary using only synthetic homes and a fake toolchain.
# Quoted child programs and the literal argv probe expand only in the child.
# shellcheck disable=SC2016
set -eu
repo_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
fixture=$(mktemp -d "${TMPDIR:-/tmp}/hermetic-home-proof.XXXXXX")
trap 'rm -rf -- "$fixture"' EXIT

mkdir -p "$fixture/bin" "$fixture/toolchain" "$fixture/tmp with spaces"
mkdir -p "$fixture/outer/home/.codewhale/fleets" "$fixture/cargo" "$fixture/rustup"
printf '%s\n' 'My fleet' > "$fixture/outer/home/.codewhale/fleets/selected"
printf '%s\n' '[invalid fixture' > "$fixture/outer/home/.codewhale/fleets/my-fleet.toml"
printf '%s\n' '#!/bin/sh' 'printf "%s\n" "$TEST_TOOLCHAIN/rustc"' > "$fixture/bin/rustup"
chmod +x "$fixture/bin/rustup"
fixture_stack=20971520

run_isolated() {
  env HOME="$fixture/outer/home" \
    CODEWHALE_HOME="$fixture/outer/home/.codewhale" \
    CODEWHALE_CONFIG_PATH="$fixture/outer/poison.toml" \
    DEEPSEEK_CONFIG_PATH="$fixture/outer/legacy-poison.toml" \
    DEEPSEEK_HOME="$fixture/outer/legacy-home" \
    CODEX_HOME="$fixture/outer/codex" \
    OPENAI_API_KEY=synthetic-outer-key \
    RUST_MIN_STACK="$fixture_stack" \
    CARGO_HOME="$fixture/cargo" RUSTUP_HOME="$fixture/rustup" \
    TEST_TOOLCHAIN="$fixture/toolchain" TMPDIR="$fixture/tmp with spaces" \
    PATH="$fixture/bin:$PATH" \
    "$repo_root/scripts/with-hermetic-test-home.sh" "$@"
}

run_isolated sh -c '
  set -eu
  test "$HOME" != "$1/outer/home"
  test "$USERPROFILE" = "$HOME"
  test -d "$HOME/.codewhale"
  test ! -e "$HOME/.codewhale/fleets/selected"
  test -z "${CODEWHALE_HOME+x}"
  test -z "${CODEWHALE_CONFIG_PATH+x}"
  test -z "${DEEPSEEK_CONFIG_PATH+x}"
  test -z "${DEEPSEEK_HOME+x}"
  test "$CODEX_HOME" != "$1/outer/codex"
  test -d "$XDG_CONFIG_HOME"
  test -d "$CODEX_HOME"
  test -z "$OPENAI_API_KEY"
  test "$RUST_MIN_STACK" = 20971520
  test "$CARGO_HOME" = "$1/cargo"
  test "$RUSTUP_HOME" = "$1/rustup"
  case "$PATH" in "$1/toolchain:"*) ;; *) exit 1 ;; esac
  test "$2" = '\''one argument $(not run)'\''
  printf "%s\n" "$HOME" > "$1/child-home"
' sh "$fixture" 'one argument $(not run)'
child_home=$(cat "$fixture/child-home")
test ! -d "${child_home%/*}"
test "$(cat "$fixture/outer/home/.codewhale/fleets/selected")" = 'My fleet'
test "$(cat "$fixture/outer/home/.codewhale/fleets/my-fleet.toml")" = '[invalid fixture'
printf '%s\n' 'ok 1 - isolated homes, credentials, argv, toolchain and outer state'

status=0
fixture_stack=
run_isolated sh -c 'set -e; test "$RUST_MIN_STACK" = 16777216; printf "%s\n" "$HOME" > "$1/failing-home"; exit 37' sh "$fixture" || status=$?
test "$status" -eq 37
child_home=$(cat "$fixture/failing-home")
test ! -d "${child_home%/*}"
test -f "$fixture/outer/home/.codewhale/fleets/selected"
printf '%s\n' 'ok 2 - child failure status and owned-home cleanup'

status=0
run_isolated > "$fixture/usage" 2>&1 || status=$?
test "$status" -eq 2
printf '%s\n' 'ok 3 - missing command is rejected'
printf '%s\n' 'test result: 3 passed; 0 failed'
