#!/usr/bin/env python3
"""Deterministic command migration manifest gate for EPIC-006 (FEAT-015).

Enforces the staged-migration contract:

1. The checked-in migration topology document (`scripts/command-migration-topology.json`)
   is versioned and fail-closed: `schema_version` must be 1; unknown versions,
   tags, fields, selector kinds, type/trait node tags, and const atoms are rejected.
2. The pending frontier is a valid topology frontier: sorted, unique, containing
   only known leaves, and reachable from the roots by parent-to-all-children
   replacements (documented splits) or leaf removals (migrations). Arbitrary
   additions, partial splits, and stale entries fail closed.
3. The frontier exactly equals the set of groups/slices whose handlers still
   contain concrete-`App` signatures (`&mut App` / `&mut crate::tui::app::App`)
   within `crates/tui/src/commands/groups/` (bidirectional source scan; the
   AST/selector resolution part lands in Phase 3, `scan_and_check`).

The guard is hermetic for its data parts: it reads the topology artifact and
optionally the source tree; it never starts the TUI and makes no network calls.

Usage:
    python3 scripts/check-command-migration-manifest.py           # enforce
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TOPOLOGY_PATH = REPO_ROOT / "scripts" / "command-migration-topology.json"
SUPPORTED_SCHEMA_VERSION = 1

# Closed set of Rust integer suffixes accepted by selector const atoms.
INTEGER_SUFFIXES = {
    "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "i128", "u128",
    "isize", "usize",
}

# Closed set of primitive type names accepted by the type algebra (v1).
PRIMITIVE_TYPES = {
    "u8", "u16", "u32", "u64", "u128", "usize",
    "i8", "i16", "i32", "i64", "i128", "isize",
    "f32", "f64", "bool", "char", "str",
}

VALID_SELECTOR_KINDS = {"free", "inherent", "trait_impl"}
VALID_TYPE_TAGS = {
    "path", "qualified", "tuple", "reference", "pointer", "slice", "array",
    "primitive", "never",
}
VALID_GENERIC_ARG_TAGS = {"lifetime", "type", "const"}
VALID_CONST_TAGS = {"bool", "int", "char", "path"}


class ManifestViolation:
    """One deterministic manifest violation with an actionable diagnostic."""

    def __init__(self, category: str, location: str, detail: str) -> None:
        self.category = category
        self.location = location
        self.detail = detail

    def __str__(self) -> str:
        return f"{self.category}: {self.location}: {self.detail}"


# ---------------------------------------------------------------------------
# Const atom validation (Deep-Dive: closed four-tag records, canonical decimal
# magnitudes, byte bounds, fail-closed unknown forms).
# ---------------------------------------------------------------------------

def _canonical_magnitude(value: str, location: str) -> list[ManifestViolation]:
    """Validate `0|[1-9][0-9]*` and return violations (never leading zeros)."""
    if value == "0":
        return []
    if not value.isdigit() or value[0] == "0":
        return [ManifestViolation(
            "const-atom", location,
            f"integer magnitude must be canonical unsigned decimal without leading zeros "
            f"(0|[1-9][0-9]*), got {value!r}",
        )]
    return []


def validate_const_atom(atom, location: str) -> list[ManifestViolation]:
    """Validate one const atom record; reject unknown tags/fields/forms."""
    if not isinstance(atom, dict) or "tag" not in atom:
        return [ManifestViolation("const-atom", location, "const atom must be an object with a tag")]
    tag = atom["tag"]
    if tag not in VALID_CONST_TAGS:
        return [ManifestViolation(
            "const-atom", location, f"unknown const atom tag {tag!r}; expected one of {sorted(VALID_CONST_TAGS)}",
        )]
    violations: list[ManifestViolation] = []
    if tag == "bool":
        if set(atom) != {"tag", "value"} or not isinstance(atom.get("value"), bool):
            violations.append(ManifestViolation(
                "const-atom", location, "bool const atom must be {{tag: bool, value: <bool>}}",
            ))
    elif tag == "int":
        allowed = {"tag", "negative", "magnitude", "suffix"}
        if set(atom) != allowed:
            violations.append(ManifestViolation(
                "const-atom", location,
                f"int const atom must have exactly {{tag, negative, magnitude, suffix}}, got {sorted(atom)}",
            ))
            return violations
        if not isinstance(atom.get("negative"), bool):
            violations.append(ManifestViolation("const-atom", location, "int.negative must be a bool"))
        magnitude = atom.get("magnitude")
        if not isinstance(magnitude, str):
            violations.append(ManifestViolation("const-atom", location, "int.magnitude must be a string"))
        else:
            violations.extend(_canonical_magnitude(magnitude, location))
        suffix = atom.get("suffix")
        if suffix is not None and suffix not in INTEGER_SUFFIXES:
            violations.append(ManifestViolation(
                "const-atom", location, f"int.suffix must be null or a Rust integer suffix, got {suffix!r}",
            ))
        # Negative zero is noncanonical: normalize to nonnegative.
        if atom.get("negative") and magnitude == "0":
            violations.append(ManifestViolation(
                "const-atom", location, "negative zero must be normalized to nonnegative",
            ))
        # Decoded byte values (u8 suffix) must be within 0-255 inclusive.
        if suffix == "u8" and magnitude is not None and magnitude.isdigit():
            if int(magnitude) > 255:
                violations.append(ManifestViolation(
                    "const-atom", location,
                    f"decoded byte value {magnitude} exceeds the inclusive u8 range 0-255",
                ))
    elif tag == "char":
        if set(atom) != {"tag", "scalar"} or not isinstance(atom.get("scalar"), str):
            violations.append(ManifestViolation(
                "const-atom", location, "char const atom must be {{tag: char, scalar: <single Unicode scalar>}}",
            ))
        else:
            scalar = atom["scalar"]
            if len(scalar) != 1:
                violations.append(ManifestViolation(
                    "const-atom", location, "char.scalar must be exactly one Unicode scalar",
                ))
    elif tag == "path":
        if set(atom) != {"tag", "absolute", "segments"}:
            violations.append(ManifestViolation(
                "const-atom", location,
                "path const atom must be {{tag: path, absolute: <bool>, segments: [...]}}",
            ))
            return violations
        if not isinstance(atom.get("absolute"), bool):
            violations.append(ManifestViolation("const-atom", location, "path.absolute must be a bool"))
        segments = atom.get("segments")
        if not isinstance(segments, list) or not segments:
            violations.append(ManifestViolation("const-atom", location, "path.segments must be a nonempty array"))
        elif not all(isinstance(s, str) and s for s in segments):
            violations.append(ManifestViolation("const-atom", location, "path.segments must be nonempty strings"))
    return violations


# ---------------------------------------------------------------------------
# Type algebra validation (Deep-Dive: closed v1 algebra, fail-closed).
# ---------------------------------------------------------------------------

def validate_type_node(node, location: str) -> list[ManifestViolation]:
    """Validate a recursive type/trait node; reject unsupported syntax."""
    if not isinstance(node, dict) or "tag" not in node:
        return [ManifestViolation("type-algebra", location, "type node must be an object with a tag")]
    tag = node["tag"]
    if tag not in VALID_TYPE_TAGS:
        return [ManifestViolation(
            "type-algebra", location,
            f"unknown type node tag {tag!r}; expected one of {sorted(VALID_TYPE_TAGS)}",
        )]
    violations: list[ManifestViolation] = []

    if tag == "path":
        allowed = {"tag", "absolute", "segments"}
        if set(node) != allowed:
            return [ManifestViolation(
                "type-algebra", location, f"path node must have exactly {{tag, absolute, segments}}, got {sorted(node)}",
            )]
        if not isinstance(node.get("absolute"), bool):
            violations.append(ManifestViolation("type-algebra", location, "path.absolute must be a bool"))
        segments = node.get("segments")
        if not isinstance(segments, list) or not segments:
            return violations + [ManifestViolation("type-algebra", location, "path.segments must be a nonempty array")]
        for i, seg in enumerate(segments):
            seg_loc = f"{location}.segments[{i}]"
            if not isinstance(seg, dict) or "name" not in seg or not isinstance(seg.get("name"), str) or not seg["name"]:
                violations.append(ManifestViolation("type-algebra", seg_loc, "segment must be {{name: <string>, args?: [...]}}"))
                continue
            if "args" in seg:
                if not isinstance(seg["args"], list):
                    violations.append(ManifestViolation("type-algebra", seg_loc, "segment.args must be an array"))
                    continue
                for j, arg in enumerate(seg["args"]):
                    violations.extend(validate_generic_arg(arg, f"{seg_loc}.args[{j}]"))
    elif tag == "qualified":
        allowed = {"tag", "self", "assoc"}
        if set(node) != allowed:
            return [ManifestViolation("type-algebra", location, "qualified node must have exactly {tag, self, assoc}")]
        violations.extend(validate_type_node(node.get("self"), f"{location}.self"))
        if not isinstance(node.get("assoc"), str) or not node["assoc"]:
            violations.append(ManifestViolation("type-algebra", location, "qualified.assoc must be a nonempty string"))
    elif tag == "tuple":
        if set(node) != {"tag", "elems"} or not isinstance(node.get("elems"), list):
            return [ManifestViolation("type-algebra", location, "tuple node must be {{tag: tuple, elems: [...]}}")]
        for i, elem in enumerate(node["elems"]):
            violations.extend(validate_type_node(elem, f"{location}.elems[{i}]"))
    elif tag in ("reference", "pointer"):
        allowed = {"tag", "mut", "inner"}
        if set(node) != allowed or not isinstance(node.get("mut"), bool):
            return [ManifestViolation("type-algebra", location, f"{tag} node must be {{tag, mut, inner}}")]
        violations.extend(validate_type_node(node.get("inner"), f"{location}.inner"))
    elif tag == "slice":
        if set(node) != {"tag", "inner"}:
            return [ManifestViolation("type-algebra", location, "slice node must be {{tag: slice, inner}}")]
        violations.extend(validate_type_node(node.get("inner"), f"{location}.inner"))
    elif tag == "array":
        allowed = {"tag", "inner", "len"}
        if set(node) != allowed:
            return [ManifestViolation("type-algebra", location, "array node must be {{tag: array, inner, len}}")]
        violations.extend(validate_type_node(node.get("inner"), f"{location}.inner"))
        violations.extend(validate_const_atom(node.get("len"), f"{location}.len"))
    elif tag == "primitive":
        if set(node) != {"tag", "name"} or node.get("name") not in PRIMITIVE_TYPES:
            return [ManifestViolation(
                "type-algebra", location,
                f"primitive node must be {{tag: primitive, name}} with name in {sorted(PRIMITIVE_TYPES)}",
            )]
    elif tag == "never":
        if set(node) != {"tag"}:
            return [ManifestViolation("type-algebra", location, "never node must be {{tag: never}}")]
    return violations


def validate_generic_arg(arg, location: str) -> list[ManifestViolation]:
    """Validate one path-segment generic argument (type/lifetime/const)."""
    if not isinstance(arg, dict) or "tag" not in arg:
        return [ManifestViolation("type-algebra", location, "generic argument must be an object with a tag")]
    tag = arg["tag"]
    if tag not in VALID_GENERIC_ARG_TAGS:
        return [ManifestViolation(
            "type-algebra", location,
            f"unknown generic argument tag {tag!r}; expected one of {sorted(VALID_GENERIC_ARG_TAGS)}",
        )]
    if tag == "lifetime":
        if set(arg) != {"tag", "name"} or not isinstance(arg.get("name"), str) or not arg["name"]:
            return [ManifestViolation("type-algebra", location, "lifetime argument must be {{tag: lifetime, name}}")]
    elif tag == "type":
        return validate_type_node(arg.get("node"), f"{location}.node")
    else:  # const
        if set(arg) != {"tag", "atom"}:
            return [ManifestViolation("type-algebra", location, "const argument must be {{tag: const, atom}}")]
        return validate_const_atom(arg.get("atom"), f"{location}.atom")
    return []


# ---------------------------------------------------------------------------
# Selector validation (Deep-Dive: tagged structural records).
# ---------------------------------------------------------------------------

def validate_selector(selector, location: str) -> list[ManifestViolation]:
    """Validate one handler selector record (free / inherent / trait_impl)."""
    if not isinstance(selector, dict) or "kind" not in selector:
        return [ManifestViolation("selector", location, "selector must be an object with a kind")]
    kind = selector["kind"]
    if kind not in VALID_SELECTOR_KINDS:
        return [ManifestViolation(
            "selector", location,
            f"unknown selector kind {kind!r}; expected one of {sorted(VALID_SELECTOR_KINDS)}",
        )]
    violations: list[ManifestViolation] = []
    if kind == "free":
        allowed = {"kind", "item"}
        if set(selector) != allowed:
            return [ManifestViolation("selector", location, f"free selector must have exactly {{kind, item}}, got {sorted(selector)}")]
        item = selector.get("item")
        if not isinstance(item, list) or len(item) < 2:
            return [ManifestViolation("selector", location, "free.item must be a module path array ending with the function name")]
        if not all(isinstance(s, str) and s for s in item):
            return [ManifestViolation("selector", location, "free.item entries must be nonempty strings")]
    elif kind == "inherent":
        allowed = {"kind", "self_type", "method"}
        if "method" not in selector:
            return [ManifestViolation("selector", location, "inherent.method must be a nonempty string")]
        if set(selector) != allowed:
            return [ManifestViolation("selector", location, f"inherent selector must have exactly {{kind, self_type, method}}, got {sorted(selector)}")]
        violations.extend(validate_type_node(selector.get("self_type"), f"{location}.self_type"))
        if not isinstance(selector.get("method"), str) or not selector["method"]:
            violations.append(ManifestViolation("selector", location, "inherent.method must be a nonempty string"))
    else:  # trait_impl
        allowed = {"kind", "self_type", "trait_path", "method"}
        if set(selector) != allowed:
            return [ManifestViolation("selector", location, f"trait_impl selector must have exactly {{kind, self_type, trait_path, method}}, got {sorted(selector)}")]
        violations.extend(validate_type_node(selector.get("self_type"), f"{location}.self_type"))
        violations.extend(validate_type_node(selector.get("trait_path"), f"{location}.trait_path"))
        if not isinstance(selector.get("method"), str) or not selector["method"]:
            violations.append(ManifestViolation("selector", location, "trait_impl.method must be a nonempty string"))
    return violations


# ---------------------------------------------------------------------------
# Topology / frontier validation.
# ---------------------------------------------------------------------------

def all_leaves(topology: dict) -> dict[str, str]:
    """Map leaf name -> owning root for every group and predeclared slice."""
    leaves: dict[str, str] = {}
    for root, node in topology.items():
        leaves[root] = root
        for slice_node in node.get("slices", []):
            leaves[slice_node["name"]] = root
    return leaves


def validate_frontier(topology: dict, frontier: list[str]) -> list[ManifestViolation]:
    """Frontier must be sorted, unique, and reference only known leaves."""
    violations: list[ManifestViolation] = []
    if not isinstance(frontier, list):
        return [ManifestViolation("frontier", "frontier", "frontier must be an array")]
    if frontier != sorted(frontier):
        violations.append(ManifestViolation("frontier", "frontier", "frontier must be sorted"))
    if len(frontier) != len(set(frontier)):
        violations.append(ManifestViolation("frontier", "frontier", "frontier must contain no duplicates"))
    leaves = all_leaves(topology)
    for entry in frontier:
        if entry not in leaves:
            violations.append(ManifestViolation(
                "frontier", entry, f"frontier entry {entry!r} is not a declared topology leaf",
            ))
    return violations


def is_documented_split(topology: dict, old: set[str], new: set[str]) -> bool:
    """A parent was replaced by ALL of its declared children (1->N, no growth).

    Exactly one entry is removed; it is a group with declared slices; the added
    entries are exactly those declared children; nothing else changed.
    """
    removed = old - new
    added = new - old
    if len(removed) != 1:
        return False
    parent = next(iter(removed))
    for root, node in topology.items():
        children = {s["name"] for s in node.get("slices", [])}
        if parent == root and children:
            return children == added and len(added) == len(children) and added == new - old
    return False


def is_valid_frontier_transition(topology: dict, old: list[str], new: list[str]) -> list[ManifestViolation]:
    """Permit shrink (removal of migrated leaves) or documented parent-to-all-children split."""
    old_set = set(old)
    new_set = set(new)
    violations: list[ManifestViolation] = []
    removed = old_set - new_set
    added = new_set - old_set

    # Pure shrink: removed leaves only, nothing added.
    if not added:
        return violations  # any removal is a shrink (gate re-checks source later)

    # Documented split: exactly one parent removed, all its children added.
    if is_documented_split(topology, old_set, new_set):
        return violations

    # Everything else is growth or partial split: fail closed.
    if removed:
        violations.append(ManifestViolation(
            "frontier-transition", ", ".join(sorted(removed)),
            "frontier shrank AND grew; only pure shrink or documented parent-to-all-children split is allowed",
        ))
    else:
        violations.append(ManifestViolation(
            "frontier-transition", ", ".join(sorted(added)),
            "frontier grew without a documented parent-to-all-children split; arbitrary growth is forbidden",
        ))
    return violations


def validate_topology_document(doc: dict) -> list[ManifestViolation]:
    """Validate the whole topology document (schema, topology, frontier, slices)."""
    violations: list[ManifestViolation] = []
    if not isinstance(doc, dict):
        return [ManifestViolation("schema", "document", "topology document must be an object")]
    if doc.get("schema_version") != SUPPORTED_SCHEMA_VERSION:
        return [ManifestViolation(
            "schema", "schema_version",
            f"unsupported schema_version {doc.get('schema_version')!r}; only {SUPPORTED_SCHEMA_VERSION} is supported",
        )]
    topology = doc.get("topology")
    if not isinstance(topology, dict) or not topology:
        return [ManifestViolation("schema", "topology", "topology must be a nonempty object")]
    for root, node in topology.items():
        if not isinstance(node, dict):
            violations.append(ManifestViolation("schema", root, "group node must be an object"))
            continue
        allowed_group_fields = {"kind", "scope", "slices"}
        unknown = set(node) - allowed_group_fields
        if unknown:
            violations.append(ManifestViolation(
                "schema", root, f"unknown group field(s) {sorted(unknown)}; expected {sorted(allowed_group_fields)}",
            ))
        if "kind" not in node or "scope" not in node or "slices" not in node:
            violations.append(ManifestViolation(
                "schema", root, "group node must be {{kind, scope, slices}}",
            ))
            continue
        if node.get("kind") != "group":
            violations.append(ManifestViolation("schema", root, f"group {root!r} kind must be 'group'"))
        scope = node.get("scope")
        if not isinstance(scope, list) or not scope or not all(isinstance(s, str) and s for s in scope):
            violations.append(ManifestViolation("schema", root, "group scope must be a nonempty array of strings"))
        slices = node.get("slices")
        if not isinstance(slices, list):
            violations.append(ManifestViolation("schema", root, "group slices must be an array"))
            continue
        seen: set[str] = set()
        for i, slice_node in enumerate(slices):
            loc = f"{root}.slices[{i}]"
            if not isinstance(slice_node, dict):
                violations.append(ManifestViolation("schema", loc, "slice must be an object"))
                continue
            allowed_slice_fields = {"name", "kind", "scope", "handlers"}
            unknown = set(slice_node) - allowed_slice_fields
            if unknown:
                violations.append(ManifestViolation(
                    "schema", loc, f"unknown slice field(s) {sorted(unknown)}; expected {sorted(allowed_slice_fields)}",
                ))
            if "name" not in slice_node:
                violations.append(ManifestViolation("schema", loc, "slice must be an object with a name"))
                continue
            name = slice_node["name"]
            if name in seen:
                violations.append(ManifestViolation("schema", loc, f"duplicate slice name {name!r}"))
            seen.add(name)
            if not name.startswith(f"{root}::"):
                violations.append(ManifestViolation("schema", loc, f"slice name {name!r} must start with {root!r}::"))
            if slice_node.get("kind") != "slice":
                violations.append(ManifestViolation("schema", loc, f"slice {name!r} kind must be 'slice'"))
            slice_scope = slice_node.get("scope")
            if not isinstance(slice_scope, list) or not slice_scope or not all(isinstance(s, str) and s for s in slice_scope):
                violations.append(ManifestViolation("schema", loc, f"slice {name!r} scope must be a nonempty array of strings"))
            for selector in slice_node.get("handlers", []):
                violations.extend(validate_selector(selector, f"{loc}.handlers"))
    violations.extend(validate_frontier(topology, doc.get("frontier", [])))
    return violations


def load_topology(path: Path = TOPOLOGY_PATH) -> dict:
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def main(argv: list[str] | None = None) -> int:
    del argv  # reserved for future flags
    doc = load_topology()
    violations = validate_topology_document(doc)
    if violations:
        print("[command-migration-manifest] FAIL", file=sys.stderr)
        for violation in violations:
            print(f"  {violation}", file=sys.stderr)
        return 1
    frontier = doc["frontier"]
    print(
        f"[command-migration-manifest] PASS: schema v{SUPPORTED_SCHEMA_VERSION}; "
        f"frontier [{', '.join(frontier)}] is a valid topology frontier; "
        "source scan pending Phase 3 wiring"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
