"""Unit tests for scripts/wait_for_test_host.sh.

Run with:
    python3 -m unittest scripts/test_wait_for_test_host.py -v
"""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "wait_for_test_host.sh"


class WaitForTestHostTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.counter = self.tmp / "attempts"
        self.counter.write_text("0")

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def _parameters(
        self,
        *,
        host: str | None = "account.gcp.snowflakecomputing.com",
        account: str | None = "account.gcp",
        password: str = "super-secret-password",
        extra: dict | None = None,
        filename: str = "parameters.json",
    ) -> Path:
        conn: dict = {}
        if host is not None:
            conn["SNOWFLAKE_TEST_HOST"] = host
        if account is not None:
            conn["SNOWFLAKE_TEST_ACCOUNT"] = account
        conn["SNOWFLAKE_TEST_PASSWORD"] = password
        if extra:
            conn.update(extra)
        path = self.tmp / filename
        path.write_text(json.dumps({"testconnection": conn}))
        return path

    def _lookup(self, outputs: list[int]) -> Path:
        """Fake resolver: N-th call exits with outputs[N]."""
        cases = "\n".join(f"  {i + 1}) exit {code} ;;" for i, code in enumerate(outputs))
        path = self.tmp / "lookup.sh"
        path.write_text(
            textwrap.dedent(
                """\
                #!/usr/bin/env bash
                n=$(( $(cat "{counter}") + 1 ))
                echo "$n" > "{counter}"
                case "$n" in
                {cases}
                  *) exit 1 ;;
                esac
                """
            ).format(counter=self.counter, cases=cases)
        )
        path.chmod(0o755)
        return path

    def _run(
        self,
        params: Path | None = None,
        *,
        lookup: Path | None = None,
        timeout: str = "5",
        interval: str = "0",
        extra_env: dict[str, str] | None = None,
        args: list[str] | None = None,
    ) -> subprocess.CompletedProcess:
        env = dict(
            os.environ,
            WAIT_FOR_TEST_HOST_TIMEOUT_SECONDS=timeout,
            WAIT_FOR_TEST_HOST_INTERVAL_SECONDS=interval,
        )
        if lookup is not None:
            env["WAIT_FOR_TEST_HOST_LOOKUP"] = str(lookup)
        if extra_env:
            env.update(extra_env)
        cmd = ["bash", str(SCRIPT)]
        if args is not None:
            cmd.extend(args)
        elif params is not None:
            cmd.append(str(params))
        return subprocess.run(
            cmd,
            env=env,
            capture_output=True,
            text=True,
            check=False,
            cwd=self.tmp,
        )

    def _attempts(self) -> int:
        return int(self.counter.read_text().strip())

    def test_missing_parameters_file_fails_without_lookup(self) -> None:
        result = self._run(self.tmp / "missing.json", lookup=self._lookup([0]))
        self.assertEqual(result.returncode, 2)
        self.assertEqual(self._attempts(), 0)
        self.assertIn("parameters file not found", result.stderr)

    def test_empty_host_and_account_fails_without_lookup(self) -> None:
        params = self._parameters(host="", account="")
        result = self._run(params, lookup=self._lookup([0]))
        self.assertEqual(result.returncode, 2)
        self.assertEqual(self._attempts(), 0)
        self.assertIn("both empty", result.stderr)

    def test_whitespace_host_falls_back_to_account_derived_hostname(self) -> None:
        params = self._parameters(host="  ", account="myacct.us-central1.gcp")
        result = self._run(params, lookup=self._lookup([0]))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(
            "myacct.us-central1.gcp.snowflakecomputing.com",
            result.stdout + result.stderr,
        )

    def test_explicit_host_wins_over_account(self) -> None:
        params = self._parameters(
            host="explicit.snowflakecomputing.com",
            account="ignored.gcp",
        )
        result = self._run(params, lookup=self._lookup([0]))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        combined = result.stdout + result.stderr
        self.assertIn("explicit.snowflakecomputing.com", combined)
        self.assertNotIn("ignored.gcp.snowflakecomputing.com", combined)

    def test_resolves_on_first_attempt(self) -> None:
        params = self._parameters()
        result = self._run(params, lookup=self._lookup([0]))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self._attempts(), 1)
        self.assertIn("resolved", result.stdout)

    def test_retries_until_lookup_succeeds(self) -> None:
        params = self._parameters()
        result = self._run(params, lookup=self._lookup([1, 1, 0]))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self._attempts(), 3)

    def test_exhausted_window_fails_with_dns_message(self) -> None:
        params = self._parameters(host="ghost.example.invalid")
        result = self._run(
            params,
            lookup=self._lookup([1, 1, 1]),
            timeout="0",
        )
        self.assertEqual(result.returncode, 1)
        self.assertGreaterEqual(self._attempts(), 1)
        combined = result.stdout + result.stderr
        self.assertIn("runner DNS did not resolve test host", combined)
        self.assertIn("ghost.example.invalid", combined)
        self.assertIn("runner DNS failure, not a Snowflake account outage", combined)

    def test_default_lookup_resolves_localhost(self) -> None:
        params = self._parameters(host="localhost")
        result = self._run(params, lookup=None, timeout="5")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("resolved localhost", result.stdout)

    def test_does_not_print_password(self) -> None:
        params = self._parameters(password="super-secret-password")
        result = self._run(params, lookup=self._lookup([0]))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertNotIn("super-secret-password", result.stdout)
        self.assertNotIn("super-secret-password", result.stderr)

    def test_parameter_path_env_used_when_no_arg(self) -> None:
        params = self._parameters(filename="from-env.json")
        result = self._run(
            lookup=self._lookup([0]),
            extra_env={"PARAMETER_PATH": str(params)},
            args=[],
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self._attempts(), 1)


if __name__ == "__main__":
    unittest.main()
