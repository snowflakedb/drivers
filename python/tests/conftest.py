"""
pytest configuration and fixtures for PEP 249 tests.
"""

from __future__ import annotations

import os
import sys

from typing import Any
from urllib.parse import urlparse

import pytest

from snowflake.connector.constants import ENV_VAR_PARTNER
from snowflake.connector.cursor import DictCursor

from .compatibility import IS_UNIVERSAL_DRIVER, native_arrow_enabled
from .connector_factory import ConnectorFactory, create_connection_with_adapter
from .private_key_helper import get_test_private_key_path
from .wiremock_client import WiremockClient


# Type alias for a single row returned from cursor
Row = tuple[Any, ...]

_PARTNER_MODULES = ("streamlit", "ipykernel", "jupyter_core", "jupyter_client", "snowbooks")


@pytest.fixture
def isolate_application_detection(monkeypatch):
    """Clear partner env/modules so application detection starts from a blank slate."""
    monkeypatch.delenv(ENV_VAR_PARTNER, raising=False)
    for name in _PARTNER_MODULES:
        monkeypatch.delitem(sys.modules, name, raising=False)
    return monkeypatch


def pytest_configure(config):
    # scripts/ is not a Python package (no __init__.py, not on sys.path), so
    # load setup_local_reg.py by file path via importlib. bootstrap() is a
    # no-op unless SNOWFLAKE_TEST_HOST points at a *.reg.local instance.
    #
    # Unit tests must not require parameters.json: when the file is absent
    # we simply skip the bootstrap step — only integ / e2e suites need
    # credentials, and those will fail with a clearer error downstream.
    import importlib.util
    import pathlib

    config.addinivalue_line(
        "markers",
        "async_cursor_parity: also run this test against the async cursor (see run_against_sync_and_async)",
    )
    config.addinivalue_line(
        "markers",
        "async_connection_parity: also run this test against the aio Connection "
        "(see run_against_sync_and_async_connection)",
    )
    config.addinivalue_line(
        "markers",
        "skip_async(reason): skip this test only when run against the async cursor backend",
    )
    config.addinivalue_line(
        "markers",
        "skip_async_connection(reason): skip this test only when run against the async connection backend",
    )

    repo_root = pathlib.Path(__file__).resolve().parents[2]
    param_path = pathlib.Path(os.environ.get("PARAMETER_PATH", repo_root / "parameters.json"))
    if not param_path.is_file():
        return

    spec = importlib.util.spec_from_file_location("setup_local_reg", repo_root / "scripts" / "setup_local_reg.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.bootstrap(parameters_path=param_path)


def pytest_xdist_auto_num_workers(config):
    return os.cpu_count() or 1


@pytest.hookimpl(optionalhook=True)
def pytest_metadata(metadata):
    metadata["Version of snowflake.connector"] = "Universal" if IS_UNIVERSAL_DRIVER else "Old"


@pytest.fixture(scope="session")
def connector_adapter(request):
    return ConnectorFactory.create_adapter()


@pytest.fixture(scope="module")
def cursor_backend(request):
    """Select the cursor implementation under test: ``"sync"`` (default) or ``"async"``.

    Parametrized indirectly by :data:`run_against_sync_and_async`; tests that
    don't opt in fall back to the synchronous cursor. When ``"async"``, the
    ``connection``/``function_connection`` fixtures hand out a connection whose
    cursors run the async implementation behind a blocking facade, so existing
    synchronous tests exercise the async cursor unchanged.
    """
    return getattr(request, "param", "sync")


@pytest.fixture(scope="module")
def connection_backend(request):
    """Select the connection implementation under test: ``"sync"`` (default) or ``"async"``.

    Parametrized indirectly by :data:`run_against_sync_and_async_connection`.
    When ``"async"``, connection fixtures hand out an :class:`~snowflake.connector.aio.Connection`
    behind :class:`BlockingConnection` so synchronous tests exercise the async
    connection unchanged.
    """
    return getattr(request, "param", "sync")


def pytest_generate_tests(metafunc):
    """Parametrize async backends for tests that opt in via parity markers.

    Only tests whose fixture closure includes the backend fixture get the
    ``["sync", "async"]`` parametrization; marked tests that touch no connection
    simply run once unparametrized, so the marker is safe on a whole module.

    Async parametrization is universal-driver only: the reference connector has
    no ``aio`` package and the blocking async bridge lives under ``tests/``.
    """
    if not IS_UNIVERSAL_DRIVER:
        return
    if "cursor_backend" in metafunc.fixturenames:
        if metafunc.definition.get_closest_marker("async_cursor_parity") is not None:
            metafunc.parametrize("cursor_backend", ["sync", "async"], indirect=True)
    if "connection_backend" in metafunc.fixturenames:
        if metafunc.definition.get_closest_marker("async_connection_parity") is not None:
            metafunc.parametrize("connection_backend", ["sync", "async"], indirect=True)


def _wrap_for_async_backends(
    connection: Any,
    *,
    connection_backend: str,
    cursor_backend: str,
) -> Any:
    """Wrap *connection* when either async backend is selected (universal driver only)."""
    if not IS_UNIVERSAL_DRIVER:
        return connection
    if connection_backend != "async" and cursor_backend != "async":
        return connection
    from ._async_bridge import maybe_blocking_async_connection

    return maybe_blocking_async_connection(connection)


# Apply as a class decorator or module-level ``pytestmark`` to run a test
# (class/module) against BOTH the sync and the async cursor implementations.
# The actual parametrization is performed in ``pytest_generate_tests`` and only
# applies to tests that pull in a connection/cursor fixture (and therefore the
# ``cursor_backend`` fixture); marked tests that use neither simply run once.
run_against_sync_and_async = pytest.mark.async_cursor_parity

# Apply as a class decorator or module-level ``pytestmark`` to run connection
# integration tests against BOTH sync :class:`Connection` and aio :class:`Connection`.
run_against_sync_and_async_connection = pytest.mark.async_connection_parity


def skip_async(reason: str):
    """Mark a test to be skipped only under the async cursor backend.

    Use for tests that assert against a concrete sync cursor type
    (e.g. ``isinstance(cur, SnowflakeCursor)``) which cannot hold for the
    blocking async facade.
    """
    return pytest.mark.skip_async(reason=reason)


def skip_async_connection(reason: str):
    """Mark a test to be skipped only under the async connection backend.

    Use for tests that assert on sync-only connection internals (e.g.
    ``connection._autocommit``) which do not exist on the aio :class:`~snowflake.connector.aio.Connection`.
    """
    return pytest.mark.skip_async_connection(reason=reason)


def with_paramstyle(style: str):
    """Decorator that sets the paramstyle on the ``connection`` fixture.

    Usage::

        @with_paramstyle("qmark")
        class TestBinding:
            def test_example(self, cursor): ...


        @with_paramstyle("numeric")
        def test_numeric(self, cursor): ...
    """
    return pytest.mark.parametrize("connection", [style], indirect=True)


def with_paramstyles(*styles: str):
    """Like ``with_paramstyle``, but run the decorated tests for multiple paramstyles."""
    return pytest.mark.parametrize("connection", list(styles), indirect=True)


@pytest.fixture(scope="module")
def connection(request, connector_adapter, connection_backend, cursor_backend):
    """Module-scoped test connection; shared across tests in the same module.

    Use ``@with_paramstyle(...)`` to enable parameter binding. When a paramstyle
    is supplied via indirect parametrize, a distinct instance is created per
    paramstyle so tests with different paramstyles don't collide.

    When ``cursor_backend == "async"`` the connection is wrapped so its cursors
    run the async cursor implementation (see ``run_against_sync_and_async``).
    When ``connection_backend == "async"`` the connection itself is an
    aio :class:`~snowflake.connector.aio.Connection` behind :class:`BlockingConnection`.

    Tests that mutate connection state (close, autocommit, commit/rollback,
    set_autocommit, etc.) must use ``function_connection`` instead — this
    fixture is reused across tests in a module and must remain untouched.
    """
    paramstyle = getattr(request, "param", None)
    conn = create_connection_with_adapter(connector_adapter, paramstyle=paramstyle)
    wrapped = _wrap_for_async_backends(
        conn,
        connection_backend=connection_backend,
        cursor_backend=cursor_backend,
    )
    try:
        yield wrapped
    finally:
        if not wrapped.is_closed():
            wrapped.close()


@pytest.fixture
def function_connection(connector_adapter, connection_backend, cursor_backend):
    """Function-scoped connection for tests that mutate connection state.

    Required for tests that call ``close()``, ``autocommit()``, ``commit()``,
    ``rollback()``, or similar methods that invalidate the session.
    """
    conn = create_connection_with_adapter(connector_adapter)
    wrapped = _wrap_for_async_backends(
        conn,
        connection_backend=connection_backend,
        cursor_backend=cursor_backend,
    )
    try:
        yield wrapped
    finally:
        if not wrapped.is_closed():
            wrapped.close()


@pytest.fixture(scope="module")
def connection_factory(connector_adapter, connection_backend, cursor_backend):
    """Factory function for creating connections with custom parameters."""

    def _create_connection(**override_params):
        """Create a connection with custom parameters.

        Args:
            **override_params: Parameters to override defaults

        Example:
            conn = connection_factory(account="test_account", user="test_user")
        """
        conn = create_connection_with_adapter(connector_adapter, **override_params)
        return _wrap_for_async_backends(
            conn,
            connection_backend=connection_backend,
            cursor_backend=cursor_backend,
        )

    return _create_connection


@pytest.fixture
def cursor(connection):
    """Create a test cursor from a connection."""
    with connection.cursor() as cursor:
        yield cursor


@pytest.fixture
def dict_cursor(connection):
    """Create a DictCursor from a connection."""
    with connection.cursor(cursor_class=DictCursor) as cursor:
        yield cursor


@pytest.fixture
def cursor_with_numpy(connector_adapter):
    """Create a cursor from a connection with numpy=True."""
    with create_connection_with_adapter(connector_adapter, numpy=True) as conn:
        with conn.cursor() as cur:
            yield cur


@pytest.fixture
def execute_query(cursor):
    """Helper replacing cursor if your only use case is to execute a query."""

    def _execute_query(*args: Any, single_row: bool = False, **kwargs: Any) -> Row | list[Row] | None:
        cursor.execute(*args, **kwargs)
        if single_row:
            return cursor.fetchone()
        return cursor.fetchall()

    return _execute_query


@pytest.fixture
def executemany_insert(cursor):
    """Fixture for bulk-inserting rows via executemany and reading them back.

    Returns a callable:
        executemany_insert(table_name, sql, rows) -> list[Row]

    It executes cursor.executemany(sql, rows), then SELECTs all rows
    from the table ordered by the first column.

    Useful for testing multirow binding.
    """

    def _executemany_insert(table_name: str, sql: str, rows: list[tuple[Any, ...]]) -> list[tuple[Any, ...]]:
        cursor.executemany(sql, rows)
        cursor.execute(f"SELECT * FROM {table_name} ORDER BY 1")
        return cursor.fetchall()

    return _executemany_insert


@pytest.fixture
def tmp_schema(cursor):
    """Create a temporary schema."""
    import uuid

    schema_name = f"test_schema_{uuid.uuid4().hex}"
    cursor.execute(f"CREATE SCHEMA {schema_name}")
    try:
        yield schema_name
    finally:
        cursor.execute(f"DROP SCHEMA {schema_name}")


@pytest.fixture
def int_test_connection_factory(connector_adapter):
    """Factory function for creating connections with integration test parameters."""

    def _create_connection(**override_params):
        """Create a connection with integration test parameters."""
        default_server_url = "http://localhost:8090"
        server_url = override_params.get("server_url", default_server_url)
        parsed_url = urlparse(server_url)

        # Default integration test parameters
        integration_params = {
            "account": "test_account",
            "user": "test_user",
            "database": "test_database",
            "schema": "test_schema",
            "warehouse": "test_warehouse",
            "role": "test_role",
            "server_url": server_url,
            "protocol": parsed_url.scheme,
            "host": parsed_url.hostname,
            "port": parsed_url.port,
            "authenticator": "SNOWFLAKE_JWT",
            "private_key_file": get_test_private_key_path(),
        }

        integration_params.update(override_params)

        return create_connection_with_adapter(connector_adapter, **integration_params)

    return _create_connection


@pytest.fixture(scope="session")
def _wiremock_session():
    """Start one Wiremock JVM per xdist worker and reuse it across tests."""
    client = WiremockClient().start()
    try:
        yield client
    finally:
        client.stop()


@pytest.fixture
def wiremock(_wiremock_session):
    """Per-test Wiremock handle backed by a session-scoped JVM.

    Mappings and captured requests are cleared before each test; the JVM itself
    stays up, saving ~1–3 s of startup per test.
    """
    _wiremock_session.reset()
    return _wiremock_session


def pytest_runtest_setup(item):
    """Skip tests based on connector type and markers."""
    callspec = getattr(item, "callspec", None)
    params = callspec.params if callspec is not None else {}

    skip_async_marker = item.get_closest_marker("skip_async")
    if skip_async_marker is not None and params.get("cursor_backend") == "async":
        reason = skip_async_marker.kwargs.get("reason", "Skipping test for async cursor backend")
        pytest.skip(reason)

    skip_async_connection_marker = item.get_closest_marker("skip_async_connection")
    if skip_async_connection_marker is not None and params.get("connection_backend") == "async":
        reason = skip_async_connection_marker.kwargs.get(
            "reason",
            "Skipping test for async connection backend",
        )
        pytest.skip(reason)

    if IS_UNIVERSAL_DRIVER and item.get_closest_marker("skip_universal"):
        marker = item.get_closest_marker("skip_universal")
        reason = marker.kwargs.get("reason", "Skipping test for universal driver")
        pytest.skip(reason)
    elif not IS_UNIVERSAL_DRIVER and item.get_closest_marker("skip_reference"):
        marker = item.get_closest_marker("skip_reference")
        reason = marker.kwargs.get("reason", "Skipping test for reference driver")
        pytest.skip(reason)
    marker = item.get_closest_marker("skip_unless_native_arrow")
    if marker is not None and not native_arrow_enabled():
        reason = marker.kwargs.get(
            "reason",
            "Requires the native-arrow row iterator (skipped on the Cython/reference converter)",
        )
        pytest.skip(reason)
    marker = item.get_closest_marker("skip_for_json_result_set")
    if marker is not None:
        result_format = os.getenv("QUERY_RESULT_FORMAT")
        if result_format and result_format.upper() == "JSON":
            reason = marker.kwargs.get("reason", "Test requires Arrow format precision")
            pytest.skip(f"Skipped for JSON result format: {reason}")

    if item.get_closest_marker("require_vpn") and os.environ.get("JENKINS_URL") is None:
        pytest.skip("Requires VPN (run on Jenkins)")


from tests.helpers.fixtures import core_proxy as core_proxy  # noqa: E402
from tests.helpers.fixtures import mock_async_db_api as mock_async_db_api  # noqa: E402
from tests.helpers.fixtures import mock_db_api as mock_db_api  # noqa: E402
