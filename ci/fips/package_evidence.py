#!/usr/bin/env python3
"""Write a FIPS evidence bundle for one finished ODBC package.

The subject is the package file itself. The SBOM lists the files that package
installs. Release Engineering signing and notarization happen later and change
those bytes, so this bundle does not cover the published file.
"""

import argparse
import gzip
import hashlib
import io
import json
import lzma
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile


STAGE = "gha-package-candidate"
RPM_LEAD = 96
RPM_MAGIC = b"\xed\xab\xee\xdb"
RPM_HEADER_MAGIC = b"\x8e\xad\xe8\x01\x00\x00\x00\x00"
CPIO_NEWC = (b"070701", b"070702")
S_IFMT = 0o170000
S_IFDIR = 0o040000
S_IFLNK = 0o120000


class PackageEvidenceError(Exception):
    pass


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _align(value: int, boundary: int) -> int:
    return (value + boundary - 1) & ~(boundary - 1)


def safe_member(dest: Path, name: str) -> Path:
    relative = Path(name)
    if relative.is_absolute() or any(part == ".." for part in relative.parts):
        raise PackageEvidenceError(f"payload path escapes the package: {name}")
    path = dest / relative
    parent = path.parent.resolve()
    root = dest.resolve()
    if parent != root and root not in parent.parents:
        raise PackageEvidenceError(f"payload path escapes the package: {name}")
    return path


def extract_tar_bytes(payload: bytes, dest: Path) -> None:
    try:
        with tarfile.open(fileobj=io.BytesIO(payload), mode="r:*") as archive:
            try:
                archive.extractall(dest, filter="data")
            except TypeError:
                _extract_tar_unchecked(archive, dest)
    except (tarfile.TarError, PackageEvidenceError) as error:
        raise PackageEvidenceError(f"tar payload: {error}") from error


def _extract_tar_unchecked(archive: tarfile.TarFile, dest: Path) -> None:
    for member in archive.getmembers():
        path = safe_member(dest, member.name)
        if member.issym():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.symlink_to(member.linkname)
            continue
        if member.isdir():
            path.mkdir(parents=True, exist_ok=True)
            continue
        if not member.isfile():
            continue
        source = archive.extractfile(member)
        if source is None:
            raise PackageEvidenceError(f"missing tar member {member.name}")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(source.read())


def iter_ar(data: bytes):
    if not data.startswith(b"!<arch>\n"):
        raise PackageEvidenceError("deb is not an ar archive")
    offset = 8
    while offset + 60 <= len(data):
        header = data[offset:offset + 60]
        size_text = header[48:58].decode("ascii", "replace").strip()
        if not size_text:
            break
        size = int(size_text)
        offset += 60
        body = data[offset:offset + size]
        if len(body) != size:
            raise PackageEvidenceError("deb ar member is truncated")
        offset += size + (size % 2)
        name = header[0:16].decode("ascii", "replace").strip()
        if name.startswith("#1/"):
            name_len = int(name[3:])
            name = body[:name_len].decode("utf-8").rstrip("\x00")
            body = body[name_len:]
        yield name.rstrip("/"), body


def extract_deb(package: Path, dest: Path) -> None:
    payload = None
    for name, body in iter_ar(package.read_bytes()):
        if name.startswith("data.tar"):
            payload = body
            break
    if payload is None:
        raise PackageEvidenceError(f"{package.name} has no data.tar member")
    extract_tar_bytes(payload, dest)


def _skip_rpm_header(data: bytes, offset: int) -> int:
    if data[offset:offset + 8] != RPM_HEADER_MAGIC:
        raise PackageEvidenceError(f"rpm header magic mismatch at {offset}")
    count = int.from_bytes(data[offset + 8:offset + 12], "big")
    store = int.from_bytes(data[offset + 12:offset + 16], "big")
    return offset + 16 + count * 16 + store


def rpm_cpio(data: bytes) -> bytes:
    if not data.startswith(RPM_MAGIC):
        raise PackageEvidenceError("not an rpm")
    # The signature header ends on an 8-byte boundary. The payload follows the
    # main header with no further padding.
    offset = _align(_skip_rpm_header(data, RPM_LEAD), 8)
    offset = _skip_rpm_header(data, offset)
    payload = data[offset:]
    if payload.startswith(b"\x1f\x8b"):
        return gzip.decompress(payload)
    if payload.startswith(b"\xfd7zXZ\x00"):
        return lzma.decompress(payload)
    if payload.startswith(b"\x28\xb5\x2f\xfd"):
        return subprocess.check_output(["zstd", "-dc"], input=payload)
    if payload.startswith(CPIO_NEWC):
        return payload
    raise PackageEvidenceError("unrecognized rpm payload compression")


def extract_newc(payload: bytes, dest: Path) -> None:
    offset = 0
    while offset < len(payload):
        if offset + 110 > len(payload):
            raise PackageEvidenceError("cpio header is truncated")
        magic = payload[offset:offset + 6]
        if magic not in CPIO_NEWC:
            raise PackageEvidenceError(f"cpio magic {magic!r}")
        fields = [
            int(payload[offset + 6 + index * 8:offset + 14 + index * 8], 16)
            for index in range(13)
        ]
        mode, file_size, name_size = fields[1], fields[6], fields[11]
        name_start = offset + 110
        name = payload[name_start:name_start + name_size].split(b"\x00", 1)[0].decode()
        data_start = offset + _align(110 + name_size, 4)
        data_end = data_start + file_size
        if data_end > len(payload):
            raise PackageEvidenceError(f"cpio member {name} is truncated")
        offset = data_end + ((4 - (file_size % 4)) % 4)
        if name in {"TRAILER!!!", "."}:
            if name == "TRAILER!!!":
                return
            continue
        path = safe_member(dest, name)
        kind = mode & S_IFMT
        if kind == S_IFDIR:
            path.mkdir(parents=True, exist_ok=True)
        elif kind == S_IFLNK:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.symlink_to(payload[data_start:data_end].decode())
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(payload[data_start:data_end])


def extract_rpm(package: Path, dest: Path) -> None:
    extract_newc(rpm_cpio(package.read_bytes()), dest)


def _run(command: list[str], runner) -> None:
    completed = runner(command)
    if completed.returncode != 0:
        raise PackageEvidenceError(
            f"{command[0]} exited {completed.returncode} for {command[-1]}"
        )


def extract_pkg(package: Path, dest: Path, runner=subprocess.run) -> None:
    with tempfile.TemporaryDirectory(prefix="pkg-evidence-") as tmp:
        expanded = Path(tmp) / "expanded"
        _run(["pkgutil", "--expand-full", str(package), str(expanded)], runner)
        payload = expanded / "Payload"
        if not payload.is_dir():
            raise PackageEvidenceError(f"{package.name} payload was not expanded")
        for child in payload.iterdir():
            shutil.move(str(child), dest / child.name)


def extract_msi(package: Path, dest: Path, runner=subprocess.run) -> None:
    target = str(dest)
    if not target.endswith(("\\", "/")):
        target += "\\"
    _run(["msiexec", "/a", str(package), "/qn", f"TARGETDIR={target}"], runner)


DRIVER_FILENAMES = frozenset({"libsfodbc.so", "libsfodbc.dylib"})


def packaged_driver(package: Path, dest: Path, runner=subprocess.run) -> Path:
    extract_payload(package, dest, runner)
    found = sorted(
        path for path in dest.rglob("*")
        if path.is_file() and not path.is_symlink() and path.name in DRIVER_FILENAMES
    )
    if len(found) != 1:
        raise PackageEvidenceError(
            f"{package.name}: expected one packaged libsfodbc driver, found {len(found)}"
        )
    return found[0]


def extract_payload(package: Path, dest: Path, runner=subprocess.run) -> None:
    name = package.name.lower()
    if name.endswith((".tar.gz", ".tgz")):
        extract_tar_bytes(package.read_bytes(), dest)
    elif name.endswith(".deb"):
        extract_deb(package, dest)
    elif name.endswith(".rpm"):
        extract_rpm(package, dest)
    elif name.endswith(".pkg"):
        extract_pkg(package, dest, runner)
    elif name.endswith(".msi"):
        extract_msi(package, dest, runner)
    else:
        raise PackageEvidenceError(f"unsupported package type: {package.name}")


def inventory(root: Path) -> list[dict]:
    records = []
    for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
        dirnames.sort()
        for filename in sorted(filenames):
            path = Path(dirpath) / filename
            relative = path.relative_to(root).as_posix()
            if path.is_symlink():
                records.append({
                    "path": relative,
                    "kind": "symlink",
                    "target": os.readlink(path),
                })
            elif path.is_file():
                records.append({
                    "path": relative,
                    "kind": "file",
                    "size": path.stat().st_size,
                    "sha256": sha256_file(path),
                })
    if not records:
        raise PackageEvidenceError(f"package payload under {root} has no files")
    return records


def _component(record: dict) -> dict:
    component = {"type": "file", "name": record["path"]}
    if record["kind"] == "symlink":
        component["properties"] = [{
            "name": "snowflake:symlink-target",
            "value": record["target"],
        }]
        return component
    component["hashes"] = [{"alg": "SHA-256", "content": record["sha256"]}]
    return component


def write_bundle(package: Path, output_dir: Path, runner=subprocess.run) -> None:
    if not package.is_file():
        raise PackageEvidenceError(f"package is not a file: {package}")
    package_sha = sha256_file(package)
    with tempfile.TemporaryDirectory(prefix="package-evidence-") as tmp:
        payload = Path(tmp) / "payload"
        payload.mkdir()
        extract_payload(package, payload, runner)
        records = inventory(payload)
    sbom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "name": package.name,
                "hashes": [{"alg": "SHA-256", "content": package_sha}],
            },
            "properties": [
                {"name": "snowflake:subject-name", "value": package.name},
                {"name": "snowflake:subject-sha256", "value": package_sha},
                {"name": "snowflake:stage", "value": STAGE},
                {"name": "snowflake:covers-releng-published-bytes", "value": "false"},
            ],
            "tools": [{
                "type": "application",
                "name": "package_evidence.py",
                "version": "1",
            }],
        },
        "components": [_component(record) for record in records],
    }
    output_dir.mkdir(parents=True, exist_ok=True)
    sbom_path = output_dir / "sbom.cdx.json"
    sbom_path.write_text(json.dumps(sbom, indent=2) + "\n", encoding="utf-8")
    sbom_sha = sha256_file(sbom_path)
    hashes = {
        "package": {"name": package.name, "sha256": package_sha},
        "sbom": {"name": sbom_path.name, "sha256": sbom_sha},
        "payload": records,
    }
    (output_dir / "hashes.json").write_text(json.dumps(hashes, indent=2) + "\n", encoding="utf-8")
    manifest = {
        "schema_version": 1,
        "stage": STAGE,
        "covers_releng_published_bytes": False,
        "package_name": package.name,
        "package_sha256": package_sha,
        "sbom_sha256": sbom_sha,
        "payload_files": len(records),
        "source_sha": os.environ.get("GITHUB_SHA", ""),
    }
    (output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    try:
        write_bundle(args.package, args.output_dir)
    except (OSError, PackageEvidenceError, subprocess.SubprocessError, ValueError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print(f"PASS {args.package.name}: wrote {args.output_dir}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
