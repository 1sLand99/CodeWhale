#!/usr/bin/env python3
"""Hermetic tests for check-source-structure-budget.py."""

from __future__ import annotations

import importlib.util
import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "check-source-structure-budget.py"
SPEC = importlib.util.spec_from_file_location("check_source_structure_budget", SCRIPT)
assert SPEC and SPEC.loader
mod = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = mod
SPEC.loader.exec_module(mod)


def snapshot() -> mod.StructureSnapshot:
    return mod.StructureSnapshot(
        ("codewhale-a", "codewhale-b"),
        ("codewhale-a:codew", "codewhale-a:codewhale"),
        {
            "crates/a/src/lib.rs": 1_200,
            "crates/b/src/main.rs": 1_050,
            "crates/b/src/small.rs": 200,
        },
    )


class StructureBudgetTests(unittest.TestCase):
    def test_equal_snapshot_passes(self) -> None:
        current = snapshot()
        budget = mod.validate_budget(mod.budget_document(current))
        self.assertEqual(mod.compare(current, budget), ([], []))

    def test_deletion_and_decomposition_pass_freely(self) -> None:
        budget = mod.validate_budget(mod.budget_document(snapshot()))
        current = mod.StructureSnapshot(
            ("codewhale-a",),
            ("codewhale-a:codew",),
            {
                "crates/a/src/lib.rs": 900,
                "crates/b/src/small.rs": 150,
            },
        )
        failures, improvements = mod.compare(current, budget)
        self.assertEqual(failures, [])
        self.assertGreaterEqual(len(improvements), 4)

    def test_added_package_and_binary_fail(self) -> None:
        baseline = snapshot()
        budget = mod.validate_budget(mod.budget_document(baseline))
        current = mod.StructureSnapshot(
            (*baseline.workspace_packages, "codewhale-c"),
            (*baseline.binary_targets, "codewhale-c:whale-helper"),
            dict(baseline.module_lines),
        )
        failures, _ = mod.compare(current, budget)
        self.assertTrue(any("packages added" in failure for failure in failures))
        self.assertTrue(any("binary targets added" in failure for failure in failures))

    def test_new_large_file_and_aggregate_growth_fail(self) -> None:
        baseline = snapshot()
        budget = mod.validate_budget(mod.budget_document(baseline))
        modules = dict(baseline.module_lines)
        modules["crates/c/src/lib.rs"] = mod.LARGE_MODULE_THRESHOLD
        failures, _ = mod.compare(
            mod.StructureSnapshot(
                baseline.workspace_packages,
                baseline.binary_targets,
                modules,
            ),
            budget,
        )
        self.assertTrue(any("new thousand-line" in failure for failure in failures))
        self.assertTrue(any("aggregate owned" in failure for failure in failures))

    def test_line_neutral_ownership_move_passes_but_larger_maximum_fails(self) -> None:
        baseline = snapshot()
        budget = mod.validate_budget(mod.budget_document(baseline))
        moved = dict(baseline.module_lines)
        moved["crates/a/src/lib.rs"] -= 50
        moved["crates/b/src/main.rs"] += 50
        self.assertEqual(
            mod.compare(
                mod.StructureSnapshot(
                    baseline.workspace_packages, baseline.binary_targets, moved
                ),
                budget,
            )[0],
            [],
        )

        moved["crates/a/src/lib.rs"] += 51
        failures, _ = mod.compare(
            mod.StructureSnapshot(
                baseline.workspace_packages, baseline.binary_targets, moved
            ),
            budget,
        )
        self.assertTrue(any("largest module grew" in failure for failure in failures))

    def test_budget_document_round_trips_with_exact_types(self) -> None:
        document = mod.budget_document(snapshot())
        budget = mod.validate_budget(document)
        self.assertEqual(budget.max_module_lines, 1_200)
        self.assertEqual(budget.max_total_owned_rust_lines, 2_450)
        document["max_module_lines"] = True
        with self.assertRaisesRegex(mod.StructureBudgetError, "non-negative integer"):
            mod.validate_budget(document)

    def test_source_discovery_excludes_owned_test_modules(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "crates" / "a" / "src"
            (source / "tests").mkdir(parents=True)
            (source / "lib.rs").write_text("one\ntwo\n", encoding="utf-8")
            (source / "tests.rs").write_text("test\n", encoding="utf-8")
            (source / "tests" / "fixture.rs").write_text("fixture\n", encoding="utf-8")
            discovered = [path.relative_to(root).as_posix() for path in mod.production_rust_files(root)]
        self.assertEqual(discovered, ["crates/a/src/lib.rs"])

    def test_metadata_command_is_locked_no_deps_and_offline(self) -> None:
        completed = subprocess.CompletedProcess(
            args=[], returncode=0, stdout=json.dumps({"packages": []}), stderr=""
        )
        with mock.patch.object(mod.subprocess, "run", return_value=completed) as run:
            self.assertEqual(mod.cargo_metadata(ROOT), {"packages": []})
        command = run.call_args.args[0]
        environment = run.call_args.kwargs["env"]
        self.assertEqual(command[0:2], ["cargo", "metadata"])
        self.assertIn("--offline", command)
        self.assertIn("--locked", command)
        self.assertIn("--no-deps", command)
        self.assertEqual(environment["CARGO_NET_OFFLINE"], "true")

    def test_atomic_update_preserves_permissions_and_cleans_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "budget.json"
            path.write_text("{}", encoding="utf-8")
            os.chmod(path, 0o640)
            mod.write_json_atomic(path, mod.budget_document(snapshot()))
            self.assertEqual(stat.S_IMODE(path.stat().st_mode), 0o640)
            original = path.read_text(encoding="utf-8")
            with (
                mock.patch.object(mod.os, "replace", side_effect=OSError("stop")),
                self.assertRaisesRegex(OSError, "stop"),
            ):
                mod.write_json_atomic(path, {"replacement": True})
            self.assertEqual(path.read_text(encoding="utf-8"), original)
            self.assertEqual(list(Path(tmp).glob(".budget.json.*.tmp")), [])


if __name__ == "__main__":
    raise SystemExit(unittest.main())
