"""Run a pytest selection inside a Snowflake stored procedure.

Stages the connector wheel(s), an optional snowpark wheel, a zip of ``tests/``
and ``sproc_handler.py`` into a throwaway schema, creates a Python stored
procedure from them and calls it. The procedure's pytest log is printed here and
its JUnit report is written to ``reports/stored_proc.xml``.

Usage, from ``python/`` with the connector installed and ``parameters.json``
available::

    SPROC_CONNECTOR_WHEEL=dist/snowflake_connector_python-...-manylinux_2_28_x86_64.whl \\
    SPROC_SNOWPARK_WHEEL=dist/snowflake_snowpark_python-...-py3-none-any.whl \\
        python -m tests.stored_proc.run_stored_proc [pytest args...]

``SPROC_CONNECTOR_WHEEL`` takes one wheel per architecture, separated by
``os.pathsep``; the procedure picks the one matching its sandbox. All of them
must target the same CPython version, which becomes the procedure's
``RUNTIME_VERSION``. Pytest args are paths relative to ``python/``, taken from
the command line or, when there are none, from ``SPROC_PYTEST_ARGS``; with
neither, ``DEFAULT_PYTEST_ARGS`` runs.
"""

from __future__ import annotations

import json
import os
import re
import shlex
import sys
import tempfile
import uuid
import zipfile

from pathlib import Path

from tests.config import IN_STORED_PROC_ENV_VAR, get_test_parameters
from tests.connector_factory import ConnectorFactory, create_connection_with_adapter

from . import sproc_handler


PYTHON_DIR = Path(__file__).resolve().parents[2]
REPORT_PATH = PYTHON_DIR / "reports" / "stored_proc.xml"
STAGE = "DRIVER_SPROC_PAYLOAD"
PROCEDURE = "RUN_DRIVER_TESTS"
UNIVERSAL_DRIVER_BACKEND_PARAMETER = "ENABLE_UNIVERSAL_DRIVER_BACKEND"
DEFAULT_PYTEST_ARGS = ["tests/e2e/stored_proc"]
# Runtime dependencies of the connector and of the tests, resolved by the
# sandbox from Snowflake's Anaconda channel. snowpark provides the handler's
# session; the staged wheels shadow it and the connector it pulls in.
PACKAGES = (
    "snowflake-snowpark-python",
    "pytest",
    "pytest-xdist",
    "cryptography",
    "protobuf",
    "pytz",
    "tomlkit",
    "requests",
)
# Forwarded into the procedure so the tests pick the same database objects and
# result format as the job that launched them.
FORWARDED_TEST_PARAMETERS = (
    "SNOWFLAKE_TEST_DATABASE",
    "SNOWFLAKE_TEST_SCHEMA",
    "SNOWFLAKE_TEST_WAREHOUSE",
    "SNOWFLAKE_TEST_WAREHOUSE_PYTHON",
    "SNOWFLAKE_TEST_ROLE",
)
FORWARDED_ENV_VARS = ("QUERY_RESULT_FORMAT", "SNOWFLAKE_TEST_USE_CORE_ARROW")
_CPYTHON_TAG = re.compile(r"-cp(\d)(\d+)-")
# snowflake_connector_python-<version>-<python tag>-...
_WHEEL_VERSION = re.compile(r"^snowflake_connector_python-([^-]+)-")


def main(pytest_args: list[str]) -> int:
    connector_wheels = [Path(p) for p in _required_env("SPROC_CONNECTOR_WHEEL").split(os.pathsep) if p]
    snowpark_wheel = os.environ.get("SPROC_SNOWPARK_WHEEL")
    wheels = connector_wheels + ([Path(snowpark_wheel)] if snowpark_wheel else [])
    runtime_version = _runtime_version(connector_wheels)
    pytest_args = pytest_args or shlex.split(os.environ.get("SPROC_PYTEST_ARGS", "")) or DEFAULT_PYTEST_ARGS
    # tests/e2e/stored_proc/conftest.py fails every test unless the procedure
    # imported this version, i.e. the staged wheel and not the sandbox's own.
    pytest_args = [*pytest_args, f"--expected-connector-version={_connector_version(connector_wheels)}"]

    adapter = ConnectorFactory.create_adapter()
    with tempfile.TemporaryDirectory() as tmp, create_connection_with_adapter(adapter) as conn:
        tests_zip = _zip_tests(Path(tmp) / sproc_handler.TESTS_ZIP)
        cur = conn.cursor()
        # The procedure runs in this session (EXECUTE AS CALLER), so it inherits the setting.
        cur.execute(f"ALTER SESSION SET {UNIVERSAL_DRIVER_BACKEND_PARAMETER} = True")
        database = cur.execute("SELECT CURRENT_DATABASE()").fetchone()[0]
        schema = f"{database}.DRIVER_SPROC_{uuid.uuid4().hex.upper()}"
        stage = f"{schema}.{STAGE}"
        procedure = f"{schema}.{PROCEDURE}"
        cur.execute("CREATE SCHEMA IDENTIFIER(?)", (schema,), _force_qmark_paramstyle=True)
        try:
            cur.execute("CREATE STAGE IDENTIFIER(?)", (stage,), _force_qmark_paramstyle=True)
            for path in [*wheels, tests_zip, Path(sproc_handler.__file__)]:
                print(f"Uploading {path.name}")
                # PUT cannot bind its stage or file path. Both are interpolated on purpose: the stage
                # name is CURRENT_DATABASE() plus a uuid hex, and the path is a local file this script created.
                cur.execute(f"PUT 'file://{path.resolve().as_posix()}' @{stage} AUTO_COMPRESS=FALSE OVERWRITE=TRUE")
            cur.execute(_create_procedure_sql(procedure, stage, runtime_version))
            print(f"Running {pytest_args} in {procedure} (Python {runtime_version})")
            cur.execute(
                f"CALL {procedure}(%s, PARSE_JSON(%s)::ARRAY, PARSE_JSON(%s)::ARRAY, PARSE_JSON(%s)::OBJECT)",
                (stage, json.dumps([w.name for w in wheels]), json.dumps(pytest_args), json.dumps(_sproc_env())),
            )
            result = json.loads(cur.fetchone()[0])
        finally:
            cur.execute("DROP SCHEMA IF EXISTS IDENTIFIER(?) CASCADE", (schema,), _force_qmark_paramstyle=True)

    print(result.pop("log"))
    junit = result.pop("junit")
    if junit:
        REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
        REPORT_PATH.write_text(junit)
    print(json.dumps(result, indent=2))
    return result["exit_code"]


def _required_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"{name} must be set")
    return value


def _runtime_version(connector_wheels: list[Path]) -> str:
    versions = set()
    for wheel in connector_wheels:
        match = _CPYTHON_TAG.search(wheel.name)
        if not match:
            raise SystemExit(f"{wheel.name} has no CPython tag")
        versions.add(f"{match[1]}.{match[2]}")
    if len(versions) != 1:
        raise SystemExit(f"Connector wheels must target one CPython version, got {sorted(versions)}")
    return versions.pop()


def _connector_version(connector_wheels: list[Path]) -> str:
    versions = set()
    for wheel in connector_wheels:
        match = _WHEEL_VERSION.match(wheel.name)
        if not match:
            raise SystemExit(f"{wheel.name} is not a snowflake_connector_python wheel")
        versions.add(match[1])
    if len(versions) != 1:
        raise SystemExit(f"Connector wheels must share one version, got {sorted(versions)}")
    return versions.pop()


def _zip_tests(target: Path) -> Path:
    tests_dir = PYTHON_DIR / "tests"
    with zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as zf:
        for path in sorted(tests_dir.rglob("*")):
            if path.is_file() and "__pycache__" not in path.parts:
                zf.write(path, path.relative_to(PYTHON_DIR).as_posix())
    return target


def _sproc_env() -> dict[str, str]:
    params = get_test_parameters()
    env = {key: str(params[key]) for key in FORWARDED_TEST_PARAMETERS if params.get(key)}
    env.update({key: os.environ[key] for key in FORWARDED_ENV_VARS if key in os.environ})
    env[IN_STORED_PROC_ENV_VAR] = "1"
    return env


def _create_procedure_sql(procedure: str, stage: str, runtime_version: str) -> str:
    packages = ", ".join(f"'{p}'" for p in PACKAGES)
    handler_module = Path(sproc_handler.__file__).stem
    return f"""
        CREATE OR REPLACE PROCEDURE {procedure}(STAGE STRING, WHEELS ARRAY, PYTEST_ARGS ARRAY, ENV OBJECT)
        RETURNS VARIANT
        LANGUAGE PYTHON
        RUNTIME_VERSION = '{runtime_version}'
        PACKAGES = ({packages})
        IMPORTS = ('@{stage}/{handler_module}.py')
        HANDLER = '{handler_module}.run'
        EXECUTE AS CALLER
    """


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
