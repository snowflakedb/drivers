#!/usr/bin/env python3
import argparse, json, os, sys
from typing import Dict, Iterable, Set


def load_json(path: str) -> dict:
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        print(f"[compare] missing file: {path}", file=sys.stderr)
        sys.exit(2)

def outcomes(js: dict) -> Dict[str, str]:
    # pytest-json-report schema
    tests = js.get("tests", [])
    return {t["nodeid"]: t["outcome"] for t in tests}

def block(title: str, items: Iterable[str], limit: int = 80) -> str:
    s: Set[str] = set(items)
    out = [f"### {title} ({len(s)})"]
    if not s:
        return "\n".join(out + ["_none_", ""])
    for i, node in enumerate(sorted(s)):
        if i >= limit:
            out.append(f"- … and {len(s) - limit} more")
            break
        out.append(f"- `{node}`")
    out.append("")
    return "\n".join(out)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--py", required=True, help="Python version label, e.g. 3.12")
    ap.add_argument("--universal", required=True, help="Path to universal JSON report")
    ap.add_argument("--reference", required=True, help="Path to reference JSON report")
    ap.add_argument("--summary", default="", help="Path to GITHUB_STEP_SUMMARY (optional)")
    ap.add_argument("--fail-on-regressions", type=int, default=0, help="1 to exit nonzero if regressions exist")
    args = ap.parse_args()

    U = outcomes(load_json(args.universal))
    R = outcomes(load_json(args.reference))

    u_pass = {k for k,v in U.items() if v == "passed"}
    u_fail = {k for k,v in U.items() if v in ("failed","error")}
    u_skip = {k for k,v in U.items() if v == "skipped"}

    r_pass = {k for k,v in R.items() if v == "passed"}
    r_fail = {k for k,v in R.items() if v in ("failed","error")}
    r_skip = {k for k,v in R.items() if v == "skipped"}

    regress     = r_pass & u_fail
    bcrs     = r_fail & u_pass
    both_fail   = r_fail & u_fail
    only_u_skip = u_skip - r_skip
    only_r_skip = r_skip - u_skip

    header = f"## Universal vs Reference — Python {args.py}\n"
    counts = (
        f"- Total (universal): {len(U)} | pass {len(u_pass)} / fail {len(u_fail)} / skip {len(u_skip)}\n"
        f"- Total (reference): {len(R)} | pass {len(r_pass)} / fail {len(r_fail)} / skip {len(r_skip)}\n\n"
    )
    body = "".join([
        block("Regressions (ref ✅ / universal ❌)", regress),
        block("Breaking changes (ref ❌ / universal ✅)", bcrs),
        block("Both failing", both_fail),
        block("Skipped only on universal", only_u_skip),
        block("Skipped only on reference", only_r_skip),
    ])
    md = header + counts + body
    print(md)

    if args.summary:
        try:
            with open(args.summary, "a", encoding="utf-8") as s:
                s.write(md)
        except Exception as e:
            print(f"[compare] could not write summary: {e}", file=sys.stderr)

    if args.fail_on_regressions and regress:
        sys.exit(1)

if __name__ == "__main__":
    main()
