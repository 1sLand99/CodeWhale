#!/bin/sh
# Map a workspace area or source path to the fastest cargo test for that
# area. Developer iteration aid only; no product behavior.
#
# Usage:
#   scripts/dev-test.sh <area|path> [filter...]
#   scripts/dev-test.sh --list
#
# Examples:
#   scripts/dev-test.sh config
#   scripts/dev-test.sh tui elapsed::
#   scripts/dev-test.sh crates/tui/src/elapsed.rs
#   scripts/dev-test.sh tui-pty qa_pty
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"

usage() {
  printf '%s\n' "usage: scripts/dev-test.sh <area|path> [filter...]" >&2
  printf '%s\n' "       scripts/dev-test.sh --list" >&2
  exit 2
}

list_areas() {
  cat <<'EOF'
area              command
----              -------
config            cargo test -p codewhale-config --lib --locked
protocol          cargo test -p codewhale-protocol --lib --locked
execpolicy        cargo test -p codewhale-execpolicy --lib --locked
paths             cargo test -p codewhale-paths --lib --locked
secrets           cargo test -p codewhale-secrets --lib --locked
cli               cargo test -p codewhale-cli --lib --locked
core              cargo test -p codewhale-core --lib --locked
tools             cargo test -p codewhale-tools --lib --locked
tui               cargo test -p codewhale-tui --lib --locked
tui-integration   cargo test -p codewhale-tui --test integration --locked
tui-pty           cargo test -p codewhale-tui --test pty --locked
tui-cucumber      cargo test -p codewhale-tui --test cucumber --locked

path prefix                         area / extra filter
-----------                         -------------------
crates/config/                      config
crates/protocol/                    protocol
crates/execpolicy/                  execpolicy
crates/paths/                       paths
crates/secrets/                     secrets
crates/cli/                         cli
crates/core/                        core
crates/tools/                       tools
crates/tui/src/tui/                 tui  tui::
crates/tui/src/tools/               tui  tools::
crates/tui/src/core/                tui  core::
crates/tui/src/commands/            tui  commands::
crates/tui/src/<file>.rs            tui  <file>::
crates/tui/tests/integration/       tui-integration  <stem>
crates/tui/tests/pty/               tui-pty  <stem>
crates/tui/tests/cucumber/          tui-cucumber  <stem>
crates/tui/tests/                  tui-integration

Do not use cargo test --workspace for a single-area edit. --lib and
--tests are disjoint; a green --lib run does not cover crates/tui/tests/.
EOF
}

[ $# -ge 1 ] || usage

if [ "$1" = "--list" ] || [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
  list_areas
  exit 0
fi

area=$1
shift

# Path form: map a source path onto an area, and invent a filter only when
# the caller did not already pass one.
if [ -e "$area" ] || printf '%s' "$area" | grep -q /; then
  rel=${area#./}
  extra=
  case $rel in
    crates/config/*|crates/config) area=config ;;
    crates/protocol/*|crates/protocol) area=protocol ;;
    crates/execpolicy/*|crates/execpolicy) area=execpolicy ;;
    crates/paths/*|crates/paths) area=paths ;;
    crates/secrets/*|crates/secrets) area=secrets ;;
    crates/cli/*|crates/cli) area=cli ;;
    crates/core/*|crates/core) area=core ;;
    crates/tools/*|crates/tools) area=tools ;;
    crates/tui/tests/integration/*)
      area=tui-integration
      extra=$(basename "$rel" .rs)
      ;;
    crates/tui/tests/pty/*)
      area=tui-pty
      extra=$(basename "$rel" .rs)
      ;;
    crates/tui/tests/cucumber/*)
      area=tui-cucumber
      extra=$(basename "$rel" .rs)
      ;;
    crates/tui/tests/*)
      area=tui-integration
      extra=$(basename "$rel" .rs)
      ;;
    crates/tui/src/tui/*)
      area=tui
      extra=tui::
      ;;
    crates/tui/src/tools/*)
      area=tui
      extra=tools::
      ;;
    crates/tui/src/core/*)
      area=tui
      extra=core::
      ;;
    crates/tui/src/commands/*)
      area=tui
      extra=commands::
      ;;
    crates/tui/src/*)
      area=tui
      extra=$(basename "$rel" .rs)::
      ;;
    crates/tui/*|crates/tui)
      area=tui
      ;;
    crates/*)
      crate=$(printf '%s' "$rel" | awk -F/ '{print $2}')
      area=$crate
      ;;
    *)
      printf '%s\n' "dev-test: no area mapping for path: $area" >&2
      exit 2
      ;;
  esac
  case $extra in
    ''|main|main::|lib::|mod|mod::) extra= ;;
  esac
  if [ $# -eq 0 ] && [ -n "${extra:-}" ]; then
    set -- "$extra"
  fi
fi

pkg=
target=--lib
case $area in
  config) pkg=codewhale-config ;;
  protocol) pkg=codewhale-protocol ;;
  execpolicy) pkg=codewhale-execpolicy ;;
  paths) pkg=codewhale-paths ;;
  secrets) pkg=codewhale-secrets ;;
  cli) pkg=codewhale-cli ;;
  core) pkg=codewhale-core ;;
  tools) pkg=codewhale-tools ;;
  tui) pkg=codewhale-tui ;;
  tui-integration)
    pkg=codewhale-tui
    target=--test
    harness=integration
    ;;
  tui-pty)
    pkg=codewhale-tui
    target=--test
    harness=pty
    ;;
  tui-cucumber)
    pkg=codewhale-tui
    target=--test
    harness=cucumber
    ;;
  *)
    printf '%s\n' "dev-test: unknown area: $area (try --list)" >&2
    exit 2
    ;;
esac

if [ "$target" = "--test" ]; then
  set -- test -p "$pkg" --test "$harness" --locked "$@"
else
  set -- test -p "$pkg" --lib --locked "$@"
fi

printf '+ cargo %s\n' "$*"
exec cargo "$@"
