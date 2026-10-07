"""Exercise the ctypes ABI and reject artifacts that do not report FIPS TLS."""

from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


@unittest.skipUnless(sys.platform == "linux" and shutil.which("cc"), "requires ELF cc")
class OdbcStatusTests(unittest.TestCase):
    def test_requires_success_and_both_fips_fields(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            artifact = Path(directory) / "status.so"
            for result, provider, feature, expected in (
                (0, 1, 1, 0), (-1, 1, 1, 1), (0, 0, 1, 1), (0, 1, 0, 1), (0, 2, 1, 1),
            ):
                with self.subTest(result=result, provider=provider, feature=feature):
                    source = f"""
                        #include <stdint.h>
                        struct Status {{ uint32_t provider; uint32_t feature; }};
                        int32_t SFGetTlsStatus(struct Status *status) {{
                            status->provider = {provider};
                            status->feature = {feature};
                            return {result};
                        }}
                    """
                    subprocess.run(
                        ["cc", "-shared", "-fPIC", "-x", "c", "-o", str(artifact), "-"],
                        input=source, text=True, capture_output=True, check=True,
                    )
                    checked = subprocess.run(
                        [sys.executable, str(Path(__file__).with_name("check_odbc_tls_status.py")),
                         str(artifact)], text=True, capture_output=True,
                    )
                    self.assertEqual(checked.returncode, expected, checked.stderr)


if __name__ == "__main__":
    unittest.main()
