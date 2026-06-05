#!/usr/bin/env bash
# run-pinchbench.sh — Run PinchBench benchmarks with CodeWhale model routing.
#
# PinchBench evaluates agent performance on real-world tasks (calendar, email,
# coding, research, file management). It uses OpenClaw as the agent runtime and
# routes models through OpenRouter by default.
#
# Known issues with Xiaomi MiMo v2.5:
#   1. PinchBench validates models against OpenRouter's /models endpoint.
#      MiMo models MUST use the openrouter/ prefix or validation is skipped.
#   2. PinchBench requires OPENROUTER_API_KEY even when using a direct provider.
#      The --direct-mimo flag sets up a custom OpenAI-compatible endpoint in
#      OpenClaw's models.json to bypass this.
#   3. MiMo v2.5 Pro has a 128K context window but PinchBench tasks are small.
#      No special handling needed, but worth noting for cost estimates.
#   4. The Xiaomi Token Plan endpoint (token-plan-sgp.xiaomimimo.com) uses
#      tp- prefixed keys. Pay-as-you-go (api.xiaomimimo.com) uses sk- keys.
#      Make sure XIAOMI_MIMO_API_KEY matches the endpoint you're using.
#   5. OpenRouter model ID for MiMo: xiaomi/mimo-v2.5-pro (Pro) or
#      xiaomi/mimo-v2.5 (Omni). PinchBench expects the full provider/model.
#
# Usage:
#   ./scripts/benchmarks/run-pinchbench.sh --help
#   ./scripts/benchmarks/run-pinchbench.sh --model xiaomi/mimo-v2.5-pro
#   ./scripts/benchmarks/run-pinchbench.sh --direct-mimo --suite task_calendar
#
# Prerequisites:
#   - PinchBench cloned (or use --install)
#   - Python 3.10+ with uv
#   - OPENROUTER_API_KEY (for OpenRouter routing)
#   - OR XIAOMI_MIMO_API_KEY + --direct-mimo (for direct Xiaomi API)
#   - A running OpenClaw instance

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Defaults — MiMo v2.5 Pro via OpenRouter
MODEL="openrouter/xiaomi/mimo-v2.5-pro"
SUITE="all"
PINCHBENCH_DIR="${PINCHBENCH_DIR:-/tmp/pinchbench}"
RESULTS_DIR="./results/pinchbench"
INSTALL_PINCHBENCH=false
RUNS=1
JUDGE_MODEL=""
NO_UPLOAD=true
DIRECT_MIMO=false
MIMO_BASE_URL=""
EXTRA_ARGS=()

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Run PinchBench benchmarks. Defaults to Xiaomi MiMo v2.5 Pro via OpenRouter.

Options:
  --model MODEL           Model ID (default: openrouter/xiaomi/mimo-v2.5-pro)
                          Common values:
                            openrouter/xiaomi/mimo-v2.5-pro  — MiMo Pro via OpenRouter
                            openrouter/xiaomi/mimo-v2.5      — MiMo Omni via OpenRouter
                            openrouter/deepseek/deepseek-v4-pro — DeepSeek V4 Pro via OpenRouter
  --suite SUITE           Task suite: all, automated-only, or comma-separated IDs
  --runs N                Runs per task for averaging (default: 1)
  --judge MODEL           Judge model for LLM grading (default: uses OpenClaw agent)
  --direct-mimo           Route MiMo directly via Xiaomi API (bypasses OpenRouter)
                          Requires XIAOMI_MIMO_API_KEY. Sets model to mimo-v2.5-pro.
  --mimo-base-url URL     Override MiMo API base URL (default: Token Plan Singapore)
  --pinchbench-dir DIR    PinchBench install directory (default: /tmp/pinchbench)
  --results-dir DIR       Local results directory (default: ./results/pinchbench)
  --install               Install/clone PinchBench before running
  --upload                Upload results to pinchbench.com leaderboard
  -- [EXTRA_ARGS...]      Additional arguments passed to PinchBench
  -h, --help              Show this help

Environment variables:
  OPENROUTER_API_KEY      Required for OpenRouter model routing
  XIAOMI_MIMO_API_KEY     Required for --direct-mimo (or XIAOMI_API_KEY / MIMO_API_KEY)
  XIAOMI_MIMO_BASE_URL    Override MiMo API endpoint

Examples:
  # MiMo v2.5 Pro via OpenRouter (default)
  $(basename "$0")

  # MiMo v2.5 Pro via direct Xiaomi API
  $(basename "$0") --direct-mimo

  # Specific tasks with MiMo
  $(basename "$0") --suite task_calendar,task_stock

  # Install PinchBench and run
  $(basename "$0") --install

  # DeepSeek V4 Pro via OpenRouter
  $(basename "$0") --model openrouter/deepseek/deepseek-v4-pro
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --model) MODEL="$2"; shift 2 ;;
        --suite) SUITE="$2"; shift 2 ;;
        --runs) RUNS="$2"; shift 2 ;;
        --judge) JUDGE_MODEL="$2"; shift 2 ;;
        --direct-mimo) DIRECT_MIMO=true; shift ;;
        --mimo-base-url) MIMO_BASE_URL="$2"; shift 2 ;;
        --pinchbench-dir) PINCHBENCH_DIR="$2"; shift 2 ;;
        --results-dir) RESULTS_DIR="$2"; shift 2 ;;
        --install) INSTALL_PINCHBENCH=true; shift ;;
        --upload) NO_UPLOAD=false; shift ;;
        --) shift; EXTRA_ARGS=("$@"); break ;;
        -h|--help) usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
    esac
done

# ── Direct MiMo mode ────────────────────────────────────────────────────────
# When --direct-mimo is set, we configure PinchBench to use Xiaomi's API
# directly instead of routing through OpenRouter. This creates a custom
# OpenAI-compatible provider entry in OpenClaw's models.json.
if [[ "$DIRECT_MIMO" == true ]]; then
    MODEL="mimo-v2.5-pro"

    # Resolve API key from multiple env var names
    MIMO_KEY="${XIAOMI_MIMO_API_KEY:-${XIAOMI_API_KEY:-${MIMO_API_KEY:-}}}"
    if [[ -z "$MIMO_KEY" ]]; then
        echo "Error: --direct-mimo requires XIAOMI_MIMO_API_KEY (or XIAOMI_API_KEY / MIMO_API_KEY)" >&2
        echo "  Token Plan keys (tp-...): https://token-plan-sgp.xiaomimimo.com/v1" >&2
        echo "  Pay-as-you-go keys (sk-...): https://api.xiaomimimo.com/v1" >&2
        exit 1
    fi

    # Determine base URL: flag > env > default (Token Plan Singapore)
    if [[ -z "$MIMO_BASE_URL" ]]; then
        MIMO_BASE_URL="${XIAOMI_MIMO_BASE_URL:-https://token-plan-sgp.xiaomimimo.com/v1}"
    fi

    # Detect key type and warn if mismatched
    if [[ "$MIMO_KEY" == tp-* && "$MIMO_BASE_URL" == *"api.xiaomimimo.com"* ]]; then
        echo "Warning: tp- key used with pay-as-you-go endpoint. Token Plan keys work with:" >&2
        echo "  https://token-plan-sgp.xiaomimimo.com/v1" >&2
    elif [[ "$MIMO_KEY" == sk-* && "$MIMO_BASE_URL" == *"token-plan"* ]]; then
        echo "Warning: sk- key used with Token Plan endpoint. Pay-as-you-go keys work with:" >&2
        echo "  https://api.xiaomimimo.com/v1" >&2
    fi

    echo "Direct MiMo mode:"
    echo "  Model:    $MODEL"
    echo "  Endpoint: $MIMO_BASE_URL"
    echo "  Key type: ${MIMO_KEY:0:3}..."
    echo ""

    # Export for PinchBench's lib_agent.py custom provider setup
    export OPENAI_API_KEY="$MIMO_KEY"
    export OPENAI_BASE_URL="$MIMO_BASE_URL"
fi

# ── Prereq checks ───────────────────────────────────────────────────────────
if [[ "$DIRECT_MIMO" != true ]]; then
    # OpenRouter mode — need the key
    if [[ -z "${OPENROUTER_API_KEY:-}" ]]; then
        echo "Warning: OPENROUTER_API_KEY not set. PinchBench may fail model validation." >&2
        echo "  Either set OPENROUTER_API_KEY or use --direct-mimo with XIAOMI_MIMO_API_KEY." >&2
    fi
fi

# ── Install PinchBench ──────────────────────────────────────────────────────
if [[ "$INSTALL_PINCHBENCH" == true || ! -d "$PINCHBENCH_DIR" ]]; then
    echo "Installing PinchBench to $PINCHBENCH_DIR ..."
    if [[ -d "$PINCHBENCH_DIR" ]]; then
        cd "$PINCHBENCH_DIR" && git pull
    else
        git clone https://github.com/pinchbench/skill.git "$PINCHBENCH_DIR"
    fi
    cd "$PINCHBENCH_DIR"
    uv venv .venv 2>/dev/null || true
    source .venv/bin/activate
    uv pip install -e .
fi

if [[ ! -d "$PINCHBENCH_DIR" ]]; then
    echo "Error: PinchBench not found at $PINCHBENCH_DIR" >&2
    echo "Run with --install to clone it automatically." >&2
    exit 1
fi

cd "$PINCHBENCH_DIR"

if [[ -f ".venv/bin/activate" ]]; then
    source .venv/bin/activate
fi

mkdir -p "$RESULTS_DIR"

# ── Record metadata ─────────────────────────────────────────────────────────
METADATA_FILE="$RESULTS_DIR/run_metadata.json"
cat > "$METADATA_FILE" <<META
{
    "codewhale_version": "$(codewhale --version 2>/dev/null || echo unknown)",
    "git_commit": "$(cd "$REPO_ROOT" && git rev-parse HEAD 2>/dev/null || echo unknown)",
    "pinchbench_commit": "$(git -C "$PINCHBENCH_DIR" rev-parse HEAD 2>/dev/null || echo unknown)",
    "model": "$MODEL",
    "routing": "$(if [[ "$DIRECT_MIMO" == true ]]; then echo "direct-xiaomi"; else echo "openrouter"; fi)",
    "suite": "$SUITE",
    "runs": $RUNS,
    "timestamp_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "platform": "$(uname -s)/$(uname -m)"
}
META
echo "Run metadata: $METADATA_FILE"

# ── Build and run PinchBench ────────────────────────────────────────────────
PB_ARGS=("--model" "$MODEL" "--suite" "$SUITE" "--runs" "$RUNS" "--output-dir" "$RESULTS_DIR")

if [[ -n "$JUDGE_MODEL" ]]; then
    PB_ARGS+=("--judge" "$JUDGE_MODEL")
fi

if [[ "$NO_UPLOAD" == true ]]; then
    PB_ARGS+=("--no-upload")
fi

# Pass direct-mimo endpoint info via env for lib_agent.py's custom provider setup
if [[ "$DIRECT_MIMO" == true ]]; then
    PB_ARGS+=("--base-url" "$MIMO_BASE_URL")
fi

PB_ARGS+=("${EXTRA_ARGS[@]}")

echo "Running PinchBench..."
echo "  Model:    $MODEL"
echo "  Suite:    $SUITE"
echo "  Runs:     $RUNS"
echo "  Output:   $RESULTS_DIR"
if [[ "$DIRECT_MIMO" == true ]]; then
    echo "  Routing:  Direct Xiaomi API ($MIMO_BASE_URL)"
else
    echo "  Routing:  OpenRouter"
fi
echo ""

./scripts/run.sh "${PB_ARGS[@]}"

echo ""
echo "Results written to $RESULTS_DIR"
