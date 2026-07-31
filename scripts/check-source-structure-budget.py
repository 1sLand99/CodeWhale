#!/usr/bin/env python3
"""Enforce one-way ceilings on Codewhale's source ownership structure.

The budget permits deletion and line-neutral ownership moves freely. Adding a
workspace package or binary, creating a new thousand-line production Rust
module, increasing the largest owned file, or growing aggregate owned Rust
source requires an explicit budget update and review.
"""

from __future__ import annotations

import argparse
import json
import os
import stat
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence

REPO_ROOT = Path(__file__).resolve().parent.parent
BUDGET_PATH = REPO_ROOT / "scripts" / "source-structure-budget.json"
DOCUMENT_KIND = "codewhale.source_structure_budget"
SCHEMA_VERSION = 1
LARGE_MODULE_THRESHOLD = 1_000


class StructureBudgetError(ValueError):
    """The budget or measured source structure is invalid."""


@dataclass(frozen=True)
class StructureSnapshot:
    workspace_packages: tuple[str, ...]
    binary_targets: tuple[str, ...]
    module_lines: dict[str, int]


@dataclass(frozen=True)
class StructureBudget:
    workspace_packages: tuple[str, ...]
    binary_targets: tuple[str, ...]
    allowed_large_modules: tuple[str, ...]
    max_large_module_count: int
    max_module_lines: int
    max_total_owned_rust_lines: int


def production_rust_files(root: Path) -> list[Path]:
    files = []
    for path in (root / "crates").glob("*/src/**/*.rs"):
        relative = path.relative_to(root)
        if path.name == "tests.rs" or "tests" in relative.parts:
            continue
        files.append(path)
    return sorted(files)


def source_line_count(path: Path) -> int:
    return len(path.read_bytes().splitlines())


def cargo_metadata(root: Path) -> dict[str, Any]:
    environment = os.environ.copy()
    environment["CARGO_NET_OFFLINE"] = "true"
    process = subprocess.run(
        [
            "cargo",
            "metadata",
            "--offline",
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        ],
        cwd=root,
        env=environment,
        capture_output=True,
        text=True,
        check=False,
    )
    if process.returncode != 0:
        sys.stderr.write(process.stderr)
        raise StructureBudgetError(
            f"cargo metadata failed with exit code {process.returncode}"
        )
    try:
        document = json.loads(process.stdout)
    except json.JSONDecodeError as error:
        raise StructureBudgetError(f"cargo metadata emitted invalid JSON: {error}") from error
    if not isinstance(document, dict) or not isinstance(document.get("packages"), list):
        raise StructureBudgetError("cargo metadata did not contain a package list")
    return document


def measure(root: Path = REPO_ROOT) -> StructureSnapshot:
    metadata = cargo_metadata(root)
    packages = tuple(sorted(package["name"] for package in metadata["packages"]))
    binaries = tuple(
        sorted(
            f"{package['name']}:{target['name']}"
            for package in metadata["packages"]
            for target in package.get("targets", [])
            if "bin" in target.get("kind", [])
        )
    )
    module_lines = {
        path.relative_to(root).as_posix(): source_line_count(path)
        for path in production_rust_files(root)
    }
    return StructureSnapshot(packages, binaries, module_lines)


def require_string_list(document: dict[str, Any], field: str) -> tuple[str, ...]:
    value = document.get(field)
    if (
        not isinstance(value, list)
        or any(not isinstance(item, str) or not item for item in value)
        or value != sorted(set(value))
    ):
        raise StructureBudgetError(f"`{field}` must be sorted unique non-empty strings")
    return tuple(value)


def require_non_negative_int(document: dict[str, Any], field: str) -> int:
    value = document.get(field)
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise StructureBudgetError(f"`{field}` must be a non-negative integer")
    return value


def validate_budget(document: dict[str, Any]) -> StructureBudget:
    if document.get("document_kind") != DOCUMENT_KIND:
        raise StructureBudgetError(f"document_kind must be `{DOCUMENT_KIND}`")
    schema_version = document.get("schema_version")
    if isinstance(schema_version, bool) or schema_version != SCHEMA_VERSION:
        raise StructureBudgetError(f"schema_version must be {SCHEMA_VERSION}")
    threshold = document.get("large_module_threshold_lines")
    if isinstance(threshold, bool) or threshold != LARGE_MODULE_THRESHOLD:
        raise StructureBudgetError(
            f"large_module_threshold_lines must be {LARGE_MODULE_THRESHOLD}"
        )
    allowed_large_modules = require_string_list(document, "allowed_large_modules")
    for path in allowed_large_modules:
        if (
            not path.startswith("crates/")
            or Path(path).is_absolute()
            or ".." in Path(path).parts
        ):
            raise StructureBudgetError(f"invalid large-module path: {path!r}")
    return StructureBudget(
        require_string_list(document, "workspace_packages"),
        require_string_list(document, "binary_targets"),
        allowed_large_modules,
        require_non_negative_int(document, "max_large_module_count"),
        require_non_negative_int(document, "max_module_lines"),
        require_non_negative_int(document, "max_total_owned_rust_lines"),
    )


def load_budget(path: Path) -> tuple[dict[str, Any], StructureBudget]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise StructureBudgetError(f"missing source-structure budget: {path}") from error
    except (OSError, json.JSONDecodeError) as error:
        raise StructureBudgetError(f"invalid source-structure budget {path}: {error}") from error
    if not isinstance(document, dict):
        raise StructureBudgetError("source-structure budget must be a JSON object")
    return document, validate_budget(document)


def compare(current: StructureSnapshot, budget: StructureBudget) -> tuple[list[str], list[str]]:
    failures: list[str] = []
    improvements: list[str] = []
    added_packages = sorted(set(current.workspace_packages) - set(budget.workspace_packages))
    if added_packages:
        failures.append(f"workspace packages added: {', '.join(added_packages)}")
    removed_packages = sorted(set(budget.workspace_packages) - set(current.workspace_packages))
    if removed_packages:
        improvements.append(f"workspace packages removed: {', '.join(removed_packages)}")

    added_binaries = sorted(set(current.binary_targets) - set(budget.binary_targets))
    if added_binaries:
        failures.append(f"binary targets added: {', '.join(added_binaries)}")
    removed_binaries = sorted(set(budget.binary_targets) - set(current.binary_targets))
    if removed_binaries:
        improvements.append(f"binary targets removed: {', '.join(removed_binaries)}")

    current_large = {
        path for path, lines in current.module_lines.items() if lines >= LARGE_MODULE_THRESHOLD
    }
    allowed_large = set(budget.allowed_large_modules)
    for path in sorted(current_large - allowed_large):
        failures.append(
            f"new thousand-line production module: {path} has "
            f"{current.module_lines[path]} lines"
        )
    removed_large = sorted(allowed_large - current_large)
    if removed_large:
        improvements.append(
            "large modules removed or split below "
            f"{LARGE_MODULE_THRESHOLD}: {', '.join(removed_large)}"
        )

    large_count = len(current_large)
    if large_count > budget.max_large_module_count:
        failures.append(
            f"large-module count grew: {large_count} > {budget.max_large_module_count}"
        )
    elif large_count < budget.max_large_module_count:
        improvements.append(
            f"large-module count shrank: {large_count} < {budget.max_large_module_count}"
        )

    largest = max(current.module_lines.values(), default=0)
    if largest > budget.max_module_lines:
        failures.append(f"largest module grew: {largest} > {budget.max_module_lines} lines")
    elif largest < budget.max_module_lines:
        improvements.append(f"largest module shrank: {largest} < {budget.max_module_lines} lines")

    total = sum(current.module_lines.values())
    if total > budget.max_total_owned_rust_lines:
        failures.append(
            "aggregate owned Rust source grew: "
            f"{total} > {budget.max_total_owned_rust_lines} lines"
        )
    elif total < budget.max_total_owned_rust_lines:
        improvements.append(
            "aggregate owned Rust source shrank: "
            f"{total} < {budget.max_total_owned_rust_lines} lines"
        )
    return failures, improvements


def budget_document(snapshot: StructureSnapshot) -> dict[str, Any]:
    large_modules = sorted(
        path for path, lines in snapshot.module_lines.items() if lines >= LARGE_MODULE_THRESHOLD
    )
    return {
        "_comment": (
            "One-way source ownership ceilings. Deletion and line-neutral ownership moves "
            "pass; new packages, binaries, 1000-line module paths, a larger maximum module, "
            "or aggregate owned Rust growth require an explicit reviewed update."
        ),
        "allowed_large_modules": large_modules,
        "binary_targets": list(snapshot.binary_targets),
        "document_kind": DOCUMENT_KIND,
        "large_module_threshold_lines": LARGE_MODULE_THRESHOLD,
        "max_large_module_count": len(large_modules),
        "max_module_lines": max(snapshot.module_lines.values(), default=0),
        "max_total_owned_rust_lines": sum(snapshot.module_lines.values()),
        "schema_version": SCHEMA_VERSION,
        "workspace_packages": list(snapshot.workspace_packages),
    }


def write_json_atomic(path: Path, document: dict[str, Any]) -> None:
    mode = stat.S_IMODE(path.stat().st_mode) if path.exists() else 0o644
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(document, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.chmod(temporary, mode)
        os.replace(temporary, path)
    except BaseException:
        temporary.unlink(missing_ok=True)
        raise


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--budget", type=Path, default=BUDGET_PATH, help=argparse.SUPPRESS)
    parser.add_argument("--update", action="store_true", help="lock in current improvements")
    args = parser.parse_args(argv)

    try:
        current = measure()
        if args.update and not args.budget.exists():
            write_json_atomic(args.budget, budget_document(current))
            print(f"[source-structure-budget] initialized {args.budget}")
            return 0
        _document, budget = load_budget(args.budget)
        failures, improvements = compare(current, budget)
    except (OSError, StructureBudgetError) as error:
        print(f"[source-structure-budget] ERROR: {error}", file=sys.stderr)
        return 2

    if failures:
        print("[source-structure-budget] FAIL:", file=sys.stderr)
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        return 1
    if args.update:
        try:
            write_json_atomic(args.budget, budget_document(current))
        except OSError as error:
            print(f"[source-structure-budget] ERROR: {error}", file=sys.stderr)
            return 2
        print(
            f"[source-structure-budget] tightened {args.budget}: "
            f"{len(improvements)} improvement(s) locked in"
        )
        return 0
    print(
        "[source-structure-budget] PASS: "
        f"{len(current.workspace_packages)} packages, "
        f"{len(current.binary_targets)} binaries, "
        f"{sum(lines >= LARGE_MODULE_THRESHOLD for lines in current.module_lines.values())} "
        "large owned modules, "
        f"{sum(current.module_lines.values())} owned Rust lines"
    )
    for improvement in improvements:
        print(f"  can tighten: {improvement}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
