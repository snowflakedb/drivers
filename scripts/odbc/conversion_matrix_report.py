#!/usr/bin/env python3
"""
Generates a color-coded HTML comparison report of ODBC conversion matrices
from two build directories (e.g. cmake-build vs cmake-build-reference).

Usage:
    python3 scripts/odbc/conversion_matrix_report.py [--out report.html]
           [--dir-a odbc_tests/cmake-build/tests/e2e/types/conversion_matrix]
           [--dir-b odbc_tests/cmake-build-reference/tests/e2e/types/conversion_matrix]
           [--label-a "Our Driver"] [--label-b "Reference (Snowflake)"]
"""

import argparse
import csv
import glob
import html
import os
import sys
from collections import OrderedDict
from pathlib import Path

WORKSPACE = Path(__file__).resolve().parents[2]

DEFAULT_DIR_A = WORKSPACE / "odbc_tests/cmake-build/tests/e2e/types/conversion_matrix"
DEFAULT_DIR_B = WORKSPACE / "odbc_tests/cmake-build-reference/tests/e2e/types/conversion_matrix"

C_TYPES_ORDER = [
    "SQL_C_CHAR", "SQL_C_WCHAR", "SQL_C_BIT",
    "SQL_C_TINYINT", "SQL_C_STINYINT", "SQL_C_UTINYINT",
    "SQL_C_SHORT", "SQL_C_SSHORT", "SQL_C_USHORT",
    "SQL_C_LONG", "SQL_C_SLONG", "SQL_C_ULONG",
    "SQL_C_SBIGINT", "SQL_C_UBIGINT",
    "SQL_C_FLOAT", "SQL_C_DOUBLE", "SQL_C_NUMERIC",
    "SQL_C_BINARY",
    "SQL_C_TYPE_DATE", "SQL_C_TYPE_TIME", "SQL_C_TYPE_TIMESTAMP",
    "SQL_C_INTERVAL_YEAR", "SQL_C_INTERVAL_MONTH",
    "SQL_C_INTERVAL_DAY", "SQL_C_INTERVAL_HOUR",
    "SQL_C_INTERVAL_MINUTE", "SQL_C_INTERVAL_SECOND",
    "SQL_C_INTERVAL_YEAR_TO_MONTH",
    "SQL_C_INTERVAL_DAY_TO_HOUR", "SQL_C_INTERVAL_DAY_TO_MINUTE",
    "SQL_C_INTERVAL_DAY_TO_SECOND",
    "SQL_C_INTERVAL_HOUR_TO_MINUTE", "SQL_C_INTERVAL_HOUR_TO_SECOND",
    "SQL_C_INTERVAL_MINUTE_TO_SECOND",
    "SQL_C_GUID",
]

C_TYPE_SHORT = {
    "SQL_C_CHAR": "CHAR", "SQL_C_WCHAR": "WCHAR", "SQL_C_BIT": "BIT",
    "SQL_C_TINYINT": "TINY", "SQL_C_STINYINT": "STINY", "SQL_C_UTINYINT": "UTINY",
    "SQL_C_SHORT": "SHORT", "SQL_C_SSHORT": "SSHORT", "SQL_C_USHORT": "USHORT",
    "SQL_C_LONG": "LONG", "SQL_C_SLONG": "SLONG", "SQL_C_ULONG": "ULONG",
    "SQL_C_SBIGINT": "SBIG", "SQL_C_UBIGINT": "UBIG",
    "SQL_C_FLOAT": "FLT", "SQL_C_DOUBLE": "DBL", "SQL_C_NUMERIC": "NUM",
    "SQL_C_BINARY": "BIN",
    "SQL_C_TYPE_DATE": "DATE", "SQL_C_TYPE_TIME": "TIME", "SQL_C_TYPE_TIMESTAMP": "TS",
    "SQL_C_INTERVAL_YEAR": "IV_Y", "SQL_C_INTERVAL_MONTH": "IV_MO",
    "SQL_C_INTERVAL_DAY": "IV_D", "SQL_C_INTERVAL_HOUR": "IV_H",
    "SQL_C_INTERVAL_MINUTE": "IV_MI", "SQL_C_INTERVAL_SECOND": "IV_S",
    "SQL_C_INTERVAL_YEAR_TO_MONTH": "IV_YM",
    "SQL_C_INTERVAL_DAY_TO_HOUR": "IV_DH", "SQL_C_INTERVAL_DAY_TO_MINUTE": "IV_DM",
    "SQL_C_INTERVAL_DAY_TO_SECOND": "IV_DS",
    "SQL_C_INTERVAL_HOUR_TO_MINUTE": "IV_HM", "SQL_C_INTERVAL_HOUR_TO_SECOND": "IV_HS",
    "SQL_C_INTERVAL_MINUTE_TO_SECOND": "IV_MS",
    "SQL_C_GUID": "GUID",
}


def classify(result_str):
    r = result_str.strip().upper()
    if r == "SQL_SUCCESS":
        return "Y"
    if r == "SQL_SUCCESS_WITH_INFO":
        return "W"
    return "E"


def parse_csv_dir(dirpath):
    """Parse all conversion_matrix_*.csv files in a directory.

    Returns two dicts:
        getdata:   { (sql_type, c_type): "Y"|"W"|"E" }
        bindparam: { (sql_type, c_type): "Y"|"W"|"E" }
    Also returns ordered lists of SQL types seen in each direction.
    """
    getdata = {}
    bindparam = {}
    getdata_sql_types = OrderedDict()
    bindparam_sql_types = OrderedDict()

    pattern = os.path.join(dirpath, "conversion_matrix_*.csv")
    for fpath in sorted(glob.glob(pattern)):
        with open(fpath, newline="", encoding="utf-8") as f:
            reader = csv.DictReader(f)
            for row in reader:
                direction = row.get("direction", "").strip()
                sql_type = row.get("sql_type", "").strip()
                c_type = row.get("c_type", "").strip()
                result = row.get("result", "").strip()
                if not direction or not sql_type or not c_type or not result:
                    continue
                code = classify(result)
                key = (sql_type, c_type)
                if direction == "getdata":
                    getdata[key] = code
                    getdata_sql_types[sql_type] = True
                elif direction == "bindparam":
                    bindparam[key] = code
                    bindparam_sql_types[sql_type] = True

    return getdata, bindparam, list(getdata_sql_types.keys()), list(bindparam_sql_types.keys())


def shorten_sql(name):
    for prefix in ("SQL_C_INTERVAL_", "SQL_C_TYPE_", "SQL_C_", "SQL_INTERVAL_", "SQL_TYPE_", "SQL_"):
        if name.startswith(prefix):
            return name[len(prefix):]
    return name


def cell_class(code):
    return {"Y": "Y", "W": "W", "E": "E"}.get(code, "M")


def diff_class(a, b):
    if a == b:
        return "same"
    if a is None or b is None:
        return "missing"
    if a in ("Y", "W") and b == "E":
        return "better"
    if a == "E" and b in ("Y", "W"):
        return "worse"
    if a == "Y" and b == "W":
        return "better"
    if a == "W" and b == "Y":
        return "worse"
    return "diff"


def render_matrix_table(data, sql_types, c_types, table_id=""):
    rows = []
    rows.append('<div class="table-wrap">')
    rows.append(f'<table id="{table_id}">')
    rows.append("<thead><tr>")
    rows.append('<th class="corner">SQL / C &rarr;</th>')
    for ct in c_types:
        rows.append(f"<th>{html.escape(C_TYPE_SHORT.get(ct, shorten_sql(ct)))}</th>")
    rows.append("</tr></thead><tbody>")
    for st in sql_types:
        rows.append("<tr>")
        rows.append(f'<td class="row-label">{html.escape(shorten_sql(st))}</td>')
        for ct in c_types:
            code = data.get((st, ct))
            if code is None:
                rows.append('<td class="M">-</td>')
            else:
                rows.append(f'<td class="{cell_class(code)}">{code}</td>')
        rows.append("</tr>")
    rows.append("</tbody></table></div>")
    return "\n".join(rows)


def render_diff_table(data_a, data_b, sql_types, c_types, label_a, label_b, table_id=""):
    rows = []
    rows.append('<div class="table-wrap">')
    rows.append(f'<table id="{table_id}" class="diff-table">')
    rows.append("<thead><tr>")
    rows.append('<th class="corner">SQL / C &rarr;</th>')
    for ct in c_types:
        rows.append(f"<th>{html.escape(C_TYPE_SHORT.get(ct, shorten_sql(ct)))}</th>")
    rows.append("</tr></thead><tbody>")

    counts = {"same": 0, "better": 0, "worse": 0, "diff": 0, "missing": 0}
    for st in sql_types:
        rows.append("<tr>")
        rows.append(f'<td class="row-label">{html.escape(shorten_sql(st))}</td>')
        for ct in c_types:
            a = data_a.get((st, ct))
            b = data_b.get((st, ct))
            dc = diff_class(a, b)
            counts[dc] += 1
            if a is None and b is None:
                rows.append('<td class="M">-</td>')
            elif a == b:
                rows.append(f'<td class="same">{a}</td>')
            else:
                a_str = a if a else "?"
                b_str = b if b else "?"
                rows.append(f'<td class="{dc}" title="{label_a}={a_str} {label_b}={b_str}">{a_str}/{b_str}</td>')
        rows.append("</tr>")
    rows.append("</tbody></table></div>")
    return "\n".join(rows), counts


def build_report(dir_a, dir_b, label_a, label_b):
    gd_a, bp_a, gd_st_a, bp_st_a = parse_csv_dir(dir_a)
    gd_b, bp_b, gd_st_b, bp_st_b = parse_csv_dir(dir_b)

    gd_sql = list(OrderedDict.fromkeys(gd_st_a + gd_st_b))
    bp_sql = list(OrderedDict.fromkeys(bp_st_a + bp_st_b))
    c_types = C_TYPES_ORDER

    gd_table_a = render_matrix_table(gd_a, gd_sql, c_types, "gd-a")
    gd_table_b = render_matrix_table(gd_b, gd_sql, c_types, "gd-b")
    bp_table_a = render_matrix_table(bp_a, bp_sql, c_types, "bp-a")
    bp_table_b = render_matrix_table(bp_b, bp_sql, c_types, "bp-b")

    gd_diff, gd_counts = render_diff_table(gd_a, gd_b, gd_sql, c_types, label_a, label_b, "gd-diff")
    bp_diff, bp_counts = render_diff_table(bp_a, bp_b, bp_sql, c_types, label_a, label_b, "bp-diff")

    def count_supported(data):
        y = sum(1 for v in data.values() if v == "Y")
        w = sum(1 for v in data.values() if v == "W")
        e = sum(1 for v in data.values() if v == "E")
        return y, w, e

    def calc_impl_pct(data_a, data_b):
        """Percentage of ref-supported conversions (Y|W) that A also supports (Y|W)."""
        ref_supported = {k for k, v in data_b.items() if v in ("Y", "W")}
        if not ref_supported:
            return 0, 0, 0.0
        implemented = sum(1 for k in ref_supported if data_a.get(k) in ("Y", "W"))
        return implemented, len(ref_supported), (implemented / len(ref_supported)) * 100

    gy_a, gw_a, ge_a = count_supported(gd_a)
    gy_b, gw_b, ge_b = count_supported(gd_b)
    by_a, bw_a, be_a = count_supported(bp_a)
    by_b, bw_b, be_b = count_supported(bp_b)

    gd_impl, gd_ref_total, gd_impl_pct = calc_impl_pct(gd_a, gd_b)
    bp_impl, bp_ref_total, bp_impl_pct = calc_impl_pct(bp_a, bp_b)
    all_a = {**{("gd", *k): v for k, v in gd_a.items()}, **{("bp", *k): v for k, v in bp_a.items()}}
    all_b = {**{("gd", *k): v for k, v in gd_b.items()}, **{("bp", *k): v for k, v in bp_b.items()}}
    total_impl, total_ref, total_impl_pct = calc_impl_pct(all_a, all_b)

    return HTML_TEMPLATE.format(
        label_a=html.escape(label_a),
        label_b=html.escape(label_b),
        gd_table_a=gd_table_a,
        gd_table_b=gd_table_b,
        bp_table_a=bp_table_a,
        bp_table_b=bp_table_b,
        gd_diff=gd_diff,
        bp_diff=bp_diff,
        gd_same=gd_counts["same"], gd_better=gd_counts["better"],
        gd_worse=gd_counts["worse"], gd_other=gd_counts["diff"] + gd_counts["missing"],
        bp_same=bp_counts["same"], bp_better=bp_counts["better"],
        bp_worse=bp_counts["worse"], bp_other=bp_counts["diff"] + bp_counts["missing"],
        gy_a=gy_a, gw_a=gw_a, ge_a=ge_a, gt_a=gy_a+gw_a+ge_a,
        gy_b=gy_b, gw_b=gw_b, ge_b=ge_b, gt_b=gy_b+gw_b+ge_b,
        by_a=by_a, bw_a=bw_a, be_a=be_a, bt_a=by_a+bw_a+be_a,
        by_b=by_b, bw_b=bw_b, be_b=be_b, bt_b=by_b+bw_b+be_b,
        gd_impl=gd_impl, gd_ref_total=gd_ref_total, gd_impl_pct=f"{gd_impl_pct:.1f}",
        bp_impl=bp_impl, bp_ref_total=bp_ref_total, bp_impl_pct=f"{bp_impl_pct:.1f}",
        total_impl=total_impl, total_ref=total_ref, total_impl_pct=f"{total_impl_pct:.1f}",
    )


HTML_TEMPLATE = """\
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>ODBC Conversion Matrix — Comparison Report</title>
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: 'Segoe UI', system-ui, -apple-system, sans-serif; background: #0f1117; color: #e0e0e0; padding: 24px 32px; }}
  h1 {{ font-size: 1.5rem; color: #fff; margin-bottom: 4px; }}
  h2 {{ font-size: 1.15rem; margin: 30px 0 10px; color: #b0b8c8; border-bottom: 1px solid #2a2d38; padding-bottom: 5px; }}
  h3 {{ font-size: 0.95rem; margin: 18px 0 8px; color: #8aa0c0; }}
  .subtitle {{ color: #7a8599; font-size: 0.85rem; margin-bottom: 20px; }}
  .legends {{ display: flex; flex-wrap: wrap; gap: 28px; margin-bottom: 16px; }}
  .legend {{ display: flex; flex-wrap: wrap; gap: 12px; font-size: 0.78rem; }}
  .legend-title {{ font-weight: 600; color: #9aa3b8; margin-bottom: 2px; width: 100%; }}
  .legend-item {{ display: flex; align-items: center; gap: 5px; }}
  .legend-box {{ width: 16px; height: 16px; border-radius: 3px; border: 1px solid rgba(255,255,255,0.08); }}

  .table-wrap {{ overflow-x: auto; margin-bottom: 28px; }}
  table {{ border-collapse: collapse; font-size: 0.7rem; }}
  th, td {{ padding: 3px 5px; text-align: center; border: 1px solid #23262f; white-space: nowrap; min-width: 30px; }}
  th {{ background: #1a1d27; color: #9aa3b8; font-weight: 600; position: sticky; top: 0; z-index: 2; }}
  th.corner {{ position: sticky; left: 0; top: 0; z-index: 4; background: #1a1d27; text-align: left; }}
  td.row-label {{ text-align: left; font-weight: 600; color: #cfd6e4; background: #14161e; position: sticky; left: 0; z-index: 1; min-width: 90px; }}

  .Y  {{ background: #16a34a; color: #fff; font-weight: 700; }}
  .W  {{ background: #ca8a04; color: #fff; font-weight: 700; }}
  .E  {{ background: #dc2626; color: rgba(255,255,255,0.55); }}
  .M  {{ background: #1e1e2e; color: #555; }}

  .diff-table .same  {{ background: #1a2633; color: #6b8aad; }}
  .diff-table .better {{ background: #064e3b; color: #6ee7b7; font-weight: 700; }}
  .diff-table .worse  {{ background: #7f1d1d; color: #fca5a5; font-weight: 700; }}
  .diff-table .diff   {{ background: #713f12; color: #fde68a; font-weight: 700; }}
  .diff-table .missing {{ background: #2d2040; color: #c4b5fd; }}

  td:hover {{ outline: 2px solid #fff; outline-offset: -2px; cursor: default; }}

  .stats {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 10px; margin: 12px 0 24px; }}
  .stat {{ background: #1a1d27; border-radius: 8px; padding: 12px 14px; border: 1px solid #2a2d38; }}
  .stat .label {{ color: #7a8599; font-size: 0.75rem; }}
  .stat .value {{ font-size: 1.2rem; font-weight: 700; margin-top: 2px; }}

  nav {{ position: sticky; top: 0; z-index: 10; background: #0f1117ee; backdrop-filter: blur(8px);
         padding: 10px 0; margin-bottom: 12px; border-bottom: 1px solid #2a2d38; }}
  nav a {{ color: #60a5fa; text-decoration: none; font-size: 0.82rem; margin-right: 18px; }}
  nav a:hover {{ text-decoration: underline; }}

  .impl-section {{ margin-bottom: 28px; }}
  .impl-cards {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 14px; margin-top: 10px; }}
  .impl-card {{ background: #1a1d27; border-radius: 10px; padding: 18px 20px; border: 1px solid #2a2d38; }}
  .impl-card .title {{ color: #9aa3b8; font-size: 0.8rem; margin-bottom: 8px; font-weight: 600; }}
  .impl-card .pct {{ font-size: 2rem; font-weight: 800; }}
  .impl-card .detail {{ color: #7a8599; font-size: 0.78rem; margin-top: 4px; }}
  .impl-card .bar-bg {{ background: #2a2d38; border-radius: 4px; height: 8px; margin-top: 10px; overflow: hidden; }}
  .impl-card .bar-fg {{ height: 100%; border-radius: 4px; transition: width 0.4s; }}
</style>
</head>
<body>

<h1>ODBC Conversion Matrix &mdash; Comparison Report</h1>
<p class="subtitle">{label_a} vs {label_b}</p>

<nav>
  <a href="#sec-impl">Coverage</a>
  <a href="#sec-diff-gd">Diff: GetData</a>
  <a href="#sec-diff-bp">Diff: BindParam</a>
  <a href="#sec-a-gd">{label_a}: GetData</a>
  <a href="#sec-a-bp">{label_a}: BindParam</a>
  <a href="#sec-b-gd">{label_b}: GetData</a>
  <a href="#sec-b-bp">{label_b}: BindParam</a>
</nav>

<div class="legends">
  <div class="legend">
    <div class="legend-title">Individual matrices</div>
    <div class="legend-item"><div class="legend-box" style="background:#16a34a"></div> Y = SQL_SUCCESS</div>
    <div class="legend-item"><div class="legend-box" style="background:#ca8a04"></div> W = SUCCESS_WITH_INFO</div>
    <div class="legend-item"><div class="legend-box" style="background:#dc2626"></div> E = SQL_ERROR</div>
  </div>
  <div class="legend">
    <div class="legend-title">Comparison (A / B)</div>
    <div class="legend-item"><div class="legend-box" style="background:#1a2633"></div> Same result</div>
    <div class="legend-item"><div class="legend-box" style="background:#064e3b"></div> A better than B</div>
    <div class="legend-item"><div class="legend-box" style="background:#7f1d1d"></div> A worse than B</div>
    <div class="legend-item"><div class="legend-box" style="background:#713f12"></div> Different (other)</div>
  </div>
</div>

<!-- ==================== IMPLEMENTATION COVERAGE ==================== -->
<div class="impl-section">
<h2 id="sec-impl">Implementation Coverage &mdash; {label_a} vs {label_b}</h2>
<p style="color:#7a8599;font-size:0.8rem;margin-bottom:4px">
  Of the conversions the reference driver supports (Y or W), how many does our driver also support (Y or W)?
</p>
<div class="impl-cards">
  <div class="impl-card">
    <div class="title">Overall</div>
    <div class="pct" style="color:#60a5fa">{total_impl_pct}%</div>
    <div class="detail">{total_impl} / {total_ref} conversions implemented</div>
    <div class="bar-bg"><div class="bar-fg" style="width:{total_impl_pct}%; background:#60a5fa"></div></div>
  </div>
  <div class="impl-card">
    <div class="title">SQLGetData</div>
    <div class="pct" style="color:#34d399">{gd_impl_pct}%</div>
    <div class="detail">{gd_impl} / {gd_ref_total} conversions implemented</div>
    <div class="bar-bg"><div class="bar-fg" style="width:{gd_impl_pct}%; background:#34d399"></div></div>
  </div>
  <div class="impl-card">
    <div class="title">SQLBindParameter</div>
    <div class="pct" style="color:#f472b6">{bp_impl_pct}%</div>
    <div class="detail">{bp_impl} / {bp_ref_total} conversions implemented</div>
    <div class="bar-bg"><div class="bar-fg" style="width:{bp_impl_pct}%; background:#f472b6"></div></div>
  </div>
</div>
</div>

<!-- ==================== DIFF: GETDATA ==================== -->
<h2 id="sec-diff-gd">Comparison: SQLGetData &mdash; {label_a} vs {label_b}</h2>
<p style="color:#7a8599;font-size:0.8rem;margin-bottom:8px">
  Cells show A/B. Hover for details.
</p>
<div class="stats">
  <div class="stat"><div class="label">Identical</div><div class="value">{gd_same}</div></div>
  <div class="stat"><div class="label" style="color:#6ee7b7">A better</div><div class="value" style="color:#6ee7b7">{gd_better}</div></div>
  <div class="stat"><div class="label" style="color:#fca5a5">A worse</div><div class="value" style="color:#fca5a5">{gd_worse}</div></div>
  <div class="stat"><div class="label" style="color:#fde68a">Other diff</div><div class="value" style="color:#fde68a">{gd_other}</div></div>
</div>
{gd_diff}

<!-- ==================== DIFF: BINDPARAM ==================== -->
<h2 id="sec-diff-bp">Comparison: SQLBindParameter &mdash; {label_a} vs {label_b}</h2>
<p style="color:#7a8599;font-size:0.8rem;margin-bottom:8px">
  Cells show A/B. Hover for details.
</p>
<div class="stats">
  <div class="stat"><div class="label">Identical</div><div class="value">{bp_same}</div></div>
  <div class="stat"><div class="label" style="color:#6ee7b7">A better</div><div class="value" style="color:#6ee7b7">{bp_better}</div></div>
  <div class="stat"><div class="label" style="color:#fca5a5">A worse</div><div class="value" style="color:#fca5a5">{bp_worse}</div></div>
  <div class="stat"><div class="label" style="color:#fde68a">Other diff</div><div class="value" style="color:#fde68a">{bp_other}</div></div>
</div>
{bp_diff}

<!-- ==================== A: GETDATA ==================== -->
<h2 id="sec-a-gd">{label_a} &mdash; SQLGetData</h2>
<div class="stats">
  <div class="stat"><div class="label">SUCCESS</div><div class="value" style="color:#16a34a">{gy_a}</div></div>
  <div class="stat"><div class="label">WITH_INFO</div><div class="value" style="color:#ca8a04">{gw_a}</div></div>
  <div class="stat"><div class="label">ERROR</div><div class="value" style="color:#dc2626">{ge_a}</div></div>
  <div class="stat"><div class="label">Total</div><div class="value">{gt_a}</div></div>
</div>
{gd_table_a}

<!-- ==================== A: BINDPARAM ==================== -->
<h2 id="sec-a-bp">{label_a} &mdash; SQLBindParameter</h2>
<div class="stats">
  <div class="stat"><div class="label">SUCCESS</div><div class="value" style="color:#16a34a">{by_a}</div></div>
  <div class="stat"><div class="label">WITH_INFO</div><div class="value" style="color:#ca8a04">{bw_a}</div></div>
  <div class="stat"><div class="label">ERROR</div><div class="value" style="color:#dc2626">{be_a}</div></div>
  <div class="stat"><div class="label">Total</div><div class="value">{bt_a}</div></div>
</div>
{bp_table_a}

<!-- ==================== B: GETDATA ==================== -->
<h2 id="sec-b-gd">{label_b} &mdash; SQLGetData</h2>
<div class="stats">
  <div class="stat"><div class="label">SUCCESS</div><div class="value" style="color:#16a34a">{gy_b}</div></div>
  <div class="stat"><div class="label">WITH_INFO</div><div class="value" style="color:#ca8a04">{gw_b}</div></div>
  <div class="stat"><div class="label">ERROR</div><div class="value" style="color:#dc2626">{ge_b}</div></div>
  <div class="stat"><div class="label">Total</div><div class="value">{gt_b}</div></div>
</div>
{gd_table_b}

<!-- ==================== B: BINDPARAM ==================== -->
<h2 id="sec-b-bp">{label_b} &mdash; SQLBindParameter</h2>
<div class="stats">
  <div class="stat"><div class="label">SUCCESS</div><div class="value" style="color:#16a34a">{by_b}</div></div>
  <div class="stat"><div class="label">WITH_INFO</div><div class="value" style="color:#ca8a04">{bw_b}</div></div>
  <div class="stat"><div class="label">ERROR</div><div class="value" style="color:#dc2626">{be_b}</div></div>
  <div class="stat"><div class="label">Total</div><div class="value">{bt_b}</div></div>
</div>
{bp_table_b}

</body>
</html>
"""


def main():
    parser = argparse.ArgumentParser(description="ODBC Conversion Matrix comparison report")
    parser.add_argument("--dir-a", default=str(DEFAULT_DIR_A),
                        help="Path to first build's conversion_matrix CSV dir")
    parser.add_argument("--dir-b", default=str(DEFAULT_DIR_B),
                        help="Path to second build's conversion_matrix CSV dir")
    parser.add_argument("--label-a", default="Our Driver",
                        help="Display label for first build")
    parser.add_argument("--label-b", default="Reference (Snowflake)",
                        help="Display label for second build")
    parser.add_argument("--out", default=str(WORKSPACE / "conversion_matrix_report.html"),
                        help="Output HTML file path")
    args = parser.parse_args()

    for d, name in [(args.dir_a, args.label_a), (args.dir_b, args.label_b)]:
        if not os.path.isdir(d):
            print(f"ERROR: directory not found for {name}: {d}", file=sys.stderr)
            sys.exit(1)

    report = build_report(args.dir_a, args.dir_b, args.label_a, args.label_b)

    with open(args.out, "w", encoding="utf-8") as f:
        f.write(report)

    print(f"Report written to {args.out}")
    print(f"  {args.label_a}: {args.dir_a}")
    print(f"  {args.label_b}: {args.dir_b}")


if __name__ == "__main__":
    main()
