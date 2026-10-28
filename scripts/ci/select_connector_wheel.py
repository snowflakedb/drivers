#!/usr/bin/env python3
"""Print the snowflake_connector_python wheel path for a CPython tag.

SNOW-4350374: `gh run download` extracts the named wheel artifact into dist/,
but dist/ can already hold extra *.whl files from an earlier uv/hatch step
or a multi-file artifact. Callers must not glob dist/*.whl.
"""

from __future__ import annotations

import sys
from pathlib import Path


def main(argv: list[str]) -> int:
    if len(argv) != 3:
        print("usage: select_connector_wheel.py DIST_DIR PY_VERSION", file=sys.stderr)
        return 2
    dist = Path(argv[1])
    py_tag = "cp" + argv[2].replace(".", "")
    wheels = sorted(dist.glob(f"snowflake_connector_python-*-{py_tag}-*.whl"))
    if len(wheels) != 1:
        print(
            f"::error::Expected exactly 1 snowflake_connector_python wheel for {py_tag}, "
            f"found {len(wheels)}",
            file=sys.stderr,
        )
        if dist.exists():
            for path in sorted(dist.iterdir()):
                print(path, file=sys.stderr)
        return 1
    print(wheels[0])
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
