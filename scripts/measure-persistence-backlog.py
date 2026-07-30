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
TEST_NAME = (
    "tui::persistence_actor::backlog_measurement_tests::"
    "write_paused_persistence_backlog_measurement_receipt"
)


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="codewhale-persistence-backlog-") as root:
        receipt_path = Path(root) / "receipt.json"
        env = os.environ.copy()
        env["CARGO_NET_OFFLINE"] = "true"
        env[RECEIPT_ENV] = str(receipt_path)
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
