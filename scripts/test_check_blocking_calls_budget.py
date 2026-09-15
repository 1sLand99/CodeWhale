#!/usr/bin/env python3
"""Hermetic tests for the blocking-calls budget gate (#6149)."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts" / "check-blocking-calls-budget.py"
SPEC = importlib.util.spec_from_file_location("blocking_calls", SCRIPT)
assert SPEC and SPEC.loader
mod = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = mod
SPEC.loader.exec_module(mod)


def counts(source: str) -> dict[str, int]:
    with tempfile.TemporaryDirectory() as tmp:
        victim = Path(tmp) / "victim.rs"
        victim.write_text(source, encoding="utf-8")
        return mod.file_counts(victim)


class BlockingCallScopeTests(unittest.TestCase):
    def test_sleep_in_plain_fn_counts(self) -> None:
        self.assertEqual(
            counts("fn wait() {\n    std::thread::sleep(std::time::Duration::from_millis(1));\n}\n"),
            {"thread_sleep": 1},
        )

    def test_sleep_in_async_fn_counts(self) -> None:
        self.assertEqual(
            counts("async fn run() {\n    std::thread::sleep(std::time::Duration::from_millis(1));\n}\n"),
            {"thread_sleep": 1},
        )

    def test_sleep_in_spawn_blocking_is_exempt(self) -> None:
        self.assertEqual(
            counts(
                "async fn run() {\n"
                "    tokio::task::spawn_blocking(move || {\n"
                "        std::thread::sleep(std::time::Duration::from_millis(1));\n"
                "    });\n"
                "}\n"
            ),
            {},
        )

    def test_sleep_in_dedicated_thread_is_exempt(self) -> None:
        self.assertEqual(
            counts(
                "fn pump() {\n"
                "    std::thread::Builder::new().spawn(move || {\n"
                "        std::thread::sleep(std::time::Duration::from_millis(5));\n"
                "    });\n"
                "}\n"
            ),
            {},
        )

    def test_sleep_in_tests_mod_is_exempt(self) -> None:
        self.assertEqual(
            counts(
                "fn prod() {}\n"
                "#[cfg(test)]\n"
                "mod tests {\n"
                "    fn probe() { std::thread::sleep(std::time::Duration::from_millis(1)); }\n"
                "}\n"
            ),
            {},
        )

    def test_sleep_in_cfg_test_fn_is_exempt(self) -> None:
        self.assertEqual(
            counts(
                "#[cfg(test)]\n"
                "fn helper() { std::thread::sleep(std::time::Duration::from_millis(1)); }\n"
            ),
            {},
        )

    def test_std_fs_call_counts(self) -> None:
        self.assertEqual(
            counts("async fn go() {\n    let _ = std::fs::read_to_string(p).unwrap();\n}\n"),
            {"std_fs": 1},
        )

    def test_comment_and_string_literals_do_not_count(self) -> None:
        self.assertEqual(
            counts(
                "fn doc() {\n"
                "    // std::thread::sleep(std::time::Duration::from_millis(1));\n"
                '    let s = "std::fs::read_to_string(p)";\n'
                "    let t = r#\"std::fs::write(a, b)\"#;\n"
                "}\n"
            ),
            {},
        )

    def test_tokio_equivalents_do_not_count(self) -> None:
        self.assertEqual(
            counts(
                "async fn go() {\n"
                "    tokio::time::sleep(std::time::Duration::from_millis(1)).await;\n"
                "    let _ = tokio::fs::read_to_string(p).await;\n"
                "}\n"
            ),
            {},
        )


if __name__ == "__main__":
    unittest.main()
