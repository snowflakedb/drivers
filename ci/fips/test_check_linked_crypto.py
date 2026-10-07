"""Run with: python -m unittest discover -s ci/fips -v."""

from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("check_linked_crypto.py")


@unittest.skipUnless(sys.platform == "linux" and shutil.which("cc"), "requires ELF cc/nm")
class ArtifactInspectionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.lockfile = self.root / "Cargo.lock"
        self.lockfile.write_text(
            '[[package]]\nname = "aws-lc-fips-sys"\nversion = "0.13.17"\n',
            encoding="utf-8",
        )

    def build_artifact(self, name: str, symbols: list[str]) -> Path:
        artifact = self.root / f"{name}.so"
        source = "\n".join(f"void {symbol}(void) {{}}" for symbol in symbols)
        subprocess.run(
            ["cc", "-shared", "-fPIC", "-x", "c", "-o", str(artifact), "-"],
            input=source, text=True, capture_output=True, check=True,
        )
        return artifact

    def inspect(self, *artifacts: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--lockfile", str(self.lockfile),
             *(str(artifact) for artifact in artifacts)],
            text=True, capture_output=True,
        )

    def test_accepts_the_locked_fips_module(self) -> None:
        artifact = self.build_artifact("fips", ["aws_lc_fips_0_13_17_EVP_EncryptInit_ex"])
        result = self.inspect(artifact)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_rejects_standard_aws_lc(self) -> None:
        artifact = self.build_artifact("standard", ["aws_lc_0_38_0_EVP_EncryptInit_ex"])
        result = self.inspect(artifact)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("aws_lc_0_38_0_EVP_EncryptInit_ex", result.stderr)
        self.assertIn("aws_lc_fips_0_13_17_", result.stderr)

    def test_rejects_each_other_crypto_backend_even_with_fips_present(self) -> None:
        for symbol in (
            "aws_lc_0_38_0_SHA256", "ring_core_0_17_14_SHA256",
            "EVP_EncryptInit_ex", "SSL_CTX_new", "SHA256", "d2i_X509",
            "EVP_DigestInit_ex", "OpenSSL_version_num", "OPENSSL_init_crypto", "SSL_new", "RSA_sign",
        ):
            with self.subTest(symbol=symbol):
                artifact = self.build_artifact(
                    "mixed", ["aws_lc_fips_0_13_17_SHA256", symbol],
                )
                result = self.inspect(artifact)
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn(symbol, result.stderr)

    def test_allows_private_aws_lc_helpers_without_exempting_public_apis(self) -> None:
        artifact = self.build_artifact(
            "private-helpers",
            ["aws_lc_fips_0_13_17_SHA256", "BN_from_montgomery_word",
             "EVP_MD_pctx_ops_init", "OPENSSL_memcpy", "RSA_generate_key_ex_maybe_fips"],
        )
        result = self.inspect(artifact)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_rejects_dynamic_openssl_even_without_recognized_api_symbols(self) -> None:
        self.build_artifact("libcrypto", ["some_dynamic_symbol"])
        artifact = self.root / "dynamic.so"
        subprocess.run(
            ["cc", "-shared", "-fPIC", "-x", "c", "-", "-o", str(artifact),
             "-L", str(self.root), "-Wl,--no-as-needed", "-lcrypto"],
            input="void aws_lc_fips_0_13_17_SHA256(void) {}", text=True,
            capture_output=True, check=True,
        )
        result = self.inspect(artifact)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("libcrypto.so", result.stderr)

    def test_uses_the_lockfile_version_not_a_fixed_prefix(self) -> None:
        self.lockfile.write_text(
            '[[package]]\nname = "aws-lc-fips-sys"\nversion = "0.14.2"\n',
            encoding="utf-8",
        )
        artifact = self.build_artifact("new-fips", ["aws_lc_fips_0_14_2_SHA256"])
        self.assertEqual(self.inspect(artifact).returncode, 0)
        old = self.build_artifact("old-fips", ["aws_lc_fips_0_13_17_SHA256"])
        self.assertEqual(self.inspect(old).returncode, 1)

    def test_rejects_missing_fips_definitions(self) -> None:
        artifact = self.build_artifact("no-crypto", ["not_a_crypto_module"])
        result = self.inspect(artifact)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("aws_lc_fips_0_13_17_", result.stderr)

    def test_checks_every_artifact(self) -> None:
        fips = self.build_artifact("fips", ["aws_lc_fips_0_13_17_SHA256"])
        standard = self.build_artifact("standard", ["aws_lc_0_38_0_SHA256"])
        result = self.inspect(fips, standard)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn(str(fips), result.stdout)
        self.assertIn(str(standard), result.stderr)

    def test_caps_offending_symbol_diagnostics(self) -> None:
        symbols = [f"ring_core_bad_{index:03}" for index in range(100)]
        artifact = self.build_artifact("many", ["aws_lc_fips_0_13_17_SHA256", *symbols])
        result = self.inspect(artifact)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn(symbols[0], result.stderr)
        self.assertNotIn(symbols[-1], result.stderr)
        self.assertIn("more", result.stderr)


class SymbolParsingTests(unittest.TestCase):
    def test_codegen_metadata_is_not_a_linked_crypto_module(self) -> None:
        import check_linked_crypto as check

        metadata = "0000000000000000 a aws_lc_fips_sys.83f72be9e802c948-cgu.0\n"
        symbols = check.nm_symbols(metadata + "0000000000000010 t aws_lc_fips_0_13_17_SHA256\n")
        self.assertEqual(check.symbol_errors(symbols, "aws_lc_fips_0_13_17_"), [])
        file_only = check.nm_symbols("0000000000000000 a aws_lc_fips_0_13_17_file.c\n")
        self.assertTrue(check.symbol_errors(file_only, "aws_lc_fips_0_13_17_"))

    def test_macos_symbols_and_undefined_fips_are_distinguished(self) -> None:
        import check_linked_crypto as check

        symbols = check.nm_symbols(
            "0000000100000010 t _aws_lc_fips_0_13_17_SHA256\n"
            "                 U _EVP_EncryptInit_ex\n"
        )
        errors = check.symbol_errors(symbols, "aws_lc_fips_0_13_17_")
        self.assertEqual(len(errors), 1)
        self.assertIn("EVP_EncryptInit_ex", errors[0])
        undefined = check.nm_symbols("                 U aws_lc_fips_0_13_17_SHA256\n")
        self.assertTrue(check.symbol_errors(undefined, "aws_lc_fips_0_13_17_"))

    def test_windows_requires_exact_fips_dll_and_scans_linker_map(self) -> None:
        import check_linked_crypto as check

        dependencies = (
            "  Image has the following dependencies:\n\n"
            "    aws_lc_fips_0_13_17_crypto.dll\n    KERNEL32.dll\n"
        )
        linker_map = (
            "  Address         Publics by Value              Rva+Base       Lib:Object\n"
            " 0001:00000000 SFGetTlsStatus                   0000000180001000 f driver.obj\n"
            " 0002:00000000 __imp_aws_lc_fips_0_13_17_SHA256  0000000180002000   crypto.dll\n"
        )
        self.assertEqual(
            check.windows_errors(dependencies, linker_map, "aws_lc_fips_0_13_17_"), [],
        )
        for symbol, normalized in (
            ("aws_lc_0_38_0_SHA256", "aws_lc_0_38_0_SHA256"),
            ("_ring_core_bad", "ring_core_bad"),
            ("__imp_EVP_EncryptInit_ex", "EVP_EncryptInit_ex"),
        ):
            with self.subTest(symbol=symbol):
                contaminated = linker_map + f" 0001:00000010 {symbol} 0000000180001010 f crypto.obj\n"
                errors = check.windows_errors(
                    dependencies, contaminated, "aws_lc_fips_0_13_17_",
                )
                self.assertTrue(errors)
                self.assertIn(normalized, " ".join(errors))
        self.assertTrue(check.windows_errors("    KERNEL32.dll\n", linker_map, "aws_lc_fips_0_13_17_"))
        self.assertTrue(check.windows_errors(dependencies, "", "aws_lc_fips_0_13_17_"))
        self.assertTrue(check.windows_errors(dependencies, linker_map, "aws_lc_fips_0_14_2_"))

    def test_windows_rejects_openssl_dll_imports(self) -> None:
        import check_linked_crypto as check

        linker_map = " 0001:00000000 SFGetTlsStatus 0000000180001000 f driver.obj\n"
        for dll in ("libssl-3-x64.dll", "LIBCRYPTO-3-X64.DLL", "libeay32.dll"):
            with self.subTest(dll=dll):
                errors = check.windows_errors(
                    f"    aws_lc_fips_0_13_17_crypto.dll\n    {dll}\n",
                    linker_map, "aws_lc_fips_0_13_17_",
                )
                self.assertTrue(errors)
                self.assertIn(dll.lower(), " ".join(errors))

    def test_macos_dylib_dependencies_reject_openssl_not_system_crypto(self) -> None:
        import check_linked_crypto as check

        system = (
            "libsfodbc.dylib:\n"
            "\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1.0.0)\n"
            "\t/usr/lib/system/libcorecrypto.dylib (compatibility version 1.0.0, current version 1.0.0)\n"
        )
        self.assertEqual(check.unix_library_errors(system, "darwin"), [])
        errors = check.unix_library_errors(
            system + "\t@rpath/libssl.3.dylib (compatibility version 3.0.0, current version 3.0.0)\n",
            "darwin",
        )
        self.assertEqual(len(errors), 1)
        self.assertIn("libssl.3.dylib", errors[0])


if __name__ == "__main__":
    unittest.main()
