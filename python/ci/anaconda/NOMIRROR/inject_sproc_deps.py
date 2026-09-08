"""Add stored-procedure dependencies to conda repodata."""

from __future__ import annotations

import json
import sys

from pathlib import Path


EXTRA_DEPENDENCIES = ("pyarrow", "pandas<3.0.0")

CONNECTOR_PACKAGE = "snowflake-connector-python"


def _requirement_name(spec: str) -> str:
    return spec.split()[0].split("<")[0].split(">")[0].split("=")[0].split("!")[0]


def inject(path: Path) -> int:
    with path.open() as handle:
        data = json.load(handle)

    added = 0
    for section in ("packages", "packages.conda"):
        for metadata in (data.get(section) or {}).values():
            depends = metadata.get("depends")
            if not depends:
                continue
            if not any(_requirement_name(spec) == CONNECTOR_PACKAGE for spec in depends if spec):
                continue
            present = {_requirement_name(spec) for spec in depends if spec}
            for extra in EXTRA_DEPENDENCIES:
                if _requirement_name(extra) not in present:
                    depends.append(extra)
                    added += 1

    with path.open("w") as handle:
        json.dump(data, handle, indent=2)
    return added


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__)
        return 1
    for raw in argv:
        path = Path(raw)
        if not path.is_file():
            print(f"[FAILURE] not a file: {path}", file=sys.stderr)
            return 1
        added = inject(path)
        print(f"{path}: injected {added} sproc dependency entr{'y' if added == 1 else 'ies'}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
