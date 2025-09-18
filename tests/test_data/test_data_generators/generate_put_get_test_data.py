#!/usr/bin/env python3
import argparse
import gzip
import bz2
import brotli
import io
import os
import sys
import zlib
import lzma
import zstandard as zstd
from pathlib import Path

# Current catalog structure and exact file contents to generate:
# - tests/test_data/basic/test_data.csv                 -> b"1,2,3\n"
# - tests/test_data/compression/test_data.csv           -> b"1,2,3\n" and compressed variants of this file:
#       test_data.csv.gz, .bz2, .br, .zst, .deflate, .raw_deflate
# - tests/test_data/overwrite/original/test_data.csv    -> b"original,test,data\n"
# - tests/test_data/overwrite/updated/test_data.csv     -> b"updated,test,data\n"
# - tests/test_data/wildcard/pattern_1.csv              -> b"0\n"
#   tests/test_data/wildcard/pattern_2.csv              -> b"0\n"
#   tests/test_data/wildcard/pattern_10.csv             -> b"0\n"
#   tests/test_data/wildcard/patternabc.csv             -> b"0\n"

BASE_CSV_CONTENT = b"1,2,3\n"
OVERWRITE_ORIGINAL_CONTENT = b"original,test,data\n"
OVERWRITE_UPDATED_CONTENT = b"updated,test,data\n"
WILDCARD_CONTENT = b"0\n"


def compress_deflate(data: bytes) -> bytes:
    # zlib-wrapped DEFLATE (equivalent to zlib.compress)
    return zlib.compress(data)


def compress_raw_deflate(data: bytes) -> bytes:
    # raw DEFLATE without zlib header/footer
    comp = zlib.compressobj(level=zlib.Z_DEFAULT_COMPRESSION, wbits=-15)
    out = comp.compress(data) + comp.flush()
    return out


def write_file(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def generate_basic(root: Path) -> list[Path]:
    created: list[Path] = []
    path = root / "basic" / "test_data.csv"
    write_file(path, BASE_CSV_CONTENT)
    created.append(path)
    return created


def generate_compression(root: Path) -> list[Path]:
    created: list[Path] = []
    out_dir = root / "compression"
    base_name = "test_data.csv"

    # Uncompressed base
    base_path = out_dir / base_name
    write_file(base_path, BASE_CSV_CONTENT)
    created.append(base_path)

    # Compressed variants of base
    gzip_path = out_dir / f"{base_name}.gz"
    gz_ba = bytearray(gzip.compress(BASE_CSV_CONTENT, mtime=0))
    # Force gzip header OS byte to 0xFF ("unknown") for deterministic output across platforms
    gz_ba[9] = 0xFF
    write_file(gzip_path, bytes(gz_ba))
    created.append(gzip_path)

    bz2_path = out_dir / f"{base_name}.bz2"
    write_file(bz2_path, bz2.compress(BASE_CSV_CONTENT))
    created.append(bz2_path)

    br_path = out_dir / f"{base_name}.br"
    write_file(br_path, brotli.compress(BASE_CSV_CONTENT))
    created.append(br_path)

    zst_path = out_dir / f"{base_name}.zst"
    write_file(zst_path, zstd.ZstdCompressor().compress(BASE_CSV_CONTENT))
    created.append(zst_path)

    deflate_path = out_dir / f"{base_name}.deflate"
    write_file(deflate_path, compress_deflate(BASE_CSV_CONTENT))
    created.append(deflate_path)

    raw_deflate_path = out_dir / f"{base_name}.raw_deflate"
    write_file(raw_deflate_path, compress_raw_deflate(BASE_CSV_CONTENT))
    created.append(raw_deflate_path)

    lzma_path = out_dir / f"{base_name}.xz"
    write_file(lzma_path, lzma.compress(BASE_CSV_CONTENT))
    created.append(lzma_path)

    return created


def generate_overwrite(root: Path) -> list[Path]:
    created: list[Path] = []
    original_path = root / "overwrite" / "original" / "test_data.csv"
    updated_path = root / "overwrite" / "updated" / "test_data.csv"
    write_file(original_path, OVERWRITE_ORIGINAL_CONTENT)
    write_file(updated_path, OVERWRITE_UPDATED_CONTENT)
    created.extend([original_path, updated_path])
    return created


def generate_wildcard(root: Path) -> list[Path]:
    created: list[Path] = []
    wildcard_dir = root / "wildcard"
    names = [
        "pattern_1.csv",
        "pattern_2.csv",
        "pattern_10.csv",
        "patternabc.csv",
    ]
    for name in names:
        p = wildcard_dir / name
        write_file(p, WILDCARD_CONTENT)
        created.append(p)
    return created


def main(argv=None):
    parser = argparse.ArgumentParser(description="Generate put/get test data catalog")
    parser.add_argument(
        "--out-dir",
        default=str(Path("tests/generated_test_data").absolute()),
        help="Directory to write generated files (default: tests/generated_test_data)",
    )
    args = parser.parse_args(argv)

    root = Path(args.out_dir)
    root.mkdir(parents=True, exist_ok=True)

    created: list[Path] = []
    created += generate_basic(root)
    created += generate_compression(root)
    created += generate_overwrite(root)
    created += generate_wildcard(root)

    print(f"Generated {len(created)} files under {root}")
    for p in sorted(created):
        try:
            rel = p.relative_to(Path.cwd())
        except ValueError:
            rel = p
        print(f" - {rel}")


if __name__ == "__main__":
    sys.exit(main())
