#!/usr/bin/env python3
"""Analyze the public API of Connection and Cursor classes and report implementation status.

The script inspects the source code via the ``ast`` module so it does not
need to import or instantiate any driver classes (no native libraries
required).

Usage:
    python scripts/python_api_state.py
"""

from __future__ import annotations

import ast
import sys
from dataclasses import dataclass, field
from enum import Enum, auto
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CONNECTOR_PKG = REPO_ROOT / "python" / "src" / "snowflake" / "connector"

CONNECTION_FILE = CONNECTOR_PKG / "connection.py"
CURSOR_FILE = CONNECTOR_PKG / "cursor.py"


class Status(Enum):
    IMPLEMENTED = auto()
    NOT_IMPLEMENTED = auto()
    NOT_SUPPORTED = auto()


@dataclass
class MemberInfo:
    name: str
    kind: str  # "method", "property", "staticmethod", "classmethod"
    status: Status
    is_pep249: bool
    has_setter: bool = False


@dataclass
class ClassReport:
    class_name: str
    members: list[MemberInfo] = field(default_factory=list)

    @property
    def implemented(self) -> list[MemberInfo]:
        return [m for m in self.members if m.status is Status.IMPLEMENTED]

    @property
    def not_implemented(self) -> list[MemberInfo]:
        return [m for m in self.members if m.status is Status.NOT_IMPLEMENTED]

    @property
    def not_supported(self) -> list[MemberInfo]:
        return [m for m in self.members if m.status is Status.NOT_SUPPORTED]


def _has_decorator(node: ast.FunctionDef, name: str) -> bool:
    for dec in node.decorator_list:
        if isinstance(dec, ast.Name) and dec.id == name:
            return True
        if isinstance(dec, ast.Attribute) and dec.attr == name:
            return True
    return False


def _body_raises(node: ast.FunctionDef, exc_name: str) -> bool:
    """Return True if the function body consists solely of raising *exc_name*
    (possibly preceded by a docstring)."""
    stmts = node.body
    # skip leading docstring
    start = 0
    if (
        stmts
        and isinstance(stmts[0], ast.Expr)
        and isinstance(stmts[0].value, (ast.Constant, ast.Str))
    ):
        start = 1

    remaining = stmts[start:]
    if len(remaining) != 1:
        return False
    stmt = remaining[0]
    if not isinstance(stmt, ast.Raise) or stmt.exc is None:
        return False
    call = stmt.exc
    if isinstance(call, ast.Call) and isinstance(call.func, ast.Name):
        return call.func.id == exc_name
    if isinstance(call, ast.Name):
        return call.id == exc_name
    return False


def _determine_status(node: ast.FunctionDef) -> Status:
    if _body_raises(node, "NotImplementedError"):
        return Status.NOT_IMPLEMENTED
    if _body_raises(node, "NotSupportedError"):
        return Status.NOT_SUPPORTED
    return Status.IMPLEMENTED


def _determine_kind(node: ast.FunctionDef) -> str:
    if _has_decorator(node, "property"):
        return "property"
    if _has_decorator(node, "staticmethod"):
        return "staticmethod"
    if _has_decorator(node, "classmethod"):
        return "classmethod"
    return "method"


def _analyse_class(tree: ast.Module, class_name: str) -> ClassReport:
    report = ClassReport(class_name=class_name)

    cls_node: ast.ClassDef | None = None
    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef) and node.name == class_name:
            cls_node = node
            break

    if cls_node is None:
        return report

    setter_names: set[str] = set()
    for item in cls_node.body:
        if isinstance(item, ast.FunctionDef) and item.name.endswith(".setter"):
            setter_names.add(item.name.rsplit(".", 1)[0])
        if isinstance(item, ast.FunctionDef):
            for dec in item.decorator_list:
                if isinstance(dec, ast.Attribute) and dec.attr == "setter":
                    if isinstance(dec.value, ast.Name):
                        setter_names.add(dec.value.id)

    seen: set[str] = set()
    for item in cls_node.body:
        if not isinstance(item, ast.FunctionDef):
            continue
        name = item.name
        if name.startswith("_"):
            continue
        # skip property setters (reported on the getter)
        is_setter = any(
            isinstance(d, ast.Attribute) and d.attr == "setter"
            for d in item.decorator_list
        )
        if is_setter:
            continue
        if name in seen:
            continue
        seen.add(name)

        kind = _determine_kind(item)
        status = _determine_status(item)
        is_pep249 = _has_decorator(item, "pep249")
        has_setter = name in setter_names

        report.members.append(
            MemberInfo(
                name=name,
                kind=kind,
                status=status,
                is_pep249=is_pep249,
                has_setter=has_setter,
            )
        )

    return report


def _parse_file(path: Path) -> ast.Module:
    return ast.parse(path.read_text(encoding="utf-8"), filename=str(path))


STATUS_SYMBOLS = {
    Status.IMPLEMENTED: "\033[32m✔\033[0m",
    Status.NOT_IMPLEMENTED: "\033[31m✘\033[0m",
    Status.NOT_SUPPORTED: "\033[33m⊘\033[0m",
}

STATUS_LABELS = {
    Status.IMPLEMENTED: "implemented",
    Status.NOT_IMPLEMENTED: "NOT implemented",
    Status.NOT_SUPPORTED: "not supported",
}


def _print_report(report: ClassReport) -> None:
    total = len(report.members)
    n_impl = len(report.implemented)
    n_not_impl = len(report.not_implemented)
    n_not_supp = len(report.not_supported)
    pct = (n_impl / total * 100) if total else 0

    print(f"\n{'=' * 60}")
    print(f"  {report.class_name}")
    print(f"{'=' * 60}")
    print(
        f"  Total public API: {total}  |  "
        f"Implemented: {n_impl} ({pct:.0f}%)  |  "
        f"Not implemented: {n_not_impl}  |  "
        f"Not supported: {n_not_supp}"
    )
    print(f"{'-' * 60}")

    max_name = max((len(m.name) for m in report.members), default=0)

    for member in report.members:
        sym = STATUS_SYMBOLS[member.status]
        label = STATUS_LABELS[member.status]
        kind_tag = member.kind
        if member.has_setter:
            kind_tag += " (r/w)"
        pep_tag = "  [pep249]" if member.is_pep249 else ""
        print(f"  {sym} {member.name:<{max_name}}  {kind_tag:<20} {label}{pep_tag}")


def main() -> None:
    conn_tree = _parse_file(CONNECTION_FILE)
    cursor_tree = _parse_file(CURSOR_FILE)

    conn_report = _analyse_class(conn_tree, "Connection")
    cursor_report = _analyse_class(cursor_tree, "SnowflakeCursorBase")

    print("\nPython Driver API – Implementation Status Report")
    print("=" * 60)

    _print_report(conn_report)
    _print_report(cursor_report)

    # Summary
    all_members = conn_report.members + cursor_report.members
    total = len(all_members)
    impl = sum(1 for m in all_members if m.status is Status.IMPLEMENTED)
    not_impl = sum(1 for m in all_members if m.status is Status.NOT_IMPLEMENTED)
    not_supp = sum(1 for m in all_members if m.status is Status.NOT_SUPPORTED)
    pct = (impl / total * 100) if total else 0

    print(f"\n{'=' * 60}")
    print("  Overall Summary")
    print(f"{'=' * 60}")
    print(f"  Total:           {total}")
    print(f"  Implemented:     {impl} ({pct:.0f}%)")
    print(f"  Not implemented: {not_impl}")
    print(f"  Not supported:   {not_supp}")
    print()

    sys.exit(0 if not_impl == 0 else 1)


if __name__ == "__main__":
    main()
