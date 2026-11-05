"""Run with: python -m unittest discover -s ci/fips -v."""

import gzip
import hashlib
import io
import json
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path


import package_evidence as evidence


def _tar_gz(members: list[tuple[str, bytes | None]]) -> bytes:
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
        for name, body in members:
            info = tarfile.TarInfo(name)
            if body is None:
                info.type = tarfile.SYMTYPE
                info.linkname = "libsfodbc.so"
                archive.addfile(info)
                continue
            info.size = len(body)
            archive.addfile(info, io.BytesIO(body))
    return buffer.getvalue()


def _ar(members: list[tuple[str, bytes]]) -> bytes:
    parts = [b"!<arch>\n"]
    for name, body in members:
        header = (
            f"{name:<16}".encode()
            + f"{0:<12}{0:<6}{0:<6}{'100644':<8}{len(body):<10}".encode()
            + b"`\n"
        )
        parts.append(header)
        parts.append(body)
        if len(body) % 2:
            parts.append(b"\n")
    return b"".join(parts)


def _newc(name: str, body: bytes, mode: int = 0o100644) -> bytes:
    name_bytes = name.encode() + b"\x00"
    fields = [0, mode, 0, 0, 1, 0, len(body), 0, 0, 0, 0, len(name_bytes), 0]
    header = b"070701" + b"".join(f"{value:08x}".encode() for value in fields)
    name_pad = b"\x00" * ((4 - ((110 + len(name_bytes)) % 4)) % 4)
    data_pad = b"\x00" * ((4 - (len(body) % 4)) % 4)
    return header + name_bytes + name_pad + body + data_pad


def _rpm(cpio_payload: bytes) -> bytes:
    header = b"\x8e\xad\xe8\x01\x00\x00\x00\x00" + b"\x00" * 8
    lead = b"\xed\xab\xee\xdb" + b"\x00" * 92
    return lead + header + header + gzip.compress(cpio_payload)


class PackageEvidenceTests(unittest.TestCase):
    def write(self, name: str, payload: bytes) -> tuple[Path, Path]:
        root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, root)
        package = root / name
        package.write_bytes(payload)
        return package, root / "out"

    def assert_driver(self, package: Path, output: Path, body: bytes) -> None:
        evidence.write_bundle(package, output)
        manifest = json.loads((output / "manifest.json").read_text(encoding="utf-8"))
        sbom = json.loads((output / "sbom.cdx.json").read_text(encoding="utf-8"))
        package_sha = hashlib.sha256(package.read_bytes()).hexdigest()
        self.assertEqual(manifest["stage"], "gha-package-candidate")
        self.assertFalse(manifest["covers_releng_published_bytes"])
        self.assertEqual(manifest["package_sha256"], package_sha)
        self.assertEqual(sbom["metadata"]["component"]["name"], package.name)
        self.assertEqual(
            sbom["metadata"]["component"]["hashes"][0]["content"],
            package_sha,
        )
        by_name = {component["name"]: component for component in sbom["components"]}
        self.assertEqual(by_name["usr/lib/libsfodbc.so"]["hashes"][0]["content"], hashlib.sha256(body).hexdigest())
        self.assertEqual(
            by_name["usr/lib/libsfodbc.so.link"]["properties"][0]["value"],
            "libsfodbc.so",
        )
        self.assertNotIn("hashes", by_name["usr/lib/libsfodbc.so.link"])

    def test_tar_gz_bundle_hashes_the_package_and_lists_payload_files(self) -> None:
        body = b"driver-bytes"
        package, output = self.write(
            "snowflake-odbc-fips-1.2.3.x86_64.tar.gz",
            _tar_gz([
                ("usr/lib/libsfodbc.so", body),
                ("usr/lib/libsfodbc.so.link", None),
            ]),
        )
        self.assert_driver(package, output, body)

    def test_deb_bundle_uses_the_data_archive(self) -> None:
        body = b"driver-bytes"
        data = _tar_gz([
            ("usr/lib/libsfodbc.so", body),
            ("usr/lib/libsfodbc.so.link", None),
        ])
        package, output = self.write(
            "snowflake-odbc-fips-1.2.3.amd64.deb",
            _ar([
                ("debian-binary", b"2.0\n"),
                ("control.tar.gz", _tar_gz([("control", b"Package: snowflake-odbc-fips\n")])),
                ("data.tar.gz", data),
            ]),
        )
        self.assert_driver(package, output, body)

    def test_rpm_bundle_reads_the_cpio_payload(self) -> None:
        body = b"driver-bytes"
        cpio = b"".join([
            _newc("usr/lib/libsfodbc.so", body),
            _newc("usr/lib/libsfodbc.so.link", b"libsfodbc.so", 0o120777),
            _newc("TRAILER!!!", b""),
        ])
        package, output = self.write("snowflake-odbc-fips-1.2.3.x86_64.rpm", _rpm(cpio))
        self.assert_driver(package, output, body)

    def test_pkg_inventory_ignores_package_metadata(self) -> None:
        calls = []

        def runner(command):
            calls.append(command)
            expanded = Path(command[-1])
            payload = expanded / "Payload" / "lib"
            payload.mkdir(parents=True)
            (payload / "libsfodbc.dylib").write_bytes(b"dylib")
            (expanded / "PackageInfo").write_text("<pkg/>", encoding="utf-8")
            return subprocess.CompletedProcess(command, 0)

        package, output = self.write("snowflake-odbc-fips-1.2.3.universal.pkg", b"not-a-real-pkg")
        evidence.write_bundle(package, output, runner)
        self.assertEqual(calls[0][:2], ["pkgutil", "--expand-full"])
        sbom = json.loads((output / "sbom.cdx.json").read_text(encoding="utf-8"))
        names = [component["name"] for component in sbom["components"]]
        self.assertEqual(names, ["lib/libsfodbc.dylib"])

    def test_msi_extract_uses_an_administrative_install(self) -> None:
        def runner(command):
            target = command[-1].split("=", 1)[1].rstrip("\\/")
            install = Path(target) / "Bin"
            install.mkdir(parents=True)
            (install / "sfodbc.dll").write_bytes(b"dll")
            return subprocess.CompletedProcess(command, 0)

        package, output = self.write("snowflake-odbc-fips-1.2.3.msi", b"not-a-real-msi")
        evidence.write_bundle(package, output, runner)
        sbom = json.loads((output / "sbom.cdx.json").read_text(encoding="utf-8"))
        self.assertEqual(sbom["components"][0]["name"], "Bin/sfodbc.dll")

    def test_cpio_rejects_a_path_that_escapes_the_payload(self) -> None:
        package, output = self.write(
            "escape.rpm",
            _rpm(_newc("../evil", b"nope") + _newc("TRAILER!!!", b"")),
        )
        with self.assertRaises(evidence.PackageEvidenceError):
            evidence.write_bundle(package, output)

    def test_packaged_driver_is_the_regular_file_inside_the_archive(self) -> None:
        body = b"driver-bytes"
        package, _output = self.write(
            "snowflake-odbc-fips-1.2.3.x86_64.tar.gz",
            _tar_gz([
                ("usr/lib/libsfodbc.so", body),
                ("usr/lib/libsfodbc.so.link", None),
            ]),
        )
        dest = package.parent / "payload"
        dest.mkdir()
        found = evidence.packaged_driver(package, dest)
        self.assertEqual(found.read_bytes(), body)
        self.assertEqual(found.name, "libsfodbc.so")

    def test_packaged_driver_rejects_a_missing_or_duplicate_driver(self) -> None:
        missing, _output = self.write(
            "snowflake-odbc-fips-1.2.3.x86_64.tar.gz",
            _tar_gz([("usr/lib/libsfodbc.so", None)]),
        )
        missing_dest = missing.parent / "payload"
        missing_dest.mkdir()
        with self.assertRaises(evidence.PackageEvidenceError):
            evidence.packaged_driver(missing, missing_dest)
        duplicate, _output = self.write(
            "snowflake-odbc-fips-1.2.3.x86_64.tar.gz",
            _tar_gz([
                ("usr/lib/libsfodbc.so", b"one"),
                ("opt/lib/libsfodbc.so", b"two"),
            ]),
        )
        duplicate_dest = duplicate.parent / "payload"
        duplicate_dest.mkdir()
        with self.assertRaises(evidence.PackageEvidenceError):
            evidence.packaged_driver(duplicate, duplicate_dest)

    def test_unknown_package_type_is_rejected(self) -> None:
        package, output = self.write("notes.txt", b"hello")
        with self.assertRaises(evidence.PackageEvidenceError):
            evidence.write_bundle(package, output)

    @unittest.skipUnless(shutil.which("pkgbuild") and shutil.which("pkgutil"), "pkgbuild required")
    def test_real_pkg_lists_the_payload_and_omits_package_info(self) -> None:
        root = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, root)
        stage = root / "stage" / "lib"
        stage.mkdir(parents=True)
        (stage / "libsfodbc.dylib").write_bytes(b"real-driver")
        package = root / "snowflake-odbc-fips-1.0.universal.pkg"
        subprocess.run(
            [
                "pkgbuild",
                "--identifier", "net.snowflake.evidence-test",
                "--version", "1.0",
                "--root", str(root / "stage"),
                str(package),
            ],
            check=True,
            capture_output=True,
        )
        output = root / "out"
        evidence.write_bundle(package, output)
        sbom = json.loads((output / "sbom.cdx.json").read_text(encoding="utf-8"))
        names = [component["name"] for component in sbom["components"]]
        self.assertEqual(names, ["lib/libsfodbc.dylib"])
        self.assertNotIn("PackageInfo", names)
