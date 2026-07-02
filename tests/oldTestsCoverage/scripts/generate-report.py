#!/usr/bin/env python3
"""Generate a self-contained HTML coverage report from the YAML mapping files.

Usage:
    python3 report.py                        # writes report.html next to the YAML files
    python3 report.py --out /tmp/report.html # custom output path
"""

import argparse
import html
import sys
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from utils.yaml_handler import load_mapping_yaml, get_all_test_entries, count_by_status, DRIVER_FILES

# ---------------------------------------------------------------------------
# Status metadata
# ---------------------------------------------------------------------------

STATUS_ORDER = ["mapped", "partial", "not-applicable", "unmapped"]

STATUS_META = {
    "mapped":          {"label": "Mapped",          "color": "#22c55e", "bg": "#f0fdf4", "border": "#bbf7d0"},
    "partial":         {"label": "Partial",         "color": "#f59e0b", "bg": "#fffbeb", "border": "#fde68a"},
    "not-applicable":  {"label": "N/A",             "color": "#94a3b8", "bg": "#f8fafc", "border": "#e2e8f0"},
    "unmapped":        {"label": "Unmapped",        "color": "#ef4444", "bg": "#fef2f2", "border": "#fecaca"},
}


def effective_status(entry: dict) -> str:
    status = entry.get("status")
    if status and status in STATUS_META:
        return status
    return "unmapped" if not entry.get("ud_tests") else "mapped"


# ---------------------------------------------------------------------------
# Data collection
# ---------------------------------------------------------------------------

def collect_driver_data(driver: str) -> dict:
    data = load_mapping_yaml(driver)
    counts = count_by_status(data)
    total = sum(counts.values())

    files: dict[str, list] = {}
    all_gaps = []

    tests_section = data.get("tests", {}) or {}
    for file_path, entries in tests_section.items():
        if not entries:
            continue
        file_tests = []
        for entry in entries:
            status = effective_status(entry)
            ud_tests = entry.get("ud_tests", [])
            gaps = entry.get("gaps", [])
            jira = entry.get("jira")
            notes = entry.get("notes")
            file_tests.append({
                "test_name": entry.get("test_name", "<unknown>"),
                "status": status,
                "ud_tests": [
                    (r if isinstance(r, str) else r.get("path", "")) for r in ud_tests
                ],
                "gaps": gaps,
                "notes": notes,
                "jira": jira,
            })
            if status == "partial" and gaps:
                all_gaps.append({
                    "file": file_path,
                    "test_name": entry.get("test_name", "<unknown>"),
                    "gaps": gaps,
                    "jira": jira,
                })
        files[file_path] = file_tests

    return {
        "driver": driver,
        "total": total,
        "counts": counts,
        "files": files,
        "gaps": all_gaps,
    }


# ---------------------------------------------------------------------------
# HTML helpers
# ---------------------------------------------------------------------------

def pct(count: int, total: int) -> float:
    return round(count / total * 100, 1) if total > 0 else 0.0


def progress_bar_html(counts: dict, total: int) -> str:
    segments = []
    for status in STATUS_ORDER:
        c = counts.get(status, 0)
        if c == 0:
            continue
        p = pct(c, total)
        color = STATUS_META[status]["color"]
        label = STATUS_META[status]["label"]
        segments.append(
            f'<div class="bar-seg" style="width:{p}%;background:{color}" '
            f'title="{label}: {c} ({p}%)"></div>'
        )
    return f'<div class="bar">{"".join(segments)}</div>'


def status_badge(status: str) -> str:
    m = STATUS_META.get(status, STATUS_META["unmapped"])
    return (
        f'<span class="badge" style="'
        f'color:{m["color"]};background:{m["bg"]};border-color:{m["border"]}">'
        f'{m["label"]}</span>'
    )


def jira_link(ticket: str | None) -> str:
    if not ticket:
        return ""
    url = f"https://snowflake.atlassian.net/browse/{ticket}"
    return f' <a class="jira" href="{url}" target="_blank">{html.escape(ticket)}</a>'


def ud_test_list_html(ud_tests: list[str]) -> str:
    if not ud_tests:
        return ""
    items = "".join(
        f'<li><code>{html.escape(t)}</code></li>' for t in ud_tests
    )
    return f'<ul class="ud-list">{items}</ul>'


def gaps_html(gaps: list[str]) -> str:
    if not gaps:
        return ""
    items = "".join(f'<li>{html.escape(g)}</li>' for g in gaps)
    return f'<ul class="gap-list">{items}</ul>'


# ---------------------------------------------------------------------------
# Section builders
# ---------------------------------------------------------------------------

def overview_section(all_data: list[dict]) -> str:
    rows = []
    for d in all_data:
        total = d["total"]
        counts = d["counts"]
        drv = d["driver"].upper()
        bar = progress_bar_html(counts, total)
        cells = "".join(
            f'<td class="num" style="color:{STATUS_META[s]["color"]}">'
            f'{counts.get(s, 0)}<span class="pct"> {pct(counts.get(s,0), total)}%</span></td>'
            for s in STATUS_ORDER
        )
        rows.append(
            f'<tr>'
            f'<td><button class="driver-link tab-jump" data-driver="{d["driver"]}">{drv}</button></td>'
            f'<td class="num">{total}</td>'
            f'{cells}'
            f'<td class="bar-cell">{bar}</td>'
            f'</tr>'
        )
    header_cells = "".join(
        f'<th style="color:{STATUS_META[s]["color"]}">{STATUS_META[s]["label"]}</th>'
        for s in STATUS_ORDER
    )
    return f"""
<section class="overview">
  <h2>Coverage Overview</h2>
  <table class="summary-table">
    <thead>
      <tr>
        <th>Driver</th><th>Total</th>{header_cells}<th>Progress</th>
      </tr>
    </thead>
    <tbody>{"".join(rows)}</tbody>
  </table>
</section>
"""


def driver_section(d: dict) -> str:
    driver = d["driver"]
    total = d["total"]
    counts = d["counts"]

    stat_pills = "".join(
        f'<span class="stat-pill filter-btn" data-driver="{driver}" data-status="{s}" '
        f'style="color:{STATUS_META[s]["color"]};'
        f'background:{STATUS_META[s]["bg"]};border-color:{STATUS_META[s]["border"]}">'
        f'{STATUS_META[s]["label"]} <strong>{counts.get(s,0)}</strong>'
        f'<span class="pct"> {pct(counts.get(s,0), total)}%</span></span>'
        for s in STATUS_ORDER
    )
    stat_pills = (
        f'<span class="stat-pill filter-btn filter-active" data-driver="{driver}" '
        f'data-status="all" style="color:#475569;background:#f1f5f9;border-color:#e2e8f0">'
        f'All <strong>{total}</strong></span>'
    ) + stat_pills

    # File accordion entries
    file_blocks = []
    for file_path, tests in d["files"].items():
        file_counts = {}
        for t in tests:
            file_counts[t["status"]] = file_counts.get(t["status"], 0) + 1

        dominant = max(file_counts, key=file_counts.get)
        dom_color = STATUS_META[dominant]["color"]
        mini_bar = progress_bar_html(file_counts, len(tests))
        fp_safe = html.escape(file_path)
        fp_id = f"{driver}_{file_path}".replace("/", "_").replace(".", "_")

        test_rows = []
        for t in tests:
            name = html.escape(t["test_name"])
            badge = status_badge(t["status"])
            jira = jira_link(t.get("jira"))
            ud_html = ud_test_list_html(t["ud_tests"])
            g_html = gaps_html(t["gaps"])
            notes = f'<p class="notes">{html.escape(t["notes"])}</p>' if t.get("notes") else ""
            detail = ud_html + g_html + notes
            detail_div = f'<div class="test-detail">{detail}</div>' if detail else ""
            test_rows.append(
                f'<div class="test-row status-{t["status"]}">'
                f'<div class="test-header">{badge} <span class="test-name">{name}</span>{jira}</div>'
                f'{detail_div}'
                f'</div>'
            )

        file_blocks.append(f"""
<details class="file-block">
  <summary>
    <span class="file-path" style="border-left:3px solid {dom_color}">{fp_safe}</span>
    <span class="file-count">{len(tests)} tests</span>
    {mini_bar}
  </summary>
  <div class="test-list">{"".join(test_rows)}</div>
</details>
""")

    # Gaps subsection
    gaps_content = ""
    if d["gaps"]:
        gap_items = []
        for g in d["gaps"]:
            jira = jira_link(g.get("jira"))
            gap_items.append(
                f'<div class="gap-item">'
                f'<div class="gap-origin"><code>{html.escape(g["file"])}</code>'
                f' — <strong>{html.escape(g["test_name"])}</strong>{jira}</div>'
                f'{gaps_html(g["gaps"])}'
                f'</div>'
            )
        gaps_content = f"""
<div class="gaps-panel">
  <h3>Coverage Gaps ({len(d["gaps"])} partial entries)</h3>
  {"".join(gap_items)}
</div>
"""

    return f"""
<section class="driver-section" id="{driver}">
  <h2>{driver.upper()}</h2>
  <div class="stat-pills">{stat_pills}</div>
  <div class="bar-large">{progress_bar_html(counts, total)}</div>
  <div class="file-list">{"".join(file_blocks)}</div>
  {gaps_content}
</section>
"""


# ---------------------------------------------------------------------------
# Full page
# ---------------------------------------------------------------------------

CSS = """
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
       font-size: 14px; color: #1e293b; background: #f8fafc; line-height: 1.5; }
a { color: #3b82f6; text-decoration: none; }
a:hover { text-decoration: underline; }

header { background: #0f172a; color: #f1f5f9; padding: 16px 32px 0;
         display: flex; flex-direction: column; gap: 10px; }
header .header-top { display: flex; align-items: baseline; gap: 16px; }
header h1 { font-size: 20px; font-weight: 600; }
header .ts { font-size: 12px; color: #94a3b8; }
.driver-tabs { display: flex; gap: 2px; }
.driver-tab { padding: 8px 20px; font-size: 13px; font-weight: 600; color: #94a3b8;
              cursor: pointer; border-radius: 6px 6px 0 0; border: none;
              background: transparent; transition: color .15s, background .15s; }
.driver-tab:hover { color: #f1f5f9; background: rgba(255,255,255,.08); }
.driver-tab.active { color: #f1f5f9; background: #f8fafc;
                     color: #0f172a; }

main { max-width: 1200px; margin: 0 auto; padding: 24px 32px; }

h2 { font-size: 16px; font-weight: 600; color: #0f172a; margin-bottom: 16px; padding-bottom: 8px;
     border-bottom: 1px solid #e2e8f0; }
h3 { font-size: 13px; font-weight: 600; color: #475569; margin-bottom: 12px; }

section { background: #fff; border: 1px solid #e2e8f0; border-radius: 8px;
          padding: 20px 24px; margin-bottom: 24px; }
.driver-section { display: none; }
.driver-section.active { display: block; }

/* Overview table */
.summary-table { width: 100%; border-collapse: collapse; }
.summary-table th { font-size: 11px; font-weight: 600; text-transform: uppercase;
                    letter-spacing: .05em; padding: 6px 10px; text-align: left;
                    color: #64748b; border-bottom: 1px solid #e2e8f0; }
.summary-table td { padding: 8px 10px; border-bottom: 1px solid #f1f5f9; vertical-align: middle; }
.summary-table tr:last-child td { border-bottom: none; }
.num { text-align: right; font-variant-numeric: tabular-nums; font-weight: 600; }
.pct { font-size: 11px; font-weight: 400; color: #94a3b8; }
.bar-cell { width: 220px; }
.driver-link { font-weight: 600; }
.tab-jump { background: none; border: none; padding: 0; cursor: pointer;
            color: #3b82f6; font-weight: 600; font-size: 14px; }
.tab-jump:hover { text-decoration: underline; }

/* Progress bars */
.bar, .bar-large { display: flex; height: 8px; border-radius: 4px; overflow: hidden;
                   background: #f1f5f9; }
.bar-large { height: 12px; margin-bottom: 16px; }
.bar-seg { height: 100%; transition: width .3s; }

/* Stat pills */
.stat-pills { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 16px; }
.stat-pill { font-size: 12px; padding: 3px 10px; border-radius: 12px; border: 1px solid;
             display: flex; gap: 5px; align-items: center; }
.filter-btn { cursor: pointer; transition: opacity .15s, box-shadow .15s; }
.filter-btn:hover { box-shadow: 0 0 0 2px currentColor; }
.filter-btn:not(.filter-active) { opacity: .45; }
.filter-active { opacity: 1 !important; box-shadow: 0 0 0 2px currentColor; }

/* File blocks */
.file-block { border: 1px solid #e2e8f0; border-radius: 6px; margin-bottom: 6px; overflow: hidden; }
.file-block > summary { display: flex; align-items: center; gap: 10px; padding: 8px 12px;
                        cursor: pointer; list-style: none; background: #f8fafc;
                        user-select: none; }
.file-block > summary::-webkit-details-marker { display: none; }
.file-block > summary:hover { background: #f1f5f9; }
.file-block[open] > summary { background: #f1f5f9; border-bottom: 1px solid #e2e8f0; }
.file-path { font-family: monospace; font-size: 12px; color: #334155;
             padding-left: 8px; flex: 1; }
.file-count { font-size: 11px; color: #94a3b8; white-space: nowrap; }
.file-block .bar { width: 120px; flex-shrink: 0; }

/* Test rows */
.test-list { padding: 8px 12px; display: flex; flex-direction: column; gap: 4px; }
.test-row { border: 1px solid transparent; border-radius: 4px; padding: 6px 8px; }
.test-row.status-unmapped { background: #fef2f2; border-color: #fecaca; }
.test-row.status-partial  { background: #fffbeb; border-color: #fde68a; }
.test-row.status-mapped   { background: #f0fdf4; border-color: #bbf7d0; }
.test-row.status-not-applicable { background: #f8fafc; border-color: #e2e8f0; }
.test-header { display: flex; align-items: baseline; gap: 8px; flex-wrap: wrap; }
.test-name { font-size: 13px; color: #1e293b; }
.badge { font-size: 10px; font-weight: 600; padding: 1px 7px; border-radius: 10px;
         border: 1px solid; white-space: nowrap; text-transform: uppercase; letter-spacing: .04em; }
.jira { font-size: 11px; }

/* Test detail */
.test-detail { margin-top: 6px; padding-top: 6px; border-top: 1px solid #e2e8f0; }
.ud-list { list-style: none; margin: 0 0 4px 0; display: flex; flex-direction: column; gap: 2px; }
.ud-list li { font-size: 11px; color: #475569; }
.ud-list code { font-size: 11px; background: #f1f5f9; padding: 1px 5px; border-radius: 3px; }
.gap-list { list-style: disc; padding-left: 18px; }
.gap-list li { font-size: 12px; color: #b45309; margin-bottom: 2px; }
.notes { font-size: 11px; color: #64748b; margin-top: 4px; font-style: italic; }

/* Gaps panel */
.gaps-panel { margin-top: 20px; padding-top: 16px; border-top: 2px solid #fde68a; }
.gap-item { background: #fffbeb; border: 1px solid #fde68a; border-radius: 6px;
            padding: 10px 14px; margin-bottom: 8px; }
.gap-origin { font-size: 12px; color: #475569; margin-bottom: 6px; }
.gap-origin code { font-size: 11px; color: #334155; }
"""

JS = """
function switchTab(driver) {
  document.querySelectorAll('.driver-tab').forEach(t => t.classList.remove('active'));
  document.querySelector(`.driver-tab[data-driver="${driver}"]`).classList.add('active');
  document.querySelectorAll('.driver-section').forEach(s => s.classList.remove('active'));
  document.getElementById(driver).classList.add('active');
}

// Driver tab switching
document.querySelectorAll('.driver-tab').forEach(tab => {
  tab.addEventListener('click', () => switchTab(tab.dataset.driver));
});

// Overview table row links
document.querySelectorAll('.tab-jump').forEach(btn => {
  btn.addEventListener('click', () => switchTab(btn.dataset.driver));
});

// Status filter within a driver section
document.querySelectorAll('.filter-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    const driver = btn.dataset.driver;
    const status = btn.dataset.status;
    const section = document.getElementById(driver);

    section.querySelectorAll('.filter-btn').forEach(b => b.classList.remove('filter-active'));
    btn.classList.add('filter-active');

    section.querySelectorAll('.test-row').forEach(row => {
      row.style.display = (status === 'all' || row.classList.contains('status-' + status))
        ? '' : 'none';
    });

    section.querySelectorAll('details.file-block').forEach(block => {
      const visible = [...block.querySelectorAll('.test-row')]
        .some(r => r.style.display !== 'none');
      block.style.display = visible ? '' : 'none';
      if (status !== 'all' && visible) block.open = true;
      if (status === 'all') block.open = false;
    });
  });
});
"""


def build_html(all_data: list[dict]) -> str:
    ts = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    overview = overview_section(all_data)

    tabs = (
        '<button class="driver-tab active" data-driver="overview">Overview</button>'
        + "".join(
            f'<button class="driver-tab" data-driver="{d["driver"]}">{d["driver"].upper()}</button>'
            for d in all_data
        )
    )

    driver_sections = "\n".join(driver_section(d) for d in all_data)
    overview_tab = f'<div id="overview" class="driver-section active">{overview}</div>'

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>UD Test Coverage Report</title>
<style>{CSS}</style>
</head>
<body>
<header>
  <div class="header-top">
    <h1>Universal Driver — Test Coverage Report</h1>
    <span class="ts">Generated {ts}</span>
  </div>
  <nav class="driver-tabs">{tabs}</nav>
</header>
<main>
  {overview_tab}
  {driver_sections}
</main>
<script>{JS}</script>
</body>
</html>
"""


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Generate HTML coverage report.")
    parser.add_argument(
        "--out",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "report.html",
        help="Output path (default: tests/oldTestsCoverage/report.html)",
    )
    args = parser.parse_args()

    all_data = [collect_driver_data(driver) for driver in DRIVER_FILES]
    html_content = build_html(all_data)

    args.out.write_text(html_content, encoding="utf-8")
    print(f"Report written to: {args.out}")

    # Print a quick summary to stdout
    for d in all_data:
        total = d["total"]
        counts = d["counts"]
        mapped = counts.get("mapped", 0)
        partial = counts.get("partial", 0)
        print(
            f"  {d['driver'].upper():<8} "
            f"mapped={mapped} ({pct(mapped, total)}%)  "
            f"partial={partial} ({pct(partial, total)}%)  "
            f"unmapped={counts.get('unmapped', 0)} ({pct(counts.get('unmapped',0), total)}%)  "
            f"total={total}"
        )


if __name__ == "__main__":
    main()
