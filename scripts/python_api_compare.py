#!/usr/bin/env python3
"""Compare the wrapper's public API against the reference snowflake-connector-python.

The script downloads the reference driver source from GitHub, then uses the
``ast`` module to compare public methods, properties, ``__all__`` exports,
exception classes, helper classes, and the ``pandas_tools`` module — covering
every area listed in [scope doc](https://docs.google.com/document/d/167q6SvqhrYGDmkK1xS8qAgl-EZYzn_jPguxB9jVmanA/edit?tab=t.0).

No native libraries or driver installation is required.

Prerequisites:
    - GitHub CLI (``gh``) must be installed and authenticated (``gh auth login``).
    - Network access to the GitHub API is required to download reference files.

Usage:
    python scripts/python_api_compare.py [--ref-tag TAG]

Options:
    --ref-tag TAG   Git tag or branch of the reference driver to compare
                    against (default: main)
"""

from __future__ import annotations

import ast
import argparse
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from enum import Enum, auto
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
WRAPPER_PKG = REPO_ROOT / "python" / "src" / "snowflake" / "connector"

REF_REPO = "snowflakedb/snowflake-connector-python"
REF_CONNECTOR_PREFIX = "src/snowflake/connector"

# Files to download from the reference driver
# Members excluded from the wrapper scope (not counted towards coverage).
# These are reference-driver internals or features intentionally omitted.
EXCLUDED_FROM_PUPR = {
    # crl configuration
    "allow_certificates_without_crl_url",
    "cert_revocation_check_mode",
    "crl_cache_cleanup_interval_hours",
    "crl_cache_dir",
    "crl_cache_removal_delay_days",
    "crl_cache_start_cleanup",
    "crl_cache_validity_hours",
    "crl_connection_timeout_ms",
    "crl_download_max_size",
    "crl_read_timeout_ms",
    "enable_crl_cache",
    "enable_crl_file_cache",
    # pandas helpers
    "build_location_helper",
    "chunk_helper",
    "make_pd_writer",
    # internal, but missing "_" prefix - should not be exposed
    "get_query_context",
    "set_query_context",
    "authenticate_with_retry",
    "cmd_query",
    "initialize_query_context_cache",
    "is_query_context_cache_disabled",
    "setup_ocsp_privatelink",
    # auth_class - not meant to be supported by the wrapper
    "auth_class",

}

REF_FILES = [
    f"{REF_CONNECTOR_PREFIX}/connection.py",
    f"{REF_CONNECTOR_PREFIX}/cursor.py",
    f"{REF_CONNECTOR_PREFIX}/__init__.py",
    f"{REF_CONNECTOR_PREFIX}/errors.py",
    f"{REF_CONNECTOR_PREFIX}/pandas_tools.py",
    f"{REF_CONNECTOR_PREFIX}/result_batch.py",
    f"{REF_CONNECTOR_PREFIX}/constants.py",
]


# ---------------------------------------------------------------------------
# Colours & symbols
# ---------------------------------------------------------------------------

GREEN = "\033[32m"
RED = "\033[31m"
YELLOW = "\033[33m"
CYAN = "\033[36m"
BOLD = "\033[1m"
RESET = "\033[0m"

SYM_OK = f"{GREEN}\u2714{RESET}"
SYM_MISS = f"{RED}\u2718{RESET}"
SYM_EXTRA = f"{CYAN}+{RESET}"
SYM_STUB = f"{YELLOW}\u25cb{RESET}"  # not-implemented stub
SYM_WARN = f"{YELLOW}\u26a0{RESET}"  # kind mismatch / informational warning
DIM = "\033[2m"
SYM_EXCLUDED = f"{DIM}\u2205{RESET}"  # excluded from scope


# ---------------------------------------------------------------------------
# Data model
# ---------------------------------------------------------------------------


class ImplStatus(Enum):
    IMPLEMENTED = auto()
    NOT_IMPLEMENTED = auto()
    NOT_SUPPORTED = auto()


@dataclass
class MemberInfo:
    name: str
    kind: str  # "method", "property", "staticmethod", "classmethod"
    status: ImplStatus = ImplStatus.IMPLEMENTED
    is_pep249: bool = False


@dataclass
class ClassAPI:
    class_name: str
    members: dict[str, MemberInfo] = field(default_factory=dict)

    @property
    def member_names(self) -> set[str]:
        return set(self.members.keys())


# ---------------------------------------------------------------------------
# AST helpers (reused from python_api_state.py)
# ---------------------------------------------------------------------------


def _has_decorator(node: ast.FunctionDef, name: str) -> bool:
    for dec in node.decorator_list:
        if isinstance(dec, ast.Name) and dec.id == name:
            return True
        if isinstance(dec, ast.Attribute) and dec.attr == name:
            return True
    return False


def _body_raises(node: ast.FunctionDef, exc_name: str) -> bool:
    stmts = node.body
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


def _determine_status(node: ast.FunctionDef) -> ImplStatus:
    if _body_raises(node, "NotImplementedError"):
        return ImplStatus.NOT_IMPLEMENTED
    if _body_raises(node, "NotSupportedError"):
        return ImplStatus.NOT_SUPPORTED
    return ImplStatus.IMPLEMENTED


def _determine_kind(node: ast.FunctionDef) -> str:
    if _has_decorator(node, "property"):
        return "property"
    if _has_decorator(node, "staticmethod"):
        return "staticmethod"
    if _has_decorator(node, "classmethod"):
        return "classmethod"
    return "method"


def _extract_class_api(tree: ast.Module, class_name: str, *, check_status: bool = False) -> ClassAPI:
    """Extract public members from a class in an AST tree.

    When *check_status* is True (wrapper), also determine implementation
    status.  For the reference driver this is skipped (everything is
    assumed present).
    """
    api = ClassAPI(class_name=class_name)

    cls_node: ast.ClassDef | None = None
    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef) and node.name == class_name:
            cls_node = node
            break
    if cls_node is None:
        return api

    seen: set[str] = set()
    for item in cls_node.body:
        if not isinstance(item, ast.FunctionDef):
            continue
        name = item.name
        if name.startswith("_"):
            continue
        # Skip property setters
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
        status = _determine_status(item) if check_status else ImplStatus.IMPLEMENTED
        is_pep249 = _has_decorator(item, "pep249")

        api.members[name] = MemberInfo(
            name=name,
            kind=kind,
            status=status,
            is_pep249=is_pep249,
        )
    return api


def _extract_all_list(tree: ast.Module) -> list[str]:
    """Extract the ``__all__`` list from a module."""
    for node in ast.walk(tree):
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id == "__all__":
                    if isinstance(node.value, ast.List):
                        return [
                            elt.value
                            for elt in node.value.elts
                            if isinstance(elt, ast.Constant) and isinstance(elt.value, str)
                            and not elt.value.startswith("_")
                        ]
    return []


def _extract_exception_classes(tree: ast.Module) -> dict[str, list[str]]:
    """Extract exception class names and their base classes.

    Uses a two-pass approach so the result is independent of ``ast.walk``
    traversal order: first collect every class and its bases, then
    determine which are exceptions by walking the inheritance graph.
    """
    all_classes: dict[str, list[str]] = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef):
            if node.name.startswith("_"):
                continue
            bases: list[str] = []
            for base in node.bases:
                if isinstance(base, ast.Name):
                    bases.append(base.id)
                elif isinstance(base, ast.Attribute):
                    bases.append(base.attr)
            all_classes[node.name] = bases

    # Seed: well-known root exception/warning types
    roots = {"Exception", "Warning", "Error", "BaseException"}
    exceptions: dict[str, list[str]] = {}
    changed = True
    while changed:
        changed = False
        for name, bases in all_classes.items():
            if name in exceptions:
                continue
            if any(b in roots or b in exceptions for b in bases):
                exceptions[name] = bases
                changed = True
    return exceptions


@dataclass
class ParamInfo:
    """A single parameter of a function / __init__."""
    name: str
    has_default: bool = False


@dataclass
class ClassStructure:
    """Detailed structural information about a helper class."""
    name: str
    kind: str  # "namedtuple", "enum", "class", "alias", "not_found"
    # NamedTuple / __init__ parameters (excluding 'self' / 'cls')
    fields: list[ParamInfo] = field(default_factory=list)
    # Enum member names (for Enum subclasses)
    enum_members: list[str] = field(default_factory=list)
    # Public methods and properties (name -> kind)
    public_members: dict[str, str] = field(default_factory=dict)


def _is_namedtuple_class(cls_node: ast.ClassDef) -> bool:
    """Check whether *cls_node* inherits from ``NamedTuple``."""
    for base in cls_node.bases:
        if isinstance(base, ast.Name) and base.id == "NamedTuple":
            return True
        if isinstance(base, ast.Attribute) and base.attr == "NamedTuple":
            return True
    return False


def _is_enum_class(cls_node: ast.ClassDef) -> bool:
    """Check whether *cls_node* inherits from ``Enum`` (or a subclass)."""
    enum_bases = {"Enum", "IntEnum", "Flag", "IntFlag"}
    for base in cls_node.bases:
        if isinstance(base, ast.Name) and base.id in enum_bases:
            return True
        if isinstance(base, ast.Attribute) and base.attr in enum_bases:
            return True
    return False


def _extract_namedtuple_fields(cls_node: ast.ClassDef) -> list[ParamInfo]:
    """Extract field names from a NamedTuple class body (annotated assignments)."""
    fields: list[ParamInfo] = []
    for item in cls_node.body:
        if isinstance(item, ast.AnnAssign) and isinstance(item.target, ast.Name):
            fields.append(ParamInfo(
                name=item.target.id,
                has_default=item.value is not None,
            ))
    return fields


def _extract_init_params(cls_node: ast.ClassDef) -> list[ParamInfo]:
    """Extract ``__init__`` parameters (excluding *self*)."""
    for item in cls_node.body:
        if isinstance(item, ast.FunctionDef) and item.name == "__init__":
            args = item.args
            # Number of args that have defaults
            num_defaults = len(args.defaults)
            all_args = args.args[1:]  # skip 'self'
            params: list[ParamInfo] = []
            for i, arg in enumerate(all_args):
                # defaults are right-aligned: last num_defaults args have defaults
                has_default = i >= len(all_args) - num_defaults
                params.append(ParamInfo(name=arg.arg, has_default=has_default))
            # keyword-only args
            kw_defaults = args.kw_defaults  # None entries mean no default
            for arg, default in zip(args.kwonlyargs, kw_defaults):
                params.append(ParamInfo(name=arg.arg, has_default=default is not None))
            return params
    return []


def _extract_enum_members(cls_node: ast.ClassDef) -> list[str]:
    """Extract member names from an Enum class body."""
    members: list[str] = []
    for item in cls_node.body:
        if isinstance(item, ast.Assign):
            for target in item.targets:
                if isinstance(target, ast.Name) and not target.id.startswith("_"):
                    members.append(target.id)
    return members


def _extract_public_members(cls_node: ast.ClassDef) -> dict[str, str]:
    """Extract public methods and properties from a class (name -> kind)."""
    members: dict[str, str] = {}
    for item in cls_node.body:
        if not isinstance(item, ast.FunctionDef):
            continue
        if item.name.startswith("_"):
            continue
        # Skip property setters
        is_setter = any(
            isinstance(d, ast.Attribute) and d.attr == "setter"
            for d in item.decorator_list
        )
        if is_setter:
            continue
        members[item.name] = _determine_kind(item)
    return members


def _extract_class_structure(tree: ast.Module, class_name: str) -> ClassStructure:
    """Extract detailed structural information about a class."""
    # Check if it's an alias first
    for node in ast.iter_child_nodes(tree):
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id == class_name:
                    if isinstance(node.value, ast.Name):
                        return ClassStructure(
                            name=class_name,
                            kind="alias",
                            public_members={"(alias of)": node.value.id},
                        )

    cls_node: ast.ClassDef | None = None
    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef) and node.name == class_name:
            cls_node = node
            break
    if cls_node is None:
        return ClassStructure(name=class_name, kind="not_found")

    if _is_namedtuple_class(cls_node):
        return ClassStructure(
            name=class_name,
            kind="namedtuple",
            fields=_extract_namedtuple_fields(cls_node),
            public_members=_extract_public_members(cls_node),
        )

    if _is_enum_class(cls_node):
        return ClassStructure(
            name=class_name,
            kind="enum",
            enum_members=_extract_enum_members(cls_node),
            public_members=_extract_public_members(cls_node),
        )

    return ClassStructure(
        name=class_name,
        kind="class",
        fields=_extract_init_params(cls_node),
        public_members=_extract_public_members(cls_node),
    )


def _find_class_structure(
    class_name: str,
    trees: list[ast.Module | None],
) -> ClassStructure:
    """Search for *class_name* across multiple parsed trees, return the first match."""
    for tree in trees:
        if tree is None:
            continue
        struct = _extract_class_structure(tree, class_name)
        if struct.kind != "not_found":
            return struct
    return ClassStructure(name=class_name, kind="not_found")


def _compare_class_structure(
    ref: ClassStructure,
    wrap: ClassStructure,
) -> ComparisonResult:
    """Compare detailed structure of a single helper class."""
    result = ComparisonResult(section=f"  {ref.name}")

    # --- Fields / __init__ params ---
    if ref.kind == "alias" and wrap.kind == "alias":
        ref_alias = list(ref.public_members.values())[0] if ref.public_members else "?"
        wrap_alias = list(wrap.public_members.values())[0] if wrap.public_members else "?"
        if ref_alias == wrap_alias:
            result.add(SYM_OK, f"(alias of {ref_alias})", "matches")
        else:
            result.add(SYM_MISS, f"(alias)", f"ref -> {ref_alias}, wrapper -> {wrap_alias}")
        return result

    if ref.kind == "alias" or wrap.kind == "alias":
        # One is alias, other is a real class — flag the kind mismatch
        # and, if the reference is a real class, list its fields as missing
        result.add(SYM_WARN, "(kind mismatch)", f"ref: {ref.kind}, wrapper: {wrap.kind}")
        if wrap.kind == "alias" and ref.kind in ("class", "namedtuple"):
            for f in ref.fields:
                opt = " (optional)" if f.has_default else " (required)"
                result.add(SYM_MISS, f.name, f"ref field/param not in wrapper alias{opt}")
            for name, kind in sorted(ref.public_members.items()):
                result.add(SYM_MISS, name, f"ref public {kind} not in wrapper alias")
        return result

    if ref.kind == "enum":
        ref_set = set(ref.enum_members)
        wrap_set = set(wrap.enum_members)
        for name in sorted(ref_set | wrap_set):
            if name in ref_set and name not in wrap_set:
                result.add(SYM_MISS, name, "enum member missing from wrapper")
            elif name in wrap_set and name not in ref_set:
                result.add(SYM_EXTRA, name, "wrapper-only enum member")
            else:
                result.add(SYM_OK, name, "enum member")
    else:
        # Compare fields / __init__ params
        ref_field_names = [f.name for f in ref.fields]
        wrap_field_names = [f.name for f in wrap.fields]
        ref_field_set = set(ref_field_names)
        wrap_field_set = set(wrap_field_names)
        ref_field_map = {f.name: f for f in ref.fields}
        wrap_field_map = {f.name: f for f in wrap.fields}

        # Use ordered union (ref order first, then any extras from wrapper)
        ordered = list(ref_field_names)
        for n in wrap_field_names:
            if n not in ref_field_set:
                ordered.append(n)

        for name in ordered:
            in_ref = name in ref_field_set
            in_wrap = name in wrap_field_set

            if in_ref and not in_wrap:
                opt = " (optional)" if ref_field_map[name].has_default else " (required)"
                label = "namedtuple field" if ref.kind == "namedtuple" else "__init__ param"
                result.add(SYM_MISS, name, f"{label} missing from wrapper{opt}")
            elif in_wrap and not in_ref:
                result.add(SYM_EXTRA, name, "wrapper-only")
            else:
                result.add(SYM_OK, name, "field/param")

    # --- Public methods / properties ---
    ref_pub = ref.public_members
    wrap_pub = wrap.public_members
    all_pub = sorted(set(ref_pub) | set(wrap_pub))
    for name in all_pub:
        in_ref = name in ref_pub
        in_wrap = name in wrap_pub

        if in_ref and not in_wrap:
            result.add(SYM_MISS, name, f"public {ref_pub[name]} missing from wrapper")
        elif in_wrap and not in_ref:
            result.add(SYM_EXTRA, name, f"wrapper-only {wrap_pub[name]}")
        else:
            result.add(SYM_OK, name, f"{ref_pub[name]}")

    return result


def _extract_top_level_functions(tree: ast.Module) -> set[str]:
    """Extract names of top-level (non-private) functions."""
    result = set()
    for node in ast.iter_child_nodes(tree):
        if isinstance(node, ast.FunctionDef) and not node.name.startswith("_"):
            result.add(node.name)
    return result


# ---------------------------------------------------------------------------
# Reference driver download
# ---------------------------------------------------------------------------


GH_TIMEOUT_SECONDS = 30


def _download_ref_files(ref_tag: str, dest: Path) -> tuple[Path, int]:
    """Download reference driver files from GitHub using gh CLI.

    Returns:
        A tuple of (destination directory, number of files successfully downloaded).
    """
    connector_dest = dest / "connector"
    connector_dest.mkdir(parents=True, exist_ok=True)

    import base64

    success_count = 0
    for ref_path in REF_FILES:
        local_name = Path(ref_path).name
        target = connector_dest / local_name
        cmd = [
            "gh", "api",
            f"repos/{REF_REPO}/contents/{ref_path}?ref={ref_tag}",
            "--jq", ".content",
        ]
        try:
            result = subprocess.run(
                cmd, capture_output=True, text=True, timeout=GH_TIMEOUT_SECONDS,
            )
        except FileNotFoundError:
            print(
                f"\n  {RED}Error: 'gh' CLI not found. Install it from https://cli.github.com/ "
                f"and run 'gh auth login'.{RESET}",
                file=sys.stderr,
            )
            return connector_dest, 0
        except subprocess.TimeoutExpired:
            print(f"  {RED}Timeout downloading {ref_path}{RESET}", file=sys.stderr)
            continue
        if result.returncode != 0:
            print(f"  {RED}Failed to download {ref_path}: {result.stderr.strip()}{RESET}", file=sys.stderr)
            continue
        content = base64.b64decode(result.stdout.strip()).decode("utf-8")
        target.write_text(content, encoding="utf-8")
        success_count += 1

    return connector_dest, success_count


def _parse_file(path: Path) -> ast.Module | None:
    if not path.exists():
        return None
    try:
        return ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    except (SyntaxError, UnicodeDecodeError) as exc:
        print(f"  {RED}Failed to parse {path}: {exc}{RESET}", file=sys.stderr)
        return None


# ---------------------------------------------------------------------------
# Comparison logic
# ---------------------------------------------------------------------------

@dataclass
class ComparisonResult:
    section: str
    items: list[tuple[str, str, str]] = field(default_factory=list)  # (symbol, name, detail)

    def add(self, symbol: str, name: str, detail: str = "") -> None:
        self.items.append((symbol, name, detail))

    @property
    def missing_count(self) -> int:
        return sum(1 for sym, _, _ in self.items if sym == SYM_MISS)

    @property
    def stub_count(self) -> int:
        return sum(1 for sym, _, _ in self.items if sym == SYM_STUB)

    @property
    def excluded_count(self) -> int:
        return sum(1 for sym, _, _ in self.items if sym == SYM_EXCLUDED)


def _compare_class(
    ref_api: ClassAPI,
    wrapper_api: ClassAPI,
    label: str,
) -> ComparisonResult:
    result = ComparisonResult(section=label)
    all_names = sorted(ref_api.member_names | wrapper_api.member_names)

    for name in all_names:
        ref_member = ref_api.members.get(name)
        wrap_member = wrapper_api.members.get(name)

        if ref_member and not wrap_member:
            if name in EXCLUDED_FROM_PUPR:
                result.add(SYM_EXCLUDED, name, f"excluded ({ref_member.kind} in reference)")
            else:
                result.add(SYM_MISS, name, f"missing ({ref_member.kind} in reference)")
        elif wrap_member and not ref_member:
            pep_tag = " [pep249]" if wrap_member.is_pep249 else ""
            result.add(SYM_EXTRA, name, f"wrapper-only {wrap_member.kind}{pep_tag}")
        elif wrap_member and ref_member:
            if name in EXCLUDED_FROM_PUPR:
                result.add(SYM_EXCLUDED, name, f"excluded")
            elif wrap_member.status is ImplStatus.NOT_IMPLEMENTED:
                result.add(SYM_STUB, name, f"stub (raises NotImplementedError)")
            elif wrap_member.status is ImplStatus.NOT_SUPPORTED:
                result.add(SYM_OK, name, f"not supported (by design)")
            else:
                pep_tag = " [pep249]" if wrap_member.is_pep249 else ""
                result.add(SYM_OK, name, f"implemented{pep_tag}")

    return result


def _compare_all_list(ref_all: list[str], wrapper_all: list[str]) -> ComparisonResult:
    result = ComparisonResult(section="__all__ exports")
    ref_set = set(ref_all)
    wrap_set = set(wrapper_all)
    all_names = sorted(ref_set | wrap_set)

    for name in all_names:
        in_ref = name in ref_set
        in_wrap = name in wrap_set

        if in_ref and not in_wrap:
            result.add(SYM_MISS, name, "in reference but missing from wrapper")
        elif in_wrap and not in_ref:
            result.add(SYM_EXTRA, name, "wrapper-only export")
        else:
            result.add(SYM_OK, name, "present in both")

    return result


def _compare_exceptions(
    ref_exc: dict[str, list[str]],
    wrapper_exc: dict[str, list[str]],
) -> ComparisonResult:
    result = ComparisonResult(section="Exception classes (errors.py)")
    ref_names = set(ref_exc.keys())
    wrap_names = set(wrapper_exc.keys())
    all_names = sorted(ref_names | wrap_names)

    for name in all_names:
        in_ref = name in ref_names
        in_wrap = name in wrap_names

        if in_ref and not in_wrap:
            bases = ", ".join(ref_exc[name])
            result.add(SYM_MISS, name, f"missing (inherits: {bases})")
        elif in_wrap and not in_ref:
            result.add(SYM_EXTRA, name, "wrapper-only exception")
        else:
            ref_bases = set(ref_exc[name])
            wrap_bases = set(wrapper_exc[name])
            if ref_bases == wrap_bases:
                result.add(SYM_OK, name, f"matches (inherits: {', '.join(sorted(ref_bases))})")
            else:
                result.add(SYM_WARN, name,
                           f"base mismatch (ref: {', '.join(sorted(ref_bases))}; "
                           f"wrapper: {', '.join(sorted(wrap_bases))})")

    return result


def _compare_module_functions(
    ref_funcs: set[str],
    wrapper_funcs: set[str],
    label: str,
) -> ComparisonResult:
    result = ComparisonResult(section=label)
    all_names = sorted(ref_funcs | wrapper_funcs)

    for name in all_names:
        if name in ref_funcs and name not in wrapper_funcs:
            if name in EXCLUDED_FROM_PUPR:
                result.add(SYM_EXCLUDED, name, "excluded")
            else:
                result.add(SYM_MISS, name, "missing from wrapper")
        elif name in wrapper_funcs and name not in ref_funcs:
            result.add(SYM_EXTRA, name, "wrapper-only")
        else:
            result.add(SYM_OK, name, "present")

    return result


# ---------------------------------------------------------------------------
# Report printing
# ---------------------------------------------------------------------------


def _print_section(comp: ComparisonResult) -> None:
    missing = comp.missing_count
    stubs = comp.stub_count
    excluded = comp.excluded_count
    warns = sum(1 for s, _, _ in comp.items if s == SYM_WARN)
    extras = sum(1 for s, _, _ in comp.items if s == SYM_EXTRA)
    total = len(comp.items) - excluded
    ok = total - missing - stubs - warns - extras

    print(f"\n{BOLD}{'=' * 70}")
    print(f"  {comp.section}")
    print(f"{'=' * 70}{RESET}")

    status_parts = [f"Total: {total}"]
    if ok:
        status_parts.append(f"{GREEN}OK: {ok}{RESET}")
    if stubs:
        status_parts.append(f"{YELLOW}Stubs: {stubs}{RESET}")
    if warns:
        status_parts.append(f"{YELLOW}Warns: {warns}{RESET}")
    if missing:
        status_parts.append(f"{RED}Missing: {missing}{RESET}")
    if excluded:
        status_parts.append(f"{DIM}Excluded: {excluded}{RESET}")

    print(f"  {'  |  '.join(status_parts)}")
    print(f"{'-' * 70}")

    max_name = max((len(n) for _, n, _ in comp.items), default=0)
    for sym, name, detail in comp.items:
        if sym == SYM_EXCLUDED:
            print(f"  {sym} {DIM}{name:<{max_name}}  {detail}{RESET}")
        else:
            print(f"  {sym} {name:<{max_name}}  {detail}")


def _pct(n: int, base: int) -> str:
    """Format *n* as a percentage of *base*, e.g. ``' (42%)'``."""
    if base == 0:
        return ""
    return f" ({n * 100 / base:.0f}%)"


def _print_overall_summary(sections: list[ComparisonResult]) -> None:
    total_missing = sum(s.missing_count for s in sections)
    total_stubs = sum(s.stub_count for s in sections)
    total_excluded = sum(s.excluded_count for s in sections)
    total_all = sum(len(s.items) for s in sections)
    total_extra = sum(1 for s in sections for sym, _, _ in s.items if sym == SYM_EXTRA)
    total_warns = sum(1 for s in sections for sym, _, _ in s.items if sym == SYM_WARN)
    # Excluded items are not counted in the totals
    total_items = total_all - total_excluded
    total_ok = total_items - total_missing - total_stubs - total_extra - total_warns
    # Base for percentages: everything except wrapper-only and excluded items
    base = total_items - total_extra

    print(f"\n{BOLD}{'=' * 70}")
    print(f"  Overall Summary")
    print(f"{'=' * 70}{RESET}")
    print(f"  Total compared:      {total_items}")
    print(f"  {GREEN}Present & OK:        {total_ok}{_pct(total_ok, base)}{RESET}")
    print(f"  {YELLOW}Stubs (not impl):    {total_stubs}{_pct(total_stubs, base)}{RESET}")
    print(f"  {RED}Missing from wrapper: {total_missing}{_pct(total_missing, base)}{RESET}")
    print(f"  {CYAN}Wrapper-only:        {total_extra}{RESET}")
    if total_excluded:
        print(f"  {DIM}Excluded from scope: {total_excluded}{RESET}")

    if total_missing > 0:
        print(f"\n  {RED}{BOLD}Wrapper is missing {total_missing} member(s) from the reference driver.{RESET}")
    else:
        print(f"\n  {GREEN}{BOLD}Wrapper covers all reference driver public API members!{RESET}")

    print()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    parser = argparse.ArgumentParser(description="Compare wrapper API against reference snowflake-connector-python")
    parser.add_argument("--ref-tag", default="main", help="Git tag or branch of the reference driver (default: main)")
    args = parser.parse_args()

    # Download reference files
    print(f"{BOLD}Downloading reference driver ({REF_REPO} @ {args.ref_tag})...{RESET}")
    with tempfile.TemporaryDirectory(prefix="sf_ref_") as tmpdir:
        ref_dir, download_count = _download_ref_files(args.ref_tag, Path(tmpdir))
        if download_count == 0:
            print(f"\n  {RED}{BOLD}No reference files could be downloaded. "
                  f"Check that 'gh' is installed and authenticated.{RESET}")
            sys.exit(2)

        # Parse all files
        ref_conn_tree = _parse_file(ref_dir / "connection.py")
        ref_cursor_tree = _parse_file(ref_dir / "cursor.py")
        ref_init_tree = _parse_file(ref_dir / "__init__.py")
        ref_errors_tree = _parse_file(ref_dir / "errors.py")
        ref_pandas_tree = _parse_file(ref_dir / "pandas_tools.py")

        wrap_conn_tree = _parse_file(WRAPPER_PKG / "connection.py")
        wrap_cursor_tree = _parse_file(WRAPPER_PKG / "cursor.py")
        wrap_init_tree = _parse_file(WRAPPER_PKG / "__init__.py")
        wrap_errors_tree = _parse_file(WRAPPER_PKG / "errors.py")
        wrap_pandas_tree = _parse_file(WRAPPER_PKG / "pandas_tools.py")

        sections: list[ComparisonResult] = []
        parse_failures: list[str] = []

        print(f"\n{BOLD}Python Wrapper vs Reference Driver — API Comparison{RESET}")
        print(f"{'=' * 70}")

        # --- Section 1+2: Connection class ---
        if ref_conn_tree and wrap_conn_tree:
            # Reference uses SnowflakeConnection as the main class
            ref_conn_api = _extract_class_api(ref_conn_tree, "SnowflakeConnection")
            if not ref_conn_api.members:
                # Fallback: some versions use Connection
                ref_conn_api = _extract_class_api(ref_conn_tree, "Connection")
            wrap_conn_api = _extract_class_api(wrap_conn_tree, "Connection", check_status=True)
            conn_comp = _compare_class(ref_conn_api, wrap_conn_api, "Connection (public methods & properties)")
            _print_section(conn_comp)
            sections.append(conn_comp)
        else:
            parse_failures.append("connection")
            print(f"\n  {RED}Could not parse connection files{RESET}")

        # --- Section 1+2: Cursor class ---
        if ref_cursor_tree and wrap_cursor_tree:
            # Reference: SnowflakeCursor inherits from SnowflakeCursorBase (if exists)
            # We check SnowflakeCursor in ref (which is the main class users interact with)
            ref_cursor_api = _extract_class_api(ref_cursor_tree, "SnowflakeCursor")
            # Also get members from the base class in reference
            ref_cursor_base_api = _extract_class_api(ref_cursor_tree, "SnowflakeCursorBase")
            # Merge base into cursor (base members not overridden)
            for name, member in ref_cursor_base_api.members.items():
                if name not in ref_cursor_api.members:
                    ref_cursor_api.members[name] = member

            # Wrapper: SnowflakeCursorBase has all the methods
            wrap_cursor_api = _extract_class_api(wrap_cursor_tree, "SnowflakeCursorBase", check_status=True)
            # Also get SnowflakeCursor members
            wrap_cursor_extra = _extract_class_api(wrap_cursor_tree, "SnowflakeCursor", check_status=True)
            for name, member in wrap_cursor_extra.members.items():
                if name not in wrap_cursor_api.members:
                    wrap_cursor_api.members[name] = member

            cursor_comp = _compare_class(ref_cursor_api, wrap_cursor_api, "Cursor (public methods & properties)")
            _print_section(cursor_comp)
            sections.append(cursor_comp)
        else:
            parse_failures.append("cursor")
            print(f"\n  {RED}Could not parse cursor files{RESET}")

        # --- Section 2: __all__ exports ---
        if ref_init_tree and wrap_init_tree:
            ref_all = _extract_all_list(ref_init_tree)
            wrap_all = _extract_all_list(wrap_init_tree)
            all_comp = _compare_all_list(ref_all, wrap_all)
            _print_section(all_comp)
            sections.append(all_comp)
        else:
            parse_failures.append("__init__")
            print(f"\n  {RED}Could not parse __init__ files{RESET}")

        # --- Section 4: Exception hierarchy ---
        if ref_errors_tree and wrap_errors_tree:
            ref_exc = _extract_exception_classes(ref_errors_tree)
            wrap_exc = _extract_exception_classes(wrap_errors_tree)
            exc_comp = _compare_exceptions(ref_exc, wrap_exc)
            _print_section(exc_comp)
            sections.append(exc_comp)
        else:
            parse_failures.append("errors")
            print(f"\n  {RED}Could not parse errors files{RESET}")

        # --- Section 5: Helper classes ---
        ref_result_batch_tree = _parse_file(ref_dir / "result_batch.py")
        ref_constants_tree = _parse_file(ref_dir / "constants.py")

        # Also check wrapper equivalents
        wrap_result_batch_tree = _parse_file(WRAPPER_PKG / "result_batch.py")
        wrap_constants_tree = _parse_file(WRAPPER_PKG / "constants.py")

        helper_classes = ["DictCursor", "ResultMetadata", "ResultMetadataV2", "ResultBatch", "QueryStatus"]

        ref_trees = [ref_conn_tree, ref_cursor_tree, ref_init_tree, ref_errors_tree,
                     ref_result_batch_tree, ref_constants_tree]
        wrap_trees = [wrap_conn_tree, wrap_cursor_tree, wrap_init_tree, wrap_errors_tree,
                      wrap_result_batch_tree, wrap_constants_tree]

        # Presence overview
        helper_result = ComparisonResult(section="Helper classes — presence")
        for cls_name in helper_classes:
            if cls_name in EXCLUDED_FROM_PUPR:
                helper_result.add(SYM_EXCLUDED, cls_name, "excluded from scope")
                continue

            ref_struct = _find_class_structure(cls_name, ref_trees)
            wrap_struct = _find_class_structure(cls_name, wrap_trees)
            in_ref = ref_struct.kind != "not_found"
            in_wrap = wrap_struct.kind != "not_found"

            if in_ref and not in_wrap:
                helper_result.add(SYM_MISS, cls_name, f"missing from wrapper (ref kind: {ref_struct.kind})")
            elif in_wrap and not in_ref:
                helper_result.add(SYM_EXTRA, cls_name, "wrapper-only")
            elif in_ref and in_wrap:
                helper_result.add(SYM_OK, cls_name, f"present in both (ref: {ref_struct.kind}, wrapper: {wrap_struct.kind})")
            else:
                # Not found in either side — don't add to report
                pass

        _print_section(helper_result)
        sections.append(helper_result)

        # Detailed structure comparison for each class present in both
        for cls_name in helper_classes:
            if cls_name in EXCLUDED_FROM_PUPR:
                continue

            ref_struct = _find_class_structure(cls_name, ref_trees)
            wrap_struct = _find_class_structure(cls_name, wrap_trees)

            if ref_struct.kind == "not_found" or wrap_struct.kind == "not_found":
                continue

            detail_comp = _compare_class_structure(ref_struct, wrap_struct)
            detail_comp.section = f"Helper class detail — {cls_name} ({ref_struct.kind})"
            _print_section(detail_comp)
            sections.append(detail_comp)

        # --- Section 6: pandas_tools module ---
        pandas_result = ComparisonResult(section="pandas_tools module")
        if ref_pandas_tree:
            ref_pandas_funcs = _extract_top_level_functions(ref_pandas_tree)
            if wrap_pandas_tree:
                wrap_pandas_funcs = _extract_top_level_functions(wrap_pandas_tree)
                pandas_comp = _compare_module_functions(ref_pandas_funcs, wrap_pandas_funcs, "pandas_tools module")
                _print_section(pandas_comp)
                sections.append(pandas_comp)
            else:
                for fn in sorted(ref_pandas_funcs):
                    if fn in EXCLUDED_FROM_PUPR:
                        pandas_result.add(SYM_EXCLUDED, fn, "excluded")
                    else:
                        pandas_result.add(SYM_MISS, fn, "pandas_tools.py not found in wrapper")
                _print_section(pandas_result)
                sections.append(pandas_result)
        else:
            pandas_result.add(SYM_MISS, "(module)", "pandas_tools.py not found in reference")
            _print_section(pandas_result)
            sections.append(pandas_result)

        # --- Overall ---
        if not sections:
            print(f"\n  {RED}{BOLD}No comparison sections were produced. "
                  f"Reference files may have failed to download or parse.{RESET}")
            sys.exit(2)

        _print_overall_summary(sections)

        if parse_failures:
            print(f"  {RED}{BOLD}Warning: failed to parse/compare: "
                  f"{', '.join(parse_failures)}{RESET}\n")

    has_missing = any(s.missing_count > 0 for s in sections)
    sys.exit(0 if not has_missing and not parse_failures else 1)


if __name__ == "__main__":
    main()
