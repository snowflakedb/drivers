#!/usr/bin/env python3
import argparse
import gzip
import bz2
import brotli
import io
import os
import sys
import zlib
from pathlib import Path

try:
    import zstandard as zstd
except Exception:
    zstd = None


BASE_CONTENT = b"1,2,3\n"

# Map of compression name -> (extension, compress_func)
# compress_func takes bytes and returns compressed bytes
COMPRESSORS = {
    "NONE": ("", lambda b: b),
    "GZIP": (".gz", lambda b: gzip.compress(b)),
    "BZIP2": (".bz2", lambda b: bz2.compress(b)),
    "BROTLI": (".br", lambda b: brotli.compress(b)),
    "ZSTD": (".zst", lambda b: (zstd.ZstdCompressor().compress(b) if zstd else _raise_no_zstd())),
    # Note: Snowflake uses two deflate modes in tests: DEFLATE (.deflate) and RAW_DEFLATE (.raw_deflate)
    # For our purposes, generate a zlib-wrapped DEFLATE stream for DEFLATE,
    # and a raw deflate stream for RAW_DEFLATE.
}


def _raise_no_zstd():
    raise RuntimeError("zstandard package not available; install with pip install zstandard")


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


def generate_csv_variants(out_dir: Path, base_name: str = "test_data.csv") -> dict:
    created = {}
    # Uncompressed base
    base_path = out_dir / base_name
    write_file(base_path, BASE_CONTENT)
    created["NONE"] = base_path

    # Standard compressed variants
    gzip_path = out_dir / f"{base_name}.gz"
    write_file(gzip_path, gzip.compress(BASE_CONTENT))
    created["GZIP"] = gzip_path

    bzip2_path = out_dir / f"{base_name}.bz2"
    write_file(bzip2_path, bz2.compress(BASE_CONTENT))
    created["BZIP2"] = bzip2_path

    brotli_path = out_dir / f"{base_name}.br"
    write_file(brotli_path, brotli.compress(BASE_CONTENT))
    created["BROTLI"] = brotli_path

    if zstd is not None:
        zstd_path = out_dir / f"{base_name}.zst"
        write_file(zstd_path, zstd.ZstdCompressor().compress(BASE_CONTENT))
        created["ZSTD"] = zstd_path

    # DEFLATE and RAW_DEFLATE
    deflate_path = out_dir / f"{base_name}.deflate"
    write_file(deflate_path, compress_deflate(BASE_CONTENT))
    created["DEFLATE"] = deflate_path

    raw_deflate_path = out_dir / f"{base_name}.raw_deflate"
    write_file(raw_deflate_path, compress_raw_deflate(BASE_CONTENT))
    created["RAW_DEFLATE"] = raw_deflate_path

    # Named files used by tests for auto-detect based on extension
    write_file(out_dir / "test_gzip.csv.gz", gzip.compress(BASE_CONTENT))
    write_file(out_dir / "test_bzip2.csv.bz2", bz2.compress(BASE_CONTENT))
    write_file(out_dir / "test_brotli.csv.br", brotli.compress(BASE_CONTENT))
    if zstd is not None:
        write_file(out_dir / "test_zstd.csv.zst", zstd.ZstdCompressor().compress(BASE_CONTENT))
    write_file(out_dir / "test_deflate.csv.deflate", compress_deflate(BASE_CONTENT))
    write_file(out_dir / "test_raw_deflate.csv.raw_deflate", compress_raw_deflate(BASE_CONTENT))

    # Content-based detection case: gzip without extension
    gzip_no_ext_path = out_dir / "test_auto_detect_no_extension"
    write_file(gzip_no_ext_path, gzip.compress(BASE_CONTENT))
    created["GZIP_NO_EXT"] = gzip_no_ext_path

    # Unsupported extension example (.lz) with plain content
    unsupported_lz = out_dir / "test_auto_detect.csv.lz"
    write_file(unsupported_lz, BASE_CONTENT)
    created["UNSUPPORTED_LZ"] = unsupported_lz

    # Files used by explicit SOURCE_COMPRESSION tests (no extension but compressed content)
    write_file(out_dir / "test_gzip.csv", gzip.compress(BASE_CONTENT))
    write_file(out_dir / "test_bzip2.csv", bz2.compress(BASE_CONTENT))
    if zstd is not None:
        write_file(out_dir / "test_zstd.csv", zstd.ZstdCompressor().compress(BASE_CONTENT))
    write_file(out_dir / "test_deflate.csv", compress_deflate(BASE_CONTENT))

    # Files used by overwrite and wildcard tests
    write_file(out_dir / "test_put_select.csv", BASE_CONTENT)
    write_file(out_dir / "test_put_ls.csv", BASE_CONTENT)
    write_file(out_dir / "test_get.csv", BASE_CONTENT)
    write_file(out_dir / "test_put_get_rowset.csv", BASE_CONTENT)
    write_file(out_dir / "test_none.csv", BASE_CONTENT)
    # Auto-compress tests
    write_file(out_dir / "test_put_get_compress_true.csv", BASE_CONTENT)
    write_file(out_dir / "test_put_get_compress_false.csv", BASE_CONTENT)

    # UTF-8 filename sample
    write_file(out_dir / "utf卡豆.csv", BASE_CONTENT)

    # Wildcard sets
    for i in range(1, 6):
        write_file(out_dir / f"test_put_wildcard_question_mark_{i}.csv", BASE_CONTENT)
    write_file(out_dir / "test_put_wildcard_question_mark_10.csv", BASE_CONTENT)
    write_file(out_dir / "test_put_wildcard_question_mark_abc.csv", BASE_CONTENT)

    for i in range(1, 6):
        write_file(out_dir / f"test_put_wildcard_star_{i}{i}{i}.csv", BASE_CONTENT)
    write_file(out_dir / "test_put_wildcard_star.csv", BASE_CONTENT)
    write_file(out_dir / "test_put_wildcard_star_test.txt", BASE_CONTENT)

    # Regexp set
    for i in range(1, 6):
        write_file(out_dir / f"data_{i}.csv", BASE_CONTENT)
    write_file(out_dir / "data_10.csv", BASE_CONTENT)
    write_file(out_dir / "data_abc.csv", BASE_CONTENT)

    # Overwrite sets
    write_file(out_dir / "test_overwrite_true.csv", b"original,data,1\n")
    write_file(out_dir / "test_overwrite_true_updated.csv", b"updated,data,2\n")
    write_file(out_dir / "test_overwrite_false.csv", b"original,data,1\n")
    write_file(out_dir / "test_overwrite_false_updated.csv", b"updated,data,2\n")
    for i in range(1, 4):
        write_file(out_dir / f"test_overwrite_mixed_{i}.csv", f"file{i},content,{i}\n".encode())
    # Optional updated variant used in some flows
    write_file(out_dir / "test_overwrite_mixed_2_updated.csv", b"file2,new_content,2\n")

    # Explicit compression tests
    # .dat files with actual compressed content for Rust tests
    write_file(out_dir / "test_explicit_gzip.dat", gzip.compress(BASE_CONTENT))
    write_file(out_dir / "test_explicit_bzip2.dat", bz2.compress(BASE_CONTENT))
    write_file(out_dir / "test_explicit_brotli.dat", brotli.compress(BASE_CONTENT))
    if zstd is not None:
        write_file(out_dir / "test_explicit_zstd.dat", zstd.ZstdCompressor().compress(BASE_CONTENT))
    write_file(out_dir / "test_explicit_deflate.dat", compress_deflate(BASE_CONTENT))

    # Files for Python tests that pass explicit compression but without extension
    write_file(out_dir / "test_explicit_brotli", b"brotli")  # no extension
    write_file(out_dir / "test_explicit_raw_deflate", b"rawdeflatedata")  # no extension
    write_file(out_dir / "test_explicit_none.csv.gz", BASE_CONTENT)  # wrong extension, uncompressed
    write_file(out_dir / "test_explicit_with_auto.csv.gz", BASE_CONTENT)  # wrong extension, uncompressed

    return created


def main(argv=None):
    parser = argparse.ArgumentParser(description="Generate shared put/get test data files")
    parser.add_argument(
        "--out-dir",
        default=str(Path("tests/test_data").absolute()),
        help="Directory to write generated files (default: tests/test_data)",
    )
    args = parser.parse_args(argv)

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    created = generate_csv_variants(out_dir)

    print(f"Generated {len(created)} key files under {out_dir}")
    # Print a brief index for debugging
    for k, p in sorted(created.items()):
        try:
            rel = p.relative_to(Path.cwd())
        except Exception:
            rel = p
        print(f" - {k}: {rel}")


if __name__ == "__main__":
    sys.exit(main())
