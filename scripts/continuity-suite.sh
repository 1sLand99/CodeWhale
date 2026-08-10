#!/bin/bash
# v0.9.6 runtime continuity suite — the provider-neutral release gate that
# keeps the v0.9.6 failure family fixed (codewhale-ops v0.9.7 ledger, P0).
#
# The invariants under guard, and where they live:
#   - Compaction pressure comes from the latest parent request, never
#     cumulative billing or child usage      -> tui --lib core turn/turn_loop
#   - Repeated compaction replaces the prior summary and keeps the stable
#     request prefix byte-stable             -> tui --lib compaction,
#                                               --test integration cache_guard
#   - Manual /compact queues behind an active turn (including a saturated
#     op mailbox), auto compaction runs only at a safe boundary, outcomes
#     land as durable transcript receipts    -> --test pty release_runtime_qa
#   - Truthful reasoning display and live shell-wait progress
#                                            -> --test pty release_runtime_qa
#
# Run from the repository root. Any failure is a release blocker for the
# runtime; do not rename a failure a flake without an isolated rerun.
set -u

overall=0
run_gate() {
  local name="$1"; shift
  echo "=== CONTINUITY GATE: ${name} ==="
  if "$@"; then
    echo "PASS: ${name}"
  else
    overall=1
    echo "FAIL: ${name}"
  fi
}

run_gate "turn-and-compaction-units" \
  cargo test -p codewhale-tui --lib -- compaction turn_loop turn::
run_gate "cache-prefix-guard" \
  cargo test -p codewhale-tui --test integration cache_guard
run_gate "release-runtime-qa-pty" \
  cargo test -p codewhale-tui --test pty release_runtime_qa

echo "=== CONTINUITY SUITE: $([ $overall -eq 0 ] && echo PASS || echo FAIL) ==="
exit $overall
