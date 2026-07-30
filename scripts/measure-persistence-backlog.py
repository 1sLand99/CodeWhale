#!/usr/bin/env python3
"""Measure the paused-consumer persistence-channel backlog hermetically."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path


RECEIPT_ENV = "CODEWHALE_TEST_PERSISTENCE_BACKLOG_RECEIPT_PATH"
SOURCE_SHA_ENV = "CODEWHALE_TEST_PERSISTENCE_BACKLOG_SOURCE_SHA"
SOURCE_DIRTY_ENV = "CODEWHALE_TEST_PERSISTENCE_BACKLOG_SOURCE_DIRTY"
RUSTC_VERSION_ENV = "CODEWHALE_TEST_PERSISTENCE_BACKLOG_RUSTC_VERSION"
CARGO_VERSION_ENV = "CODEWHALE_TEST_PERSISTENCE_BACKLOG_CARGO_VERSION"
ROOT = Path(__file__).resolve().parent.parent
TEST_NAME = (
    "tui::persistence_actor::backlog_measurement_tests::"
    "write_paused_persistence_backlog_measurement_receipt"
)


def main() -> int:
    source_sha = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    ).stdout.strip()
    source_dirty = bool(
        subprocess.run(
            ["git", "status", "--porcelain", "--untracked-files=normal"],
            cwd=ROOT,
            text=True,
            capture_output=True,
            check=True,
        ).stdout
    )
    rustc_version = subprocess.run(
        ["rustc", "--version"], text=True, capture_output=True, check=True
    ).stdout.strip()
    cargo_version = subprocess.run(
        ["cargo", "--version"], text=True, capture_output=True, check=True
    ).stdout.strip()
    with tempfile.TemporaryDirectory(prefix="codewhale-persistence-backlog-") as root:
        receipt_path = Path(root) / "receipt.json"
        env = os.environ.copy()
        env["CARGO_NET_OFFLINE"] = "true"
        env[RECEIPT_ENV] = str(receipt_path)
        env[SOURCE_SHA_ENV] = source_sha
        env[SOURCE_DIRTY_ENV] = str(source_dirty).lower()
        env[RUSTC_VERSION_ENV] = rustc_version
        env[CARGO_VERSION_ENV] = cargo_version
        command = [
            "cargo",
            "test",
            "--locked",
            "-p",
            "codewhale-tui",
            "--bin",
            "codewhale-tui",
            TEST_NAME,
            "--",
            "--exact",
            "--ignored",
            "--test-threads=1",
        ]
        result = subprocess.run(
            command,
            env=env,
            text=True,
            capture_output=True,
            check=False,
        )
        sys.stderr.write(result.stderr)
        if result.returncode != 0:
            sys.stdout.write(result.stdout)
            return result.returncode
        try:
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            sys.stderr.write(f"invalid persistence backlog receipt: {error}\n")
            return 1

    print(json.dumps(receipt, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
