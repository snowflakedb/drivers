#!/usr/bin/env python3
"""Run check_linked_crypto.py on the driver inside one hashed package."""

import argparse
from pathlib import Path
import subprocess
import sys
import tempfile

import package_evidence as evidence


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--linker-map", type=Path)
    args = parser.parse_args()
    if sys.platform == "win32" and args.linker_map is None:
        parser.error("Windows requires --linker-map")
    checker = Path(__file__).with_name("check_linked_crypto.py")
    try:
        with tempfile.TemporaryDirectory(prefix="packaged-driver-") as tmp:
            dest = Path(tmp) / "payload"
            dest.mkdir()
            driver = evidence.packaged_driver(args.package, dest)
            command = [sys.executable, str(checker), str(driver)]
            if args.linker_map is not None:
                command.extend(["--linker-map", str(args.linker_map)])
            result = subprocess.run(command)
    except (OSError, evidence.PackageEvidenceError, subprocess.SubprocessError, ValueError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
