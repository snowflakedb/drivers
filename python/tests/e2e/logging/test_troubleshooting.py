import os
import subprocess
import sys
import tempfile
import textwrap

import pytest

from tests.e2e.session.test_close import _assert_subprocess_ok


# Spawn a child Python so env vars are visible before snowflake.connector import triggers sf_core_python.init.
_DEBUG_LOGIN_MARKER = "Login successful, extracting session tokens"
_WORKER_CODE = textwrap.dedent("""\
    import logging

    logging.disable(logging.CRITICAL)

    from tests.connector_factory import ConnectorFactory, create_connection_with_adapter

    with create_connection_with_adapter(ConnectorFactory.create_adapter()) as conn:
        with conn.cursor() as cur:
            cur.execute("SELECT 1")
            cur.fetchone()
""")

_ASYNC_WORKER_CODE = textwrap.dedent("""\
    import asyncio
    import logging

    logging.disable(logging.CRITICAL)

    from snowflake.connector import aio
    from tests.config import get_test_parameters
    from tests.connector_factory import setup_default_jwt_auth

    params = get_test_parameters()
    connection_params = {
        "account": params.get("SNOWFLAKE_TEST_ACCOUNT"),
        "user": params.get("SNOWFLAKE_TEST_USER"),
        "database": params.get("SNOWFLAKE_TEST_DATABASE"),
        "schema": params.get("SNOWFLAKE_TEST_SCHEMA"),
        "warehouse": params.get("SNOWFLAKE_TEST_WAREHOUSE_PYTHON") or params.get("SNOWFLAKE_TEST_WAREHOUSE"),
        "role": params.get("SNOWFLAKE_TEST_ROLE"),
    }
    if params.get("SNOWFLAKE_TEST_IS_USUT"):
        connection_params["password"] = params.get("SNOWFLAKE_TEST_PASSWORD")
    else:
        setup_default_jwt_auth(connection_params)
    if params.get("SNOWFLAKE_TEST_SERVER_URL"):
        connection_params["server_url"] = params["SNOWFLAKE_TEST_SERVER_URL"]
    if params.get("SNOWFLAKE_TEST_HOST"):
        connection_params["host"] = params["SNOWFLAKE_TEST_HOST"]
    if params.get("SNOWFLAKE_TEST_PORT"):
        connection_params["port"] = params["SNOWFLAKE_TEST_PORT"]
    if params.get("SNOWFLAKE_TEST_PROTOCOL"):
        connection_params["protocol"] = params["SNOWFLAKE_TEST_PROTOCOL"]
    connection_params = {k: v for k, v in connection_params.items() if v is not None}

    async def _run():
        async with aio.connect(**connection_params) as conn:
            cur = conn.cursor()
            await cur.execute("SELECT 1")
            await cur.fetchone()

    asyncio.run(_run())
""")


def _run_troubleshooting_worker(log_dir: str, worker_code: str = _WORKER_CODE) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["SNOWFLAKE_TROUBLESHOOTING_ENABLED"] = "true"
    env["SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH"] = log_dir
    return subprocess.run(
        [sys.executable, "-c", worker_code],
        env=env,
        capture_output=True,
        text=True,
        timeout=120,
    )


@pytest.mark.skip_reference(reason="SNOWFLAKE_TROUBLESHOOTING_ENABLED is universal-driver only")
class TestTroubleshootingMode:
    def test_should_create_troubleshooting_log_file_when_enabled_via_environment_variable(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            # Given SNOWFLAKE_TROUBLESHOOTING_ENABLED is set to "true"
            # and SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH points to a temporary directory
            pass

            # When a connection is established and a query is executed
            result = _run_troubleshooting_worker(tmp_dir)
            _assert_subprocess_ok(result)

            # Then a troubleshooting log file exists in the configured directory
            log_file_path = os.path.join(tmp_dir, "sf_driver_troubleshooting.log")
            assert os.path.isfile(log_file_path), (
                f"Expected sf_driver_troubleshooting.log in {tmp_dir}, found: {os.listdir(tmp_dir)}"
            )

            # And the log file contains debug-level entries below the configured log level
            with open(log_file_path) as f:
                contents = f.read()
            assert len(contents) > 0, "Troubleshooting log file is empty"
            assert _DEBUG_LOGIN_MARKER in contents, (
                f"Expected debug-level login event in troubleshooting log, got: {contents[:500]}"
            )

    def test_should_create_troubleshooting_log_file_when_async_connection_is_enabled_via_environment_variable(
        self,
    ):
        with tempfile.TemporaryDirectory() as tmp_dir:
            # Given SNOWFLAKE_TROUBLESHOOTING_ENABLED is set to "true"
            # and SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH points to a temporary directory
            pass

            # When an async connection is established and a query is executed
            result = _run_troubleshooting_worker(tmp_dir, _ASYNC_WORKER_CODE)
            _assert_subprocess_ok(result)

            # Then a troubleshooting log file exists in the configured directory
            log_file_path = os.path.join(tmp_dir, "sf_driver_troubleshooting.log")
            assert os.path.isfile(log_file_path), (
                f"Expected sf_driver_troubleshooting.log in {tmp_dir}, found: {os.listdir(tmp_dir)}"
            )

            # And the log file contains debug-level entries below the configured log level
            with open(log_file_path) as f:
                contents = f.read()
            assert len(contents) > 0, "Troubleshooting log file is empty"
            assert _DEBUG_LOGIN_MARKER in contents, (
                f"Expected debug-level login event in troubleshooting log, got: {contents[:500]}"
            )
