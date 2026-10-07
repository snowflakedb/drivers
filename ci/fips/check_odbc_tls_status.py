#!/usr/bin/env python3
"""Check FIPS TLS status through the built ODBC C ABI, without a connection."""

import argparse
from contextlib import ExitStack
import ctypes
import os
from pathlib import Path
import sys


class SFTlsStatus(ctypes.Structure):
    _fields_ = [
        ("tls_provider_is_fips", ctypes.c_uint32),
        ("fips_build_enabled", ctypes.c_uint32),
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifact", type=Path)
    args = parser.parse_args()
    try:
        with ExitStack() as dll_directories:
            if sys.platform == "win32":
                # Python 3.8+ does not consult PATH when resolving DLL dependencies.
                for directory in dict.fromkeys(os.environ.get("PATH", "").split(os.pathsep)):
                    if directory and Path(directory).is_dir():
                        dll_directories.enter_context(os.add_dll_directory(directory))
            library = ctypes.CDLL(str(args.artifact.resolve()))
            get_status = library.SFGetTlsStatus
            get_status.argtypes = [ctypes.POINTER(SFTlsStatus)]
            get_status.restype = ctypes.c_int32
            status = SFTlsStatus()
            result = get_status(ctypes.byref(status))
            if result != 0 or status.tls_provider_is_fips != 1 or status.fips_build_enabled != 1:
                raise ValueError(
                    f"SFGetTlsStatus returned {result}; "
                    f"tls_provider_is_fips={status.tls_provider_is_fips}, "
                    f"fips_build_enabled={status.fips_build_enabled}; expected 0, 1, 1"
                )
    except (OSError, AttributeError, ValueError) as error:
        print(f"FAIL {args.artifact}: {error}", file=sys.stderr)
        return 1
    print(f"PASS {args.artifact}: SFGetTlsStatus returned 0; tls_provider_is_fips=1, fips_build_enabled=1")
    return 0


if __name__ == "__main__":
    sys.exit(main())
