#!/usr/bin/env python3
"""Hermetic contract tests for the persistence backlog ratchet."""

from __future__ import annotations

import copy
import importlib.util
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "check-persistence-backlog-budget.py"
SPEC = importlib.util.spec_from_file_location("check_persistence_backlog_budget", SCRIPT)
assert SPEC and SPEC.loader
mod = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = mod
SPEC.loader.exec_module(mod)


def receipt_fixture(*, rss_supported: bool = True) -> dict:
    receipt = {
        "document_kind": mod.RECEIPT_KIND,
        "schema_version": mod.SCHEMA_VERSION,
        **mod.FIXTURE,
        "platform": "macos" if rss_supported else "linux",
        "accepted_requests": 128,
        "retained_queued_requests": 128,
        "estimated_retained_payload_bytes": 8_500_000,
        "newest_retained_version": 127,
        "newest_version_retained": True,
        "enqueue_elapsed_ns": 500_000,
        "rss_supported": rss_supported,
        "rss_before_bytes": 100_000_000 if rss_supported else None,
        "rss_during_bytes": 112_000_000 if rss_supported else None,
        "rss_after_bytes": 103_000_000 if rss_supported else None,
        "rss_during_delta_bytes": 12_000_000 if rss_supported else None,
        "rss_after_delta_bytes": 3_000_000 if rss_supported else None,
        "limitations": ["macOS RSS only"],
    }
    return receipt


def budget_fixture(receipt: dict | None = None) -> dict:
    receipt = receipt or receipt_fixture()
    metrics = {
        field: receipt[field] if receipt[field] is not None else 0
        for field in mod.CEILING_FIELDS
    }
    return {
        "document_kind": mod.BUDGET_KIND,
        "schema_version": mod.SCHEMA_VERSION,
        "fixture": copy.deepcopy(mod.FIXTURE),
        "baseline_observation": {
            "accepted_requests": receipt["accepted_requests"],
            **copy.deepcopy(metrics),
        },
        "ceilings": copy.deepcopy(metrics),
    }


class PersistenceBacklogBudgetTests(unittest.TestCase):
    def test_equal_baseline_passes(self) -> None:
        self.assertEqual(
            mod.compare(receipt_fixture(), budget_fixture()),
            ([], []),
        )

    def test_every_receipt_field_is_required(self) -> None:
        budget = budget_fixture()
        for field in mod.REQUIRED_RECEIPT_FIELDS:
            with self.subTest(field=field):
                receipt = receipt_fixture()
                del receipt[field]
                with self.assertRaisesRegex(
                    mod.PersistenceBacklogError, "missing required field"
                ):
                    mod.compare(receipt, budget)

    def test_frozen_workload_cannot_be_weakened_to_fake_an_improvement(self) -> None:
        budget = budget_fixture()
        for field, replacement in [
            ("paused_consumer", False),
            ("requests_attempted", 64),
            ("content_bytes_per_request", 32 * 1024),
            ("single_session_id", False),
            ("expected_newest_version", 63),
            ("request_variant", "clear_checkpoint"),
            ("payload_estimator", "shallow-size"),
        ]:
            with self.subTest(field=field):
                receipt = receipt_fixture()
                receipt[field] = replacement
                with self.assertRaisesRegex(mod.PersistenceBacklogError, field):
                    mod.compare(receipt, budget)

    def test_every_ceiling_rejects_growth_and_accepts_tightening(self) -> None:
        baseline = receipt_fixture()
        baseline["retained_queued_requests"] = 64
        baseline["estimated_retained_payload_bytes"] = 4_250_000
        budget = budget_fixture(baseline)
        for field in mod.CEILING_FIELDS:
            with self.subTest(field=field):
                grown = copy.deepcopy(baseline)
                grown[field] += 1
                if field == "rss_during_delta_bytes":
                    grown["rss_during_bytes"] += 1
                elif field == "rss_after_delta_bytes":
                    grown["rss_after_bytes"] += 1
                increases, _ = mod.compare(grown, budget)
                self.assertEqual([item[0] for item in increases], [field])

                reduced = copy.deepcopy(baseline)
                reduced[field] -= 1
                if field == "rss_during_delta_bytes":
                    reduced["rss_during_bytes"] -= 1
                elif field == "rss_after_delta_bytes":
                    reduced["rss_after_bytes"] -= 1
                increases, decreases = mod.compare(reduced, budget)
                self.assertEqual(increases, [])
                self.assertIn(field, [item[0] for item in decreases])

    def test_non_macos_receipt_keeps_rss_shape_but_skips_rss_ceilings(self) -> None:
        receipt = receipt_fixture(rss_supported=False)
        budget = budget_fixture()
        increases, decreases = mod.compare(receipt, budget)
        self.assertEqual(increases, [])
        self.assertNotIn(
            "rss_during_delta_bytes", [item[0] for item in decreases]
        )
        self.assertNotIn("rss_after_delta_bytes", [item[0] for item in decreases])

    def test_rss_delta_must_match_samples(self) -> None:
        receipt = receipt_fixture()
        receipt["rss_during_delta_bytes"] += 1
        with self.assertRaisesRegex(mod.PersistenceBacklogError, "inconsistent"):
            mod.compare(receipt, budget_fixture())

    def test_sender_rejection_cannot_masquerade_as_backlog_improvement(self) -> None:
        receipt = receipt_fixture()
        receipt["accepted_requests"] = 1
        receipt["retained_queued_requests"] = 1
        receipt["estimated_retained_payload_bytes"] = 66_000
        with self.assertRaisesRegex(
            mod.PersistenceBacklogError, "sender rejection is not backlog improvement"
        ):
            mod.compare(receipt, budget_fixture())

    def test_missing_newest_version_cannot_pass_as_coalescing(self) -> None:
        receipt = receipt_fixture()
        receipt["newest_retained_version"] = 126
        receipt["newest_version_retained"] = False
        with self.assertRaisesRegex(mod.PersistenceBacklogError, "final sent version"):
            mod.compare(receipt, budget_fixture())

    def test_budget_cannot_hide_a_baseline_above_its_ceiling(self) -> None:
        budget = budget_fixture()
        budget["baseline_observation"]["retained_queued_requests"] += 1
        with self.assertRaisesRegex(mod.PersistenceBacklogError, "exceeds its ceiling"):
            mod.compare(receipt_fixture(), budget)


if __name__ == "__main__":
    unittest.main()
