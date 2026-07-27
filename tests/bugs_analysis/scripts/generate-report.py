#!/usr/bin/env python3
"""Generate a self-contained HTML bugs-analysis report from the YAML catalog files.

Usage:
    python3 generate-report.py                        # writes report.html next to the YAML files
    python3 generate-report.py --out /tmp/report.html # custom output path
"""

import argparse
import html
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

import yaml

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

SCRIPT_DIR = Path(__file__).resolve().parent
DATA_DIR = SCRIPT_DIR.parent

DRIVER_FILES = {
    "odbc":   DATA_DIR / "odbc.yaml",
    "python": DATA_DIR / "python.yaml",
}

# ---------------------------------------------------------------------------
# Status metadata
# ---------------------------------------------------------------------------

STATUS_ORDER = ["regression", "gap", "not_implemented_in_ud", "covered", "n/a", "not_analyzed"]

STATUS_META = {
    "regression":           {"label": "Regression",        "color": "#ef4444", "bg": "#fef2f2", "border": "#fecaca"},
    "gap":                  {"label": "Gap",               "color": "#f59e0b", "bg": "#fffbeb", "border": "#fde68a"},
    "not_implemented_in_ud":{"label": "Not in UD",         "color": "#8b5cf6", "bg": "#faf5ff", "border": "#ddd6fe"},
    "covered":              {"label": "Covered",           "color": "#22c55e", "bg": "#f0fdf4", "border": "#bbf7d0"},
    "n/a":                  {"label": "N/A",               "color": "#94a3b8", "bg": "#f8fafc", "border": "#e2e8f0"},
    "not_analyzed":         {"label": "Not analyzed",      "color": "#64748b", "bg": "#f1f5f9", "border": "#cbd5e1"},
}

PRIORITY_ORDER = {"Blocker": 0, "Critical": 1, "High": 2, "Medium": 3, "Low": 4}

# ---------------------------------------------------------------------------
# Data loading
# ---------------------------------------------------------------------------

def load_driver(driver: str) -> list[dict]:
    path = DRIVER_FILES[driver]
    with open(path) as f:
        data = yaml.safe_load(f)
    return data.get("bugs", [])


def collect_data(driver: str) -> dict:
    bugs = load_driver(driver)
    counts = Counter(b.get("analysis_status", "not_analyzed") for b in bugs)
    return {"driver": driver, "bugs": bugs, "counts": counts, "total": len(bugs)}

# ---------------------------------------------------------------------------
# HTML helpers
# ---------------------------------------------------------------------------

def pct(count: int, total: int) -> float:
    return round(count / total * 100, 1) if total > 0 else 0.0


def progress_bar(counts: dict, total: int, height: str = "8px") -> str:
    segs = []
    for s in STATUS_ORDER:
        c = counts.get(s, 0)
        if c == 0:
            continue
        p = pct(c, total)
        color = STATUS_META[s]["color"]
        label = STATUS_META[s]["label"]
        segs.append(
            f'<div class="bar-seg" style="width:{p}%;background:{color}" '
            f'title="{label}: {c} ({p}%)"></div>'
        )
    return f'<div class="bar" style="height:{height}">{"".join(segs)}</div>'


def status_badge(status: str) -> str:
    m = STATUS_META.get(status, STATUS_META["not_analyzed"])
    return (
        f'<span class="badge" style="color:{m["color"]};background:{m["bg"]};'
        f'border-color:{m["border"]}">{m["label"]}</span>'
    )


def jira_link(key: str) -> str:
    url = f"https://snowflakecomputing.atlassian.net/browse/{key}"
    return f'<a href="{url}" target="_blank">{html.escape(key)}</a>'


def priority_badge(priority: str) -> str:
    colors = {
        "Blocker":  ("#7f1d1d", "#fef2f2", "#fecaca"),
        "Critical": ("#991b1b", "#fef2f2", "#fca5a5"),
        "High":     ("#92400e", "#fffbeb", "#fcd34d"),
        "Medium":   ("#1e40af", "#eff6ff", "#bfdbfe"),
        "Low":      ("#374151", "#f9fafb", "#d1d5db"),
    }
    fg, bg, bd = colors.get(priority, ("#374151", "#f9fafb", "#d1d5db"))
    return (
        f'<span class="badge" style="color:{fg};background:{bg};border-color:{bd}">'
        f'{html.escape(priority)}</span>'
    )

# ---------------------------------------------------------------------------
# Section builders
# ---------------------------------------------------------------------------

def overview_section(all_data: list[dict]) -> str:
    header_cells = "".join(
        f'<th style="color:{STATUS_META[s]["color"]}">{STATUS_META[s]["label"]}</th>'
        for s in STATUS_ORDER
    )
    rows = []
    for d in all_data:
        total = d["total"]
        counts = d["counts"]
        drv = d["driver"].upper()
        bar = progress_bar(counts, total)
        cells = "".join(
            f'<td class="num" style="color:{STATUS_META[s]["color"]}">'
            f'{counts.get(s, 0)}'
            f'<span class="pct"> {pct(counts.get(s, 0), total)}%</span></td>'
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
    return f"""
<section class="overview">
  <h2>Analysis Overview</h2>
  <table class="summary-table">
    <thead>
      <tr><th>Driver</th><th>Total</th>{header_cells}<th>Progress</th></tr>
    </thead>
    <tbody>{"".join(rows)}</tbody>
  </table>
</section>
"""


def driver_section(d: dict) -> str:
    driver = d["driver"]
    total = d["total"]
    counts = d["counts"]
    bugs = d["bugs"]

    # Stat pills
    all_pill = (
        f'<span class="stat-pill filter-btn filter-active" data-driver="{driver}" '
        f'data-status="all" style="color:#475569;background:#f1f5f9;border-color:#e2e8f0">'
        f'All <strong>{total}</strong></span>'
    )
    pills = all_pill + "".join(
        f'<span class="stat-pill filter-btn" data-driver="{driver}" data-status="{s}" '
        f'style="color:{STATUS_META[s]["color"]};background:{STATUS_META[s]["bg"]};'
        f'border-color:{STATUS_META[s]["border"]}">'
        f'{STATUS_META[s]["label"]} <strong>{counts.get(s, 0)}</strong>'
        f'<span class="pct"> {pct(counts.get(s, 0), total)}%</span></span>'
        for s in STATUS_ORDER if counts.get(s, 0) > 0
    )

    large_bar = progress_bar(counts, total, height="12px")

    # Bug rows — include data-* sort keys on each row
    bug_rows = []
    for b in bugs:
        status = b.get("analysis_status", "not_analyzed")
        key = b.get("key", "")
        summary = html.escape(b.get("summary", ""))
        commit_date = b.get("commit_date", "")
        priority = b.get("priority", "Medium")
        # Numeric sort keys for priority and status
        pri_idx = PRIORITY_ORDER.get(priority, 99)
        sta_idx = STATUS_ORDER.index(status) if status in STATUS_ORDER else 99
        # Ticket number for numeric sort (SNOW-12345 → 12345)
        ticket_num = int(key.split("-")[-1]) if key and key.split("-")[-1].isdigit() else 0
        bug_rows.append(
            f'<tr class="bug-row" data-driver="{driver}" data-status="{status}"'
            f' data-ticket="{ticket_num}"'
            f' data-summary="{summary.lower()}"'
            f' data-date="{html.escape(str(commit_date))}"'
            f' data-priority="{pri_idx}"'
            f' data-analysis="{sta_idx}">'
            f'<td>{jira_link(key)}</td>'
            f'<td class="summary-cell">{summary}</td>'
            f'<td class="date-cell">{html.escape(str(commit_date))}</td>'
            f'<td>{priority_badge(priority)}</td>'
            f'<td>{status_badge(status)}</td>'
            f'</tr>'
        )

    return f"""
<div id="{driver}" class="driver-section">
  <section>
    <h2>{driver.upper()} Driver — Bug Analysis</h2>
    <div class="stat-pills">{pills}</div>
    <div class="bar-large">{large_bar}</div>
    <table class="bug-table" data-driver="{driver}">
      <thead>
        <tr>
          <th data-col="ticket" class="sortable">Ticket</th>
          <th data-col="summary" class="sortable">Summary</th>
          <th data-col="date" class="sortable">Fix Date</th>
          <th data-col="priority" class="sortable">Priority</th>
          <th data-col="analysis" class="sortable">Status</th>
        </tr>
      </thead>
      <tbody>{"".join(bug_rows)}</tbody>
    </table>
  </section>
</div>
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
.driver-tab.active { color: #0f172a; background: #f8fafc; }

main { max-width: 1280px; margin: 0 auto; padding: 24px 32px; }

h2 { font-size: 16px; font-weight: 600; color: #0f172a; margin-bottom: 16px;
     padding-bottom: 8px; border-bottom: 1px solid #e2e8f0; }

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
.bar-cell { width: 200px; }
.driver-link { font-weight: 600; }
.tab-jump { background: none; border: none; padding: 0; cursor: pointer;
            color: #3b82f6; font-weight: 600; font-size: 14px; }
.tab-jump:hover { text-decoration: underline; }

/* Progress bars */
.bar, .bar-large { display: flex; border-radius: 4px; overflow: hidden; background: #f1f5f9; }
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

/* Bug table */
.bug-table { width: 100%; border-collapse: collapse; }
.bug-table th { font-size: 11px; font-weight: 600; text-transform: uppercase;
                letter-spacing: .05em; padding: 6px 10px; text-align: left;
                color: #64748b; border-bottom: 2px solid #e2e8f0;
                position: sticky; top: 0; background: #fff; z-index: 1; }
.bug-table th.sortable { cursor: pointer; user-select: none; white-space: nowrap; }
.bug-table th.sortable:hover { color: #334155; }
.bug-table th.sortable::after { content: " ⇅"; color: #cbd5e1; font-size: 10px; }
.bug-table th.sort-asc::after  { content: " ▲"; color: #3b82f6; }
.bug-table th.sort-desc::after { content: " ▼"; color: #3b82f6; }
.bug-table td { padding: 7px 10px; border-bottom: 1px solid #f1f5f9; vertical-align: middle; }
.bug-table tbody tr:last-child td { border-bottom: none; }
.bug-table tbody tr { transition: background .1s; }
.bug-table tbody tr:hover { background: #f8fafc; }
.bug-row.hidden { display: none; }
.summary-cell { max-width: 540px; font-size: 13px; color: #334155; }
.date-cell { font-size: 12px; color: #94a3b8; white-space: nowrap; }

/* Badges */
.badge { font-size: 10px; font-weight: 600; padding: 2px 8px; border-radius: 10px;
         border: 1px solid; white-space: nowrap; text-transform: uppercase;
         letter-spacing: .04em; }
"""

JS = """
// Driver tab switching
document.querySelectorAll('.driver-tab').forEach(btn => {
  btn.addEventListener('click', () => {
    const driver = btn.dataset.driver;
    document.querySelectorAll('.driver-tab').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.driver-section').forEach(s => s.classList.remove('active'));
    btn.classList.add('active');
    const sec = document.getElementById(driver);
    if (sec) sec.classList.add('active');
  });
});

// Tab-jump buttons from overview
document.querySelectorAll('.tab-jump').forEach(btn => {
  btn.addEventListener('click', () => {
    const driver = btn.dataset.driver;
    document.querySelector(`.driver-tab[data-driver="${driver}"]`)?.click();
  });
});

// Column sorting
document.querySelectorAll('.bug-table th.sortable').forEach(th => {
  th.addEventListener('click', () => {
    const table = th.closest('table');
    const col = th.dataset.col;
    const wasAsc = th.classList.contains('sort-asc');
    const asc = !wasAsc;

    // Reset all headers in this table
    table.querySelectorAll('th.sortable').forEach(h => {
      h.classList.remove('sort-asc', 'sort-desc');
    });
    th.classList.add(asc ? 'sort-asc' : 'sort-desc');

    const tbody = table.querySelector('tbody');
    const rows = Array.from(tbody.querySelectorAll('tr.bug-row'));

    rows.sort((a, b) => {
      let av = a.dataset[col] ?? '';
      let bv = b.dataset[col] ?? '';
      // Numeric columns
      if (col === 'ticket' || col === 'priority' || col === 'analysis') {
        av = parseFloat(av) || 0;
        bv = parseFloat(bv) || 0;
        return asc ? av - bv : bv - av;
      }
      // String columns
      return asc ? av.localeCompare(bv) : bv.localeCompare(av);
    });

    rows.forEach(r => tbody.appendChild(r));
  });
});

// Status filter pills
document.querySelectorAll('.filter-btn').forEach(pill => {
  pill.addEventListener('click', () => {
    const driver = pill.dataset.driver;
    const status = pill.dataset.status;

    // Update active pill for this driver
    document.querySelectorAll(`.filter-btn[data-driver="${driver}"]`).forEach(p => {
      p.classList.remove('filter-active');
    });
    pill.classList.add('filter-active');

    // Show/hide rows
    document.querySelectorAll(`.bug-row[data-driver="${driver}"]`).forEach(row => {
      if (status === 'all' || row.dataset.status === status) {
        row.classList.remove('hidden');
      } else {
        row.classList.add('hidden');
      }
    });
  });
});
"""


def build_page(all_data: list[dict], ts: str) -> str:
    tabs = '<button class="driver-tab active" data-driver="overview">Overview</button>'
    for d in all_data:
        tabs += f'<button class="driver-tab" data-driver="{d["driver"]}">{d["driver"].upper()}</button>'

    overview = overview_section(all_data)

    driver_secs = "\n".join(driver_section(d) for d in all_data)

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>UD Bug Regression Coverage Report</title>
<style>{CSS}</style>
</head>
<body>
<header>
  <div class="header-top">
    <h1>Universal Driver — Bug Regression Coverage</h1>
    <span class="ts">Generated {html.escape(ts)}</span>
  </div>
  <nav class="driver-tabs">{tabs}</nav>
</header>
<main>
  <div id="overview" class="driver-section active">
    {overview}
  </div>
  {driver_secs}
</main>
<script>{JS}</script>
</body>
</html>
"""

# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="Generate bugs-analysis HTML report")
    parser.add_argument("--out", type=Path, default=DATA_DIR / "report.html")
    args = parser.parse_args()

    ts = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    all_data = [collect_data(d) for d in DRIVER_FILES]
    page = build_page(all_data, ts)

    args.out.write_text(page, encoding="utf-8")
    print(f"Wrote {args.out}  ({len(page):,} bytes)")


if __name__ == "__main__":
    main()
