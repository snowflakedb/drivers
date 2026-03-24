#!/usr/bin/env python3
from __future__ import annotations

import argparse
import csv
import json
import re
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

try:
    import javalang
except ImportError as exc:  # pragma: no cover
    raise SystemExit("Missing dependency 'javalang'. Install with: python3 -m pip install javalang") from exc


JDBC_INTERFACES = [
    "java.sql.Wrapper",
    "java.sql.Connection",
    "java.sql.Statement",
    "java.sql.PreparedStatement",
    "java.sql.CallableStatement",
    "java.sql.ResultSet",
    "java.sql.ResultSetMetaData",
    "java.sql.DatabaseMetaData",
    "java.sql.ParameterMetaData",
    "java.sql.Driver",
    "java.sql.Array",
    "java.sql.Blob",
    "java.sql.Clob",
    "java.sql.NClob",
    "java.sql.SQLXML",
    "java.sql.Struct",
    "java.sql.Ref",
    "java.sql.RowId",
    "java.sql.Savepoint",
    "java.sql.SQLData",
    "java.sql.SQLInput",
    "java.sql.SQLOutput",
    "javax.sql.CommonDataSource",
    "javax.sql.DataSource",
    "javax.sql.ConnectionPoolDataSource",
    "javax.sql.PooledConnection",
    "javax.sql.XADataSource",
    "javax.sql.XAConnection",
]
SUMMARY_ORDER = ["implemented", "unsupported_by_design", "not_implemented"]
CATEGORY_PRIORITY = {"implemented": 3, "unsupported_by_design": 2, "not_implemented": 1, "missing": 0}
METHOD_RE = re.compile(r"^\s*public\s+.*\s+([A-Za-z_]\w*)\(([^)]*)\).*$")
GENERIC_RE = re.compile(r"<[^<>]*>")
ARRAY_CTOR_METHOD_REF_RE = re.compile(r"\b([A-Za-z_][\w\.]*)\s*((?:\[\s*\])+)\s*::\s*new\b")
TARGET_SIMPLE_TO_FQCN = {x.split(".")[-1]: x for x in JDBC_INTERFACES}


@dataclass
class TypeInfo:
    package: str
    imports: dict[str, str]
    wildcard_imports: list[str]
    kind: str
    is_abstract: bool
    extends_names: list[str]
    implements_names: list[str]
    methods: dict[str, set[str]]


def norm_type(type_name: str) -> str:
    value = type_name.strip()
    while "<" in value and ">" in value:
        value = GENERIC_RE.sub("", value)
    value = value.replace("...", "[]").strip()
    if not value:
        return value
    suffix = ""
    while value.endswith("[]"):
        suffix += "[]"
        value = value[:-2].strip()
    if "." in value:
        value = value.split(".")[-1]
    return f"{value}{suffix}"


def norm_sig(name: str, params: list[str]) -> str:
    return f"{name}({','.join(norm_type(p) for p in params)})"


def sanitize(source: str) -> str:
    def repl(match: re.Match[str]) -> str:
        base = match.group(1)
        dims = match.group(2).count("[")
        return f"__n -> new {base}[__n]{'[]' * max(0, dims - 1)}"

    return ARRAY_CTOR_METHOD_REF_RE.sub(repl, source)


def parse_javap_interface(interface_name: str) -> tuple[list[str], set[str]]:
    out = subprocess.run(["javap", "-public", interface_name], check=True, text=True, capture_output=True).stdout
    parents, methods = [], set()
    for line in out.splitlines():
        s = line.strip()
        if " interface " in s and s.endswith("{"):
            if " extends " in s:
                parents = [x.strip() for x in s[:-1].split(" extends ", 1)[1].split(",") if x.strip()]
            continue
        if "(" not in s or not s.endswith(";") or s.startswith("public static final"):
            continue
        m = METHOD_RE.match(s)
        if not m:
            continue
        methods.add(norm_sig(m.group(1), [] if not m.group(2).strip() else [p.strip() for p in m.group(2).split(",")]))
    return parents, methods


def parse_ref_type(type_node) -> str:
    name, sub = type_node.name, getattr(type_node, "sub_type", None)
    while sub is not None:
        name, sub = f"{name}.{sub.name}", getattr(sub, "sub_type", None)
    return name


def parse_param_type(param) -> str:
    base = parse_ref_type(param.type) if hasattr(param.type, "name") else str(param.type)
    dims = len(getattr(param.type, "dimensions", []) or []) + len(getattr(param, "dimensions", []) or [])
    if getattr(param, "varargs", False):
        dims += 1
    return f"{base}{'[]' * dims}"


def throw_name(expr):
    if expr is None:
        return None
    if isinstance(expr, javalang.tree.ClassCreator):
        return norm_type(parse_ref_type(expr.type))
    if isinstance(expr, javalang.tree.Cast):
        return throw_name(expr.expression)
    if isinstance(expr, javalang.tree.TernaryExpression):
        return throw_name(expr.if_true) or throw_name(expr.if_false)
    return None


def thrown_types(method_decl) -> set[str]:
    res = set()
    for _, node in method_decl:
        if isinstance(node, javalang.tree.ThrowStatement):
            name = throw_name(node.expression)
            if name:
                res.add(name)
    return res


def collect_types(source_root: Path, package_prefixes: list[str]) -> dict[str, TypeInfo]:
    pkg_paths = [p.replace(".", "/") for p in package_prefixes]
    infos: dict[str, TypeInfo] = {}
    for java_file in sorted(source_root.rglob("*.java")):
        rel = java_file.relative_to(source_root).as_posix()
        if not any(rel.startswith(f"{p}/") for p in pkg_paths):
            continue
        try:
            tree = javalang.parse.parse(sanitize(java_file.read_text(encoding="utf-8")))
        except Exception as exc:
            print(f"Warning: skipping unparsable file: {java_file}: {exc}", file=sys.stderr)
            continue

        pkg = tree.package.name if tree.package else ""
        imports, wildcards = {}, []
        for imp in tree.imports:
            (wildcards if imp.wildcard else imports).__setitem__(imp.path.split(".")[-1] if not imp.wildcard else len(wildcards), imp.path)  # type: ignore[index]
        if wildcards and isinstance(wildcards, dict):  # safety for mypy/runtime weirdness
            wildcards = list(wildcards.values())

        for decl in tree.types:
            if not isinstance(decl, (javalang.tree.ClassDeclaration, javalang.tree.InterfaceDeclaration)):
                continue
            kind = "interface" if isinstance(decl, javalang.tree.InterfaceDeclaration) else "class"
            extends = [parse_ref_type(x) for x in (decl.extends or [])] if kind == "interface" else ([parse_ref_type(decl.extends)] if decl.extends else [])
            implements = [parse_ref_type(x) for x in (decl.implements or [])] if kind == "class" else []
            methods = {norm_sig(m.name, [parse_param_type(p) for p in m.parameters]): thrown_types(m) for m in decl.methods}
            fqcn = f"{pkg}.{decl.name}" if pkg else decl.name
            infos[fqcn] = TypeInfo(pkg, imports, list(wildcards), kind, "abstract" in (decl.modifiers or set()), extends, implements, methods)
    return infos


def simple_index(type_infos: dict[str, TypeInfo]) -> dict[str, set[str]]:
    out: dict[str, set[str]] = defaultdict(set)
    for fqcn in type_infos:
        out[fqcn.split(".")[-1]].add(fqcn)
    return out


def resolve(raw: str, owner: TypeInfo, type_infos: dict[str, TypeInfo], idx: dict[str, set[str]]) -> str:
    if not raw:
        return raw
    name = norm_type(raw)
    if "." in name:
        return name
    if name in owner.imports:
        return owner.imports[name]
    if name in TARGET_SIMPLE_TO_FQCN:
        return TARGET_SIMPLE_TO_FQCN[name]
    same_pkg = f"{owner.package}.{name}" if owner.package else name
    if same_pkg in type_infos:
        return same_pkg
    for wildcard in owner.wildcard_imports:
        candidate = f"{wildcard}.{name}"
        if candidate in type_infos or candidate in JDBC_INTERFACES:
            return candidate
    return next(iter(idx[name])) if name in idx and len(idx[name]) == 1 else name


def classify(driver_kind: str, thrown: set[str]) -> str:
    if driver_kind == "old":
        return "unsupported_by_design" if thrown & {"SnowflakeLoggedFeatureNotSupportedException", "SQLFeatureNotSupportedException"} else "implemented"
    if "NotImplementedException" in thrown:
        return "not_implemented"
    return "unsupported_by_design" if "SQLFeatureNotSupportedException" in thrown else "implemented"


def expand_interface_methods(iface: str, parents: dict[str, list[str]], declared: dict[str, set[str]], cache: dict[str, set[str]]) -> set[str]:
    if iface in cache:
        return cache[iface]
    res = set(declared.get(iface, set()))
    for p in parents.get(iface, []):
        res.update(expand_interface_methods(p, parents, declared, cache))
    cache[iface] = res
    return res


def build_source_api_surface(source_roots: list[Path], package_prefix: str) -> tuple[dict[str, list[str]], dict[str, set[str]]]:
    merged_parents: dict[str, list[str]] = {}
    merged_methods: dict[str, set[str]] = defaultdict(set)
    for root in source_roots:
        infos = collect_types(root, [package_prefix])
        idx = simple_index(infos)
        for fqcn, info in infos.items():
            if info.kind != "interface" or not fqcn.startswith(f"{package_prefix}."):
                continue
            parents = [resolve(p, info, infos, idx) for p in info.extends_names]
            merged_parents[fqcn] = sorted(set(merged_parents.get(fqcn, [])).union(parents))
            merged_methods[fqcn].update(info.methods.keys())
    return merged_parents, dict(merged_methods)


def analyze_driver(driver_name: str, source_root: Path, package_prefixes: list[str], driver_kind: str, target_interfaces: set[str], target_parents: dict[str, list[str]], target_declared: dict[str, set[str]]) -> dict:
    infos = collect_types(source_root, package_prefixes)
    idx = simple_index(infos)
    resolved = {fqcn: [resolve(n, info, infos, idx) for n in (info.extends_names + info.implements_names)] for fqcn, info in infos.items()}

    eff_cache, iface_cache, req_cache = {}, {}, {}

    def effective_methods(cls: str):
        if cls in eff_cache:
            return eff_cache[cls]
        out = {}
        for parent in resolved.get(cls, []):
            if parent in infos and infos[parent].kind == "class":
                out.update(effective_methods(parent))
        out.update({sig: classify(driver_kind, thrown) for sig, thrown in infos[cls].methods.items()})
        eff_cache[cls] = out
        return out

    def collect_target_from_interface(iface: str, seen: set[str]) -> set[str]:
        if iface in seen:
            return set()
        seen.add(iface)
        out = {iface}
        for p in target_parents.get(iface, []):
            out.update(collect_target_from_interface(p, seen))
        return out

    def implemented_ifaces(cls: str, seen: set[str]) -> set[str]:
        if cls in seen:
            return set()
        seen.add(cls)
        out = set()
        for p in resolved.get(cls, []):
            if p in target_interfaces:
                out.update(collect_target_from_interface(p, set()))
            elif p in infos:
                out.update(implemented_ifaces(p, seen))
        return out

    def ifaces_for_class(cls: str):
        if cls not in iface_cache:
            iface_cache[cls] = implemented_ifaces(cls, set())
        return iface_cache[cls]

    classes = [fqcn for fqcn, info in infos.items() if info.kind == "class" and not info.is_abstract and any(info.package.startswith(p) for p in package_prefixes)]
    method_categories: dict[tuple[str, str], str] = {}
    for cls in classes:
        ifaces = ifaces_for_class(cls)
        if not ifaces:
            continue
        eff = effective_methods(cls)
        for iface in ifaces:
            req = expand_interface_methods(iface, target_parents, target_declared, req_cache)
            for sig in req:
                k = (iface, sig)
                new, old = eff.get(sig, "missing"), method_categories.get(k, "missing")
                if CATEGORY_PRIORITY[new] > CATEGORY_PRIORITY[old]:
                    method_categories[k] = new

    totals = defaultdict(int)
    by_interface: dict[str, dict[str, int]] = {}
    for (iface, _), cat in method_categories.items():
        totals[cat] += 1
        by_interface.setdefault(iface, defaultdict(int))
        by_interface[iface][cat] += 1
    by_interface = {k: dict(sorted(v.items())) for k, v in sorted(by_interface.items())}

    return {
        "driver_name": driver_name,
        "source_root": str(source_root),
        "analyzed_package_prefixes": package_prefixes,
        "analyzed_concrete_classes": sorted(classes),
        "total_classes_analyzed": len(classes),
        "totals": dict(sorted(totals.items())),
        "by_interface": by_interface,
        "method_categories": {f"{i}::{s}": c for (i, s), c in sorted(method_categories.items())},
    }


def aggregates(method_categories: dict[str, str]) -> tuple[dict[str, int], dict[str, dict[str, int]]]:
    totals, by_interface = defaultdict(int), defaultdict(lambda: defaultdict(int))
    for key, cat in method_categories.items():
        iface, _ = key.split("::", 1)
        totals[cat] += 1
        by_interface[iface][cat] += 1
    return dict(sorted(totals.items())), {k: dict(sorted(v.items())) for k, v in sorted(by_interface.items())}


def reconcile(baseline_keys: list[str], old_raw: dict[str, str], new_raw: dict[str, str]) -> tuple[dict[str, str], dict[str, str]]:
    old_ifaces = {k.split("::", 1)[0] for k, v in old_raw.items() if v != "missing"}
    new_ifaces = {k.split("::", 1)[0] for k, v in new_raw.items() if v != "missing"}
    out_old, out_new = {}, {}
    for key in baseline_keys:
        iface, _ = key.split("::", 1)
        old_raw_cat = old_raw.get(key, "missing")
        old_cat = "implemented" if (iface in old_ifaces and old_raw_cat == "implemented") else "unsupported_by_design"

        if old_raw_cat == "missing":
            # Leadership view: baseline methods missing in old are treated as unsupported_by_design.
            new_cat = "unsupported_by_design"
        elif iface not in new_ifaces:
            new_cat = "not_implemented"
        elif new_raw.get(key) == "not_implemented":
            new_cat = "not_implemented"
        elif new_raw.get(key) == "unsupported_by_design":
            new_cat = "unsupported_by_design"
        else:
            new_cat = "implemented"
        out_old[key], out_new[key] = old_cat, new_cat
    return out_old, out_new


def apply_categories(report: dict, categories: dict[str, str]) -> None:
    totals, by_interface = aggregates(categories)
    report["method_categories"] = dict(sorted(categories.items()))
    report["totals"] = totals
    report["by_interface"] = by_interface


def write_csv_table(path: Path, old_categories: dict[str, str], new_categories: dict[str, str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["interface", "method_signature", "old_category", "new_category", "changed"])
        writer.writeheader()
        for key in sorted(set(old_categories.keys()).union(new_categories.keys())):
            iface, sig = key.split("::", 1)
            o, n = old_categories.get(key, "unsupported_by_design"), new_categories.get(key, "unsupported_by_design")
            writer.writerow({"interface": iface, "method_signature": sig, "old_category": o, "new_category": n, "changed": str(o != n).lower()})


def default_paths() -> tuple[Path, Path]:
    script_dir = Path(__file__).resolve().parent
    old = (script_dir.parent.parent.parent.parent / "snowflake-jdbc" / "src" / "main" / "java").resolve()
    new = (script_dir.parent.parent / "src" / "main" / "java").resolve()
    return old, new


def parse_args() -> argparse.Namespace:
    old_default, new_default = default_paths()
    parser = argparse.ArgumentParser(description="Generate API coverage summary for old/new drivers.")
    parser.add_argument("--old-root", type=Path, default=old_default)
    parser.add_argument("--new-root", type=Path, default=new_default)
    parser.add_argument("--include-package-prefix", action="append", default=["net.snowflake.client.internal", "net.snowflake.client.api"])
    parser.add_argument("--output-json", type=Path, default=(Path.cwd() / "build" / "reports" / "jdbc_api_coverage_report.json"))
    parser.add_argument("--output-table-csv", type=Path, default=(Path.cwd() / "build" / "reports" / "jdbc_api_method_comparison.csv"))
    return parser.parse_args()


def print_summary(report: dict) -> None:
    totals = report["totals"]
    leadership = leadership_buckets(report)
    done = leadership["done"]
    remaining = leadership["remaining"]
    print(f"\n{report['driver_name']}")
    print("-" * len(report["driver_name"]))
    print(f"Total API methods considered: {sum(totals.values())}")
    print(f"done: {done}")
    print(f"remaining: {remaining}")
    print(f"leadership_pct: done {leadership['done_pct']}% | remaining {leadership['remaining_pct']}%")
    for key in SUMMARY_ORDER:
        if key in totals:
            print(f"{key}: {totals[key]}")


def print_delta(old_report: dict, new_report: dict) -> None:
    print("\nDelta (new - old)")
    print("-----------------")
    old_t, new_t = defaultdict(int, old_report["totals"]), defaultdict(int, new_report["totals"])
    for cat in sorted(set(old_t).union(new_t)):
        print(f"{cat}: {new_t[cat] - old_t[cat]}")
    print(f"done: {(new_t['implemented'] + new_t['unsupported_by_design']) - (old_t['implemented'] + old_t['unsupported_by_design'])}")
    print(f"remaining: {new_t['not_implemented'] - old_t['not_implemented']}")


def leadership_buckets(report: dict) -> dict[str, float | int]:
    totals = defaultdict(int, report["totals"])
    done = totals["implemented"] + totals["unsupported_by_design"]
    remaining = totals["not_implemented"]
    total = done + remaining
    done_pct = round((done / total) * 100, 2) if total else 0.0
    remaining_pct = round((remaining / total) * 100, 2) if total else 0.0
    return {
        "done": done,
        "remaining": remaining,
        "done_pct": done_pct,
        "remaining_pct": remaining_pct,
    }


def main() -> int:
    args = parse_args()
    if not args.old_root.exists() or not args.new_root.exists():
        raise SystemExit(f"Invalid roots: old={args.old_root}, new={args.new_root}")

    jdbc_parents, jdbc_declared = {}, {}
    for iface in JDBC_INTERFACES:
        p, m = parse_javap_interface(iface)
        jdbc_parents[iface], jdbc_declared[iface] = p, m
    api_parents, api_declared = build_source_api_surface([args.old_root, args.new_root], "net.snowflake.client.api")

    parents = {k: list(v) for k, v in jdbc_parents.items()}
    declared = {k: set(v) for k, v in jdbc_declared.items()}
    for iface, p in api_parents.items():
        parents[iface] = sorted(set(parents.get(iface, [])).union(p))
    for iface, m in api_declared.items():
        declared.setdefault(iface, set()).update(m)
    target_interfaces = set(parents).union(declared)

    global TARGET_SIMPLE_TO_FQCN
    TARGET_SIMPLE_TO_FQCN = {x.split(".")[-1]: x for x in target_interfaces}

    old_report = analyze_driver("snowflake-jdbc (old)", args.old_root, args.include_package_prefix, "old", target_interfaces, parents, declared)
    new_report = analyze_driver("universal-driver/jdbc (new)", args.new_root, args.include_package_prefix, "new", target_interfaces, parents, declared)

    method_cache, baseline = {}, []
    for iface in sorted(target_interfaces):
        for sig in sorted(expand_interface_methods(iface, parents, declared, method_cache)):
            baseline.append(f"{iface}::{sig}")
    old_cat, new_cat = reconcile(sorted(set(baseline)), dict(old_report["method_categories"]), dict(new_report["method_categories"]))
    apply_categories(old_report, old_cat)
    apply_categories(new_report, new_cat)

    payload = {
        "assumptions": {
            "old_driver": {"unsupported_by_design_exceptions": ["SnowflakeLoggedFeatureNotSupportedException", "SQLFeatureNotSupportedException"]},
            "new_driver": {
                "unsupported_by_design_exceptions": ["SQLFeatureNotSupportedException"],
                "not_implemented_exceptions": ["NotImplementedException"],
                "alignment_rule": "Baseline = JDBC+net.snowflake.client.api; methods missing in old/new are reported as unsupported_by_design",
            },
        },
        "old": old_report,
        "new": new_report,
        "leadership_buckets": {
            "old": leadership_buckets(old_report),
            "new": leadership_buckets(new_report),
        },
    }
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    write_csv_table(args.output_table_csv, old_report["method_categories"], new_report["method_categories"])

    print_summary(old_report)
    print_summary(new_report)
    print_delta(old_report, new_report)
    print(f"\nWrote JSON report to: {args.output_json}")
    print(f"Wrote method comparison CSV table to: {args.output_table_csv}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
