#!/usr/bin/env python3
"""Check the paused persistence backlog against one-way local ceilings."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
MEASURE_SCRIPT = ROOT / "scripts" / "measure-persistence-backlog.py"
BUDGET_PATH = ROOT / "scripts" / "persistence-backlog-budget.json"
RECEIPT_KIND = "codewhale.persistence_backlog_receipt"
BUDGET_KIND = "codewhale.persistence_backlog_budget"
SCHEMA_VERSION = 1

FIXTURE = {
    "fixture_id": "paused-production-channel-session-snapshot-v1",
    "request_variant": "session_snapshot",
    "payload_estimator": "retained-saved-session-json-bytes-v1",
    "paused_consumer": True,
    "requests_attempted": 128,
    "content_bytes_per_request": 64 * 1024,
    "single_session_id": True,
    "expected_newest_version": 127,
}

REQUIRED_RECEIPT_FIELDS = (
    "document_kind",
    "schema_version",
    "fixture_id",
    "platform",
    "request_variant",
    "payload_estimator",
    "paused_consumer",
    "requests_attempted",
    "content_bytes_per_request",
    "single_session_id",
    "expected_newest_version",
    "accepted_requests",
    "retained_queued_requests",
    "estimated_retained_payload_bytes",
    "newest_retained_version",
    "newest_version_retained",
    "enqueue_elapsed_ns",
    "rss_supported",
    "rss_before_bytes",
    "rss_during_bytes",
    "rss_after_bytes",
    "rss_during_delta_bytes",
    "rss_after_delta_bytes",
    "limitations",
)

CEILING_FIELDS = (
    "retained_queued_requests",
    "estimated_retained_payload_bytes",
    "enqueue_elapsed_ns",
    "rss_during_delta_bytes",
    "rss_after_delta_bytes",
)
RSS_SAMPLE_FIELDS = ("rss_before_bytes", "rss_during_bytes", "rss_after_bytes")
RSS_DELTA_FIELDS = ("rss_during_delta_bytes", "rss_after_delta_bytes")


class PersistenceBacklogError(ValueError):
    """A receipt or budget broke the measurement contract."""


def load_json(path: Path, label: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise PersistenceBacklogError(f"invalid {label} {path}: {error}") from error
    if not isinstance(value, dict):
        raise PersistenceBacklogError(f"{label} must be a JSON object")
    return value


def non_negative_integer(value: Any, field: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise PersistenceBacklogError(f"{field} must be a non-negative integer")
    return value


def validate_receipt(receipt: dict[str, Any]) -> None:
    missing = [field for field in REQUIRED_RECEIPT_FIELDS if field not in receipt]
    if missing:
        raise PersistenceBacklogError(
            "receipt missing required field(s): " + ", ".join(missing)
        )
    if receipt["document_kind"] != RECEIPT_KIND:
        raise PersistenceBacklogError(f"receipt document_kind must be {RECEIPT_KIND}")
    if receipt["schema_version"] != SCHEMA_VERSION:
        raise PersistenceBacklogError("receipt schema_version changed")
    for field, expected in FIXTURE.items():
        if receipt[field] != expected:
            raise PersistenceBacklogError(
                f"receipt {field} must remain {expected!r}, got {receipt[field]!r}"
            )
    if not isinstance(receipt["platform"], str) or not receipt["platform"]:
        raise PersistenceBacklogError("receipt platform must be a non-empty string")

    attempted = non_negative_integer(receipt["requests_attempted"], "requests_attempted")
    accepted = non_negative_integer(receipt["accepted_requests"], "accepted_requests")
    if accepted != attempted:
        raise PersistenceBacklogError(
            "accepted_requests must equal requests_attempted; sender rejection is not backlog improvement"
        )
    retained = non_negative_integer(
        receipt["retained_queued_requests"], "retained_queued_requests"
    )
    if retained > accepted:
        raise PersistenceBacklogError("retained_queued_requests exceeds accepted_requests")
    for field in ("estimated_retained_payload_bytes", "enqueue_elapsed_ns"):
        non_negative_integer(receipt[field], field)
    if retained == 0 or receipt["estimated_retained_payload_bytes"] == 0:
        raise PersistenceBacklogError(
            "the paused channel must retain the newest request and its payload"
        )
    newest = non_negative_integer(
        receipt["newest_retained_version"], "newest_retained_version"
    )
    if newest != FIXTURE["expected_newest_version"]:
        raise PersistenceBacklogError("newest_retained_version is not the final sent version")
    if receipt["newest_version_retained"] is not True:
        raise PersistenceBacklogError("newest_version_retained must be true")

    limitations = receipt["limitations"]
    if not isinstance(limitations, list) or not limitations or not all(
        isinstance(item, str) and item for item in limitations
    ):
        raise PersistenceBacklogError("limitations must be a non-empty string array")

    if not isinstance(receipt["rss_supported"], bool):
        raise PersistenceBacklogError("rss_supported must be boolean")
    rss_fields = RSS_SAMPLE_FIELDS + RSS_DELTA_FIELDS
    if receipt["rss_supported"]:
        for field in rss_fields:
            non_negative_integer(receipt[field], field)
        before = receipt["rss_before_bytes"]
        if receipt["rss_during_delta_bytes"] != max(
            0, receipt["rss_during_bytes"] - before
        ):
            raise PersistenceBacklogError("rss_during_delta_bytes is inconsistent")
        if receipt["rss_after_delta_bytes"] != max(
            0, receipt["rss_after_bytes"] - before
        ):
            raise PersistenceBacklogError("rss_after_delta_bytes is inconsistent")
    elif any(receipt[field] is not None for field in rss_fields):
        raise PersistenceBacklogError("unsupported RSS fields must be null")


def validate_budget(budget: dict[str, Any]) -> None:
    if budget.get("document_kind") != BUDGET_KIND:
        raise PersistenceBacklogError(f"budget document_kind must be {BUDGET_KIND}")
    if budget.get("schema_version") != SCHEMA_VERSION:
        raise PersistenceBacklogError("budget schema_version changed")
    fixture = budget.get("fixture")
    if fixture != FIXTURE:
        raise PersistenceBacklogError("budget fixture no longer matches the frozen workload")
    ceilings = budget.get("ceilings")
    baseline = budget.get("baseline_observation")
    if not isinstance(ceilings, dict) or not isinstance(baseline, dict):
        raise PersistenceBacklogError("budget needs ceilings and baseline_observation objects")
    for field in CEILING_FIELDS:
        ceiling = non_negative_integer(ceilings.get(field), f"ceilings.{field}")
        observed = non_negative_integer(
            baseline.get(field), f"baseline_observation.{field}"
        )
        if observed > ceiling:
            raise PersistenceBacklogError(
                f"baseline_observation.{field} exceeds its ceiling"
            )
    if baseline.get("accepted_requests") != FIXTURE["requests_attempted"]:
        raise PersistenceBacklogError(
            "baseline_observation.accepted_requests must equal requests_attempted"
        )


def compare(
    receipt: dict[str, Any], budget: dict[str, Any]
) -> tuple[list[tuple[str, int, int]], list[tuple[str, int, int]]]:
    validate_receipt(receipt)
    validate_budget(budget)
    increases: list[tuple[str, int, int]] = []
    decreases: list[tuple[str, int, int]] = []
    for field in CEILING_FIELDS:
        if field in RSS_DELTA_FIELDS and not receipt["rss_supported"]:
            continue
        current = receipt[field]
        ceiling = budget["ceilings"][field]
        if current > ceiling:
            increases.append((field, current, ceiling))
        elif current < ceiling:
            decreases.append((field, current, ceiling))
    return increases, decreases


def measure() -> dict[str, Any]:
    env = os.environ.copy()
    env["CARGO_NET_OFFLINE"] = "true"
    result = subprocess.run(
        [sys.executable, str(MEASURE_SCRIPT)],
        cwd=ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    sys.stderr.write(result.stderr)
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        raise PersistenceBacklogError("measurement command failed")
    try:
        receipt = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise PersistenceBacklogError(f"measurement emitted invalid JSON: {error}") from error
    if not isinstance(receipt, dict):
        raise PersistenceBacklogError("measurement receipt must be an object")
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, help="check an existing receipt")
    parser.add_argument("--budget", type=Path, default=BUDGET_PATH)
    args = parser.parse_args()
    try:
        receipt = load_json(args.receipt, "receipt") if args.receipt else measure()
        budget = load_json(args.budget, "budget")
        increases, decreases = compare(receipt, budget)
    except PersistenceBacklogError as error:
        print(f"[persistence-backlog-budget] ERROR: {error}", file=sys.stderr)
        return 2
    if increases:
        for field, current, ceiling in increases:
            print(
                f"[persistence-backlog-budget] FAIL: {field}={current} exceeds {ceiling}",
                file=sys.stderr,
            )
        return 1
    print("[persistence-backlog-budget] PASS: one-way ceilings respected")
    for field, current, ceiling in decreases:
        print(f"  can tighten {field}: {current} < {ceiling}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
