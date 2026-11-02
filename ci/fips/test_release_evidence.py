"""Run with: python -m unittest discover -s ci/fips -v."""

from pathlib import Path
import json
import os
import sys
import tempfile
import unittest


sys.path.insert(0, str(Path(__file__).resolve().parent))

import release_evidence as evidence


REPO = Path(__file__).resolve().parents[2]


def candidate_bom(sys_version: str) -> dict:
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "metadata": {
            "component": {"type": "application", "name": "odbc", "version": "4.1.0"},
        },
        "components": [
            {"type": "library", "name": "aws-lc-fips-sys", "version": sys_version},
            {"type": "library", "name": "aws-lc-sys", "version": "0.38.0"},
        ],
    }


class SbomContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.sys_version, self.reported = evidence.read_module_identity(REPO)

    def test_reads_the_manifest_pin_and_runtime_version(self) -> None:
        self.assertRegex(self.sys_version, r"\d+\.\d+\.\d+")
        self.assertTrue(self.reported.startswith("AWS-LC FIPS "))

    def test_accepts_a_candidate_that_keeps_inert_aws_lc_sys(self) -> None:
        bom = candidate_bom(self.sys_version)
        evidence.annotate_sbom(
            bom,
            self.sys_version,
            self.reported,
            subject_name="libsfodbc.so",
            subject_sha256="abc",
            tools=[("cargo-cyclonedx", "0.5.9")],
        )
        self.assertEqual(evidence.sbom_errors(bom, self.sys_version, self.reported), [])
        names = [component["name"] for component in bom["components"]]
        self.assertIn("aws-lc-sys", names)
        self.assertIn("AWS-LC FIPS", names)
        properties = {prop["name"]: prop["value"] for prop in bom["metadata"]["properties"]}
        self.assertEqual(properties["snowflake:subject-sha256"], "abc")
        self.assertEqual(properties["snowflake:stage"], "gha-candidate")
        self.assertIn("aws-lc-sys", json.dumps(bom))

    def test_rejects_a_wrong_module_pin(self) -> None:
        bom = candidate_bom(self.sys_version)
        evidence.annotate_sbom(
            bom,
            self.sys_version,
            self.reported,
            subject_name="libsfodbc.so",
            subject_sha256="abc",
            tools=[],
        )
        bom["components"][0]["version"] = "0.0.1"
        errors = evidence.sbom_errors(bom, self.sys_version, self.reported)
        self.assertTrue(any("aws-lc-fips-sys" in error for error in errors))

    def test_rejects_forbidden_crypto_crates(self) -> None:
        bom = candidate_bom(self.sys_version)
        evidence.annotate_sbom(
            bom,
            self.sys_version,
            self.reported,
            subject_name="libsfodbc.so",
            subject_sha256="abc",
            tools=[],
        )
        bom["components"].append({"type": "library", "name": "ring", "version": "0.17.14"})
        errors = evidence.sbom_errors(bom, self.sys_version, self.reported, {"ring"})
        self.assertTrue(any("ring" in error for error in errors))

    def test_accepts_ring_when_the_compile_tree_omits_it(self) -> None:
        bom = candidate_bom(self.sys_version)
        bom["components"].append({"type": "library", "name": "ring", "version": "0.17.14"})
        evidence.annotate_sbom(
            bom,
            self.sys_version,
            self.reported,
            subject_name="libsfodbc.so",
            subject_sha256="abc",
            tools=[],
            compiled={"odbc", "aws-lc-fips-sys"},
        )
        properties = {prop["name"]: prop["value"] for prop in bom["metadata"]["properties"]}
        self.assertIn("snowflake:ring", properties)
        self.assertEqual(evidence.sbom_errors(bom, self.sys_version, self.reported, {"odbc"}), [])

    def test_excluded_scope_does_not_count_as_the_runtime_module(self) -> None:
        bom = candidate_bom(self.sys_version)
        bom["components"][0]["scope"] = "excluded"
        errors = evidence.sbom_errors(bom, self.sys_version, self.reported)
        self.assertTrue(any("aws-lc-fips-sys" in error for error in errors))

    def test_cyclonedx_command_matches_the_fips_build_features(self) -> None:
        command = evidence.cyclonedx_command(
            Path("odbc/Cargo.toml"), "fips", "x86_64-unknown-linux-gnu",
        )
        self.assertIn("--all", command)
        self.assertIn("--features", command)
        self.assertIn("fips", command)
        self.assertNotIn("--all-features", command)
        self.assertNotIn("--top-level", command)
        self.assertIn("1.5", command)
        override_at = command.index("--override-filename")
        self.assertEqual(command[override_at + 1], "fips-candidate.cdx")

    def test_finds_the_cdx_name_cargo_cyclonedx_writes(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            manifest = root / "Cargo.toml"
            manifest.write_text("", encoding="utf-8")
            (root / "fips-candidate.json").write_text("{}", encoding="utf-8")
            with self.assertRaises(evidence.EvidenceError):
                evidence.generated_sbom(manifest)
            expected = root / "fips-candidate.cdx.json"
            expected.write_text("{}", encoding="utf-8")
            self.assertEqual(evidence.generated_sbom(manifest), expected)

    def test_discards_generated_sboms_outside_target(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            kept = root / "target" / "fips-candidate.cdx.json"
            kept.parent.mkdir()
            kept.write_text("{}", encoding="utf-8")
            generated = root / "odbc" / "fips-candidate.cdx.json"
            generated.parent.mkdir()
            generated.write_text("{}", encoding="utf-8")
            evidence.discard_generated_sboms(root)
            self.assertTrue(kept.is_file())
            self.assertFalse(generated.exists())


SYS_VERSION = "0.13.17"
CRYPTO_DLL = "aws_lc_fips_0_13_17_crypto.dll"
CRYPTO_DYLIB = "libaws_lc_fips_0_13_17_crypto.dylib"
TARGET = "x86_64-pc-windows-msvc"


def dumpbin_imports(*names: str) -> str:
    body = "".join(f"    {name}\n" for name in names)
    return f"Dump of file artifact.dll\n\n  Image has the following dependencies:\n\n{body}"


def otool_loads(*paths: str) -> str:
    body = "".join(f"\t{path} (compatibility version 1.0.0, current version 1.0.0)\n" for path in paths)
    return f"artifact:\n{body}"


class CompanionTests(unittest.TestCase):
    def test_requires_the_imported_windows_fips_crypto_dll(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            artifact = root / "sfodbc.dll"
            artifact.write_bytes(b"dll")
            with self.assertRaises(evidence.EvidenceError):
                evidence.windows_companions(artifact, root, "", SYS_VERSION, dumpbin_imports(CRYPTO_DLL))
            crypto = root / CRYPTO_DLL
            crypto.write_bytes(b"crypto")
            with self.assertRaises(evidence.EvidenceError):
                evidence.windows_companions(artifact, root, "", SYS_VERSION, dumpbin_imports("KERNEL32.dll"))
            found = evidence.windows_companions(artifact, root, "", SYS_VERSION, dumpbin_imports(CRYPTO_DLL))
            self.assertEqual(found, [crypto])

    def test_records_a_rust_wrapper_dll_when_the_build_emits_one(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            artifact = root / "sfodbc.dll"
            artifact.write_bytes(b"dll")
            crypto = root / CRYPTO_DLL
            crypto.write_bytes(b"crypto")
            wrapper = root / "aws_lc_fips_0_13_17_rust_wrapper.dll"
            wrapper.write_bytes(b"wrapper")
            self.assertEqual(
                evidence.windows_companions(artifact, root, "", SYS_VERSION, dumpbin_imports(CRYPTO_DLL)),
                [crypto, wrapper],
            )

    def test_finds_the_crypto_dll_beside_a_node_addon(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            addon_dir = root / "nodejs" / "_build" / "@snowflake" / "sdk-core"
            addon_dir.mkdir(parents=True)
            artifact = addon_dir / "sdk-core.win32-x64-msvc.node"
            artifact.write_bytes(b"node")
            deps = root / "target" / TARGET / "release" / "deps"
            deps.mkdir(parents=True)
            crypto = deps / CRYPTO_DLL
            crypto.write_bytes(b"crypto")
            found = evidence.windows_companions(
                artifact, root, TARGET, SYS_VERSION, dumpbin_imports(CRYPTO_DLL),
            )
            self.assertEqual(found, [crypto])
            self.assertEqual(
                evidence.repo_relative(crypto, root),
                f"target/{TARGET}/release/deps/{CRYPTO_DLL}",
            )

    def test_ignores_a_cached_crypto_dll_with_the_same_name(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            release = root / "target" / TARGET / "release"
            cached = release / "build" / "aws-lc-fips-sys-abc" / "out"
            cached.mkdir(parents=True)
            (cached / CRYPTO_DLL).write_bytes(b"stale")
            (release / "deps").mkdir()
            (release / "deps" / CRYPTO_DLL).write_bytes(b"deps")
            live = release / CRYPTO_DLL
            live.write_bytes(b"live")
            artifact = root / "nodejs" / "sdk-core.node"
            artifact.parent.mkdir(parents=True)
            artifact.write_bytes(b"node")
            found = evidence.windows_companions(
                artifact, root, TARGET, SYS_VERSION, dumpbin_imports(CRYPTO_DLL),
            )
            self.assertEqual(found, [live])

    def test_records_the_dylib_otool_names(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            release = root / "target" / "aarch64-apple-darwin" / "release"
            cached = release / "build" / "aws-lc-fips-sys-abc" / "out"
            cached.mkdir(parents=True)
            live = release / CRYPTO_DYLIB
            live.write_bytes(b"live")
            (cached / CRYPTO_DYLIB).write_bytes(b"stale")
            artifact = release / "libsfodbc.dylib"
            artifact.write_bytes(b"dylib")
            found = evidence.darwin_companions(
                artifact,
                root,
                "aarch64-apple-darwin",
                SYS_VERSION,
                otool_loads(str(live), "/usr/lib/libSystem.B.dylib"),
            )
            self.assertEqual(found, [live])

    def test_finds_the_module_emitted_by_the_build_script(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            out = root / "target" / TARGET / "release" / "build" / "aws-lc-fips-sys-abc" / "out"
            out.mkdir(parents=True)
            crypto = out / CRYPTO_DLL
            crypto.write_bytes(b"crypto")
            artifact = root / "sfodbc.dll"
            artifact.write_bytes(b"dll")
            found = evidence.windows_companions(
                artifact, root, TARGET, SYS_VERSION, dumpbin_imports(CRYPTO_DLL),
            )
            self.assertEqual(found, [crypto])

    def test_prefers_the_current_build_script_over_a_cached_copy(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            release = root / "target" / TARGET / "release" / "build"

            def emit(hash_name: str, mtime: int) -> Path:
                script_dir = release / hash_name
                script_dir.mkdir(parents=True)
                stamp = script_dir / "invoked.timestamp"
                stamp.write_text("ran", encoding="utf-8")
                os.utime(stamp, (mtime, mtime))
                out = script_dir / "out"
                out.mkdir()
                crypto = out / CRYPTO_DLL
                crypto.write_bytes(hash_name.encode())
                return crypto

            emit("aws-lc-fips-sys-old", 1_000)
            current = emit("aws-lc-fips-sys-new", 2_000)
            artifact = root / "sfodbc.dll"
            artifact.write_bytes(b"dll")
            found = evidence.windows_companions(
                artifact, root, TARGET, SYS_VERSION, dumpbin_imports(CRYPTO_DLL),
            )
            self.assertEqual(found, [current])

    def test_resolves_an_rpath_dylib_name(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            release = root / "target" / "aarch64-apple-darwin" / "release"
            release.mkdir(parents=True)
            live = release / CRYPTO_DYLIB
            live.write_bytes(b"live")
            artifact = root / "libsfodbc.dylib"
            artifact.write_bytes(b"dylib")
            found = evidence.darwin_companions(
                artifact,
                root,
                "aarch64-apple-darwin",
                SYS_VERSION,
                otool_loads(f"@rpath/{CRYPTO_DYLIB}"),
            )
            self.assertEqual(found, [live])

    def test_resolves_an_rpath_dylib_from_the_build_script(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            out = (
                root / "target" / "aarch64-apple-darwin" / "release"
                / "build" / "aws-lc-fips-sys-abc" / "out"
            )
            out.mkdir(parents=True)
            live = out / CRYPTO_DYLIB
            live.write_bytes(b"live")
            artifact = root / "libsfodbc.dylib"
            artifact.write_bytes(b"dylib")
            found = evidence.darwin_companions(
                artifact,
                root,
                "aarch64-apple-darwin",
                SYS_VERSION,
                otool_loads(f"@rpath/{CRYPTO_DYLIB}"),
            )
            self.assertEqual(found, [live])


class ToolPinTests(unittest.TestCase):
    def test_matches_the_cargo_cyclonedx_version_token(self) -> None:
        self.assertTrue(evidence.cyclonedx_pin_matches("cargo-cyclonedx 0.5.9", "0.5.9"))
        self.assertFalse(evidence.cyclonedx_pin_matches("cargo-cyclonedx 0.5.90", "0.5.9"))
        self.assertFalse(evidence.cyclonedx_pin_matches("cargo-cyclonedx 0.5.9.1", "0.5.9"))
        self.assertFalse(evidence.cyclonedx_pin_matches("cargo-cyclonedx 10.5.9", "0.5.9"))


if __name__ == "__main__":
    unittest.main()
