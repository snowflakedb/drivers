"""
Integration tests for PEP 249 Connection objects.
"""

import uuid

from io import StringIO
from unittest.mock import Mock

import pytest

from snowflake.connector.constants import QueryStatus
from snowflake.connector.cursor import DictCursor
from snowflake.connector.errors import DatabaseError, InterfaceError, ProgrammingError


class TestConnectionInfo:
    """Integration tests for Connection._get_connection_info."""

    @pytest.mark.skip_reference(reason="Reference driver has no _get_connection_info method")
    def test_get_connection_info_returns_info_after_connect(self, connection):
        """Test that _get_connection_info returns info after connection is established."""
        # Given an established connection
        # When calling _get_connection_info
        info = connection._get_connection_info()

        # Then it should not be None
        assert info is not None


class TestConnectionParameters:
    """Reference tests: minimum parameters required to establish a connection.

    The old snowflake-connector-python driver accepts just ``account`` (plus
    auth credentials) and derives the host as ``{account}.snowflakecomputing.com``.
    These tests exercise that behavior against both drivers.
    """

    def test_connect_with_account_only_no_host_or_server_url(self, connection_factory):
        """Connection succeeds when only ``account`` is given — host/server_url are derived.

        Skipped when the test environment targets a non-production host (preprod,
        localhost, or a dev deployment) — derivation only yields a valid URL for
        accounts whose canonical host is ``{account}.snowflakecomputing.com``.

        Passing ``None`` for host/server_url/port/protocol causes the connection
        factory to omit those parameters entirely, forcing the driver to derive
        them from ``account``.
        """
        from tests.config import get_test_parameters

        test_params = get_test_parameters()
        account = test_params.get("SNOWFLAKE_TEST_ACCOUNT")
        if not account:
            pytest.skip("SNOWFLAKE_TEST_ACCOUNT not configured")
        custom_host = test_params.get("SNOWFLAKE_TEST_HOST") or ""
        custom_server_url = test_params.get("SNOWFLAKE_TEST_SERVER_URL") or ""
        expected_host = f"{account}.snowflakecomputing.com"
        if (custom_host and custom_host != expected_host) or (
            custom_server_url and expected_host not in custom_server_url
        ):
            pytest.skip(
                "Test environment overrides host/server_url; account-name derivation "
                f"yields '{expected_host}', not the configured target."
            )

        with connection_factory(host=None, server_url=None, port=None, protocol=None) as conn:
            assert not conn.is_closed()
            cur = conn.cursor()
            try:
                cur.execute("SELECT 1")
                row = cur.fetchone()
                assert row[0] == 1
            finally:
                cur.close()


class TestConnectionInfoProperties:
    """Integration tests for Connection properties backed by _get_connection_info."""

    @pytest.mark.parametrize("prop", ["account", "user", "host", "role", "database", "schema", "warehouse"])
    def test_string_property_is_set(self, connection, prop):
        """After connecting, string properties should return a non-empty string."""
        value = getattr(connection, prop)
        assert value is not None, f"connection.{prop} should not be None"
        assert isinstance(value, str), f"connection.{prop} should be a str"
        assert len(value) > 0, f"connection.{prop} should not be empty"

    def test_session_id_is_set(self, connection):
        """After connecting, session_id should return a positive integer."""
        sid = connection.session_id
        assert isinstance(sid, int)
        assert sid > 0

    def test_port_is_set(self, connection):
        """After connecting, port should return a valid port number or None."""
        port = connection.port
        if port is not None:
            assert isinstance(port, int)
            assert port > 0


class TestConnectionInfoReflectsSessionChanges:
    """Integration tests verifying that properties reflect server-side session changes."""

    def test_database_reflects_use_database(self, connection):
        """After USE DATABASE, the database property should reflect the new database."""
        original_db = connection.database
        assert original_db is not None

        tmp_db = f"TEST_DB_{uuid.uuid4().hex}".upper()
        cur = connection.cursor()
        try:
            cur.execute(f"CREATE DATABASE {tmp_db}")
            cur.execute(f"USE DATABASE {tmp_db}")
            assert connection.database.upper() == tmp_db
        finally:
            cur.execute(f"USE DATABASE {original_db}")
            cur.execute(f"DROP DATABASE IF EXISTS {tmp_db}")
            cur.close()

    def test_schema_reflects_use_schema(self, connection):
        """After USE SCHEMA, the schema property should reflect the new schema."""
        cur = connection.cursor()
        try:
            cur.execute("USE SCHEMA INFORMATION_SCHEMA")
        finally:
            cur.close()

        assert connection.schema.upper() == "INFORMATION_SCHEMA"


class TestClosedConnection:
    """Test that operations on a closed connection behaves correctly."""

    def test_commit_on_closed_connection(self, connection_factory):
        conn = connection_factory()
        conn.close()
        with pytest.raises(DatabaseError) as excinfo:
            conn.commit()
        error = excinfo.value
        assert "connection is closed" in error.msg.lower()
        assert error.errno == 250002

    def test_rollback_on_closed_connection(self, connection_factory):
        conn = connection_factory()
        conn.close()
        with pytest.raises(DatabaseError) as excinfo:
            conn.rollback()
        error = excinfo.value
        assert "connection is closed" in error.msg.lower()
        assert error.errno == 250002

    def test_cursor_on_closed_connection(self, connection_factory):
        conn = connection_factory()
        conn.close()
        with pytest.raises(DatabaseError) as excinfo:
            conn.cursor()
        error = excinfo.value
        assert "connection is closed" in error.msg.lower()
        assert error.errno == 250002

    def test_execute_on_closed_connection(self, connection_factory):
        conn = connection_factory()
        cur = conn.cursor()
        conn.close()
        with pytest.raises(InterfaceError) as excinfo:
            cur.execute("SELECT 1")
        error = excinfo.value
        assert "cursor is closed" in error.msg.lower()
        assert error.errno == 252006

    def test_double_close(self, connection_factory):
        conn = connection_factory()
        conn.close()
        conn.close()

    def test_autocommit_on_closed_connection(self, connection_factory):
        conn = connection_factory()
        conn.close()
        with pytest.raises(DatabaseError) as excinfo:
            conn.autocommit(True)
        error = excinfo.value
        assert "connection is closed" in error.msg.lower()
        assert error.errno == 250002

    def test_get_query_status_on_closed_connection(self, connection_factory):
        conn = connection_factory()
        cur = conn.cursor()
        cur.execute("SELECT 1")
        sfqid = cur.sfqid
        cur.close()
        conn.close()
        status = conn.get_query_status(sfqid)
        assert status == QueryStatus.DISCONNECTED

    def test_get_query_status_throw_if_error_on_closed_connection(self, connection_factory):
        conn = connection_factory()
        cur = conn.cursor()
        cur.execute("SELECT 1")
        sfqid = cur.sfqid
        cur.close()
        conn.close()
        with pytest.raises(ProgrammingError) as excinfo:
            conn.get_query_status_throw_if_error(sfqid)
        error = excinfo.value
        assert error.sfqid == sfqid
        assert error.errno == -1

    def test_connection_info_properties_on_closed_connection(self, connection_factory):
        conn = connection_factory()
        role = conn.role
        database = conn.database
        schema = conn.schema
        warehouse = conn.warehouse
        user = conn.user
        account = conn.account
        host = conn.host
        port = conn.port
        session_id = conn.session_id

        conn.close()

        assert conn.role == role
        assert conn.database == database
        assert conn.schema == schema
        assert conn.warehouse == warehouse
        assert conn.user == user
        assert conn.account == account
        assert conn.host == host
        assert conn.port == port
        assert conn.session_id == session_id

    def test_session_parameters_on_closed_connection(self, connection_factory):
        conn = connection_factory(session_parameters={"TIMEZONE": "America/Los_Angeles"})
        conn.close()
        assert conn._session_parameters["TIMEZONE"] == "America/Los_Angeles"


class TestConnectionMethods:
    """Test Connection object methods."""

    def test_close_connection(self, connection):
        """Test closing a connection."""
        assert not connection.is_closed()
        connection.close()
        assert connection.is_closed()


class TestConnectionOptionalMethods:
    """Test optional Connection methods."""

    @pytest.mark.skip_reference(reason="Reference driver has no set_autocommit method")
    def test_set_autocommit(self, connection):
        """Test that set_autocommit changes the autocommit flag."""
        connection.set_autocommit(False)
        assert connection._autocommit is False
        connection.set_autocommit(True)
        assert connection._autocommit is True

    @pytest.mark.skip_reference(reason="Reference driver has no set_autocommit/get_autocommit methods")
    def test_get_autocommit(self, connection):
        """Test that get_autocommit returns the current setting."""
        connection.set_autocommit(False)
        assert connection.get_autocommit() is False
        connection.set_autocommit(True)
        assert connection.get_autocommit() is True


class TestConnectionAutocommitMethod:
    """Test Connection autocommit method."""

    @pytest.mark.skip_reference(reason="Reference driver has no set_autocommit method")
    def test_autocommit_sets_flag_and_calls_set_autocommit(self, connection, monkeypatch):
        """Test that autocommit() delegates to set_autocommit."""
        mock_set_autocommit = Mock()
        monkeypatch.setattr(connection, "set_autocommit", mock_set_autocommit)

        connection.autocommit(True)

        mock_set_autocommit.assert_called_once_with(True)

    @pytest.mark.skip_reference(reason="Reference driver _autocommit defaults to None, not True")
    def test_autocommit_default_is_server_default(self, connection):
        """Test that autocommit defaults to the server default (true) when not explicitly set."""
        assert connection._autocommit is True

    @pytest.mark.skip_reference(reason="Reference driver has no get_autocommit method")
    def test_autocommit_roundtrip(self, connection):
        """Test setting autocommit via autocommit() and reading via get_autocommit()."""
        connection.autocommit(True)
        assert connection.get_autocommit() is True

        connection.autocommit(False)
        assert connection.get_autocommit() is False


class TestExecuteString:
    """Integration tests for Connection.execute_string()."""

    def test_execute_string_single_statement(self, connection):
        """Test execute_string with a single statement."""
        # When executing a single statement
        cursors = connection.execute_string("SELECT 1 AS val")

        # Then it should return a list with one cursor
        cursors = list(cursors)
        assert len(cursors) == 1
        result = cursors[0].fetchone()
        assert result == (1,)

    def test_execute_string_multiple_statements(self, connection):
        """Test execute_string with multiple semicolon-separated statements."""
        # When executing multiple statements
        cursors = connection.execute_string("SELECT 1; SELECT 2; SELECT 3")

        # Then it should return a cursor per statement
        cursors = list(cursors)
        assert len(cursors) == 3
        assert cursors[0].fetchone() == (1,)
        assert cursors[1].fetchone() == (2,)
        assert cursors[2].fetchone() == (3,)

    def test_execute_string_return_cursors_false(self, connection):
        """Test execute_string with return_cursors=False still executes all statements."""
        # Given a table to verify execution
        connection.execute_string("CREATE TEMPORARY TABLE _exec_str_test (id INTEGER)")

        # When executing with return_cursors=False
        result = connection.execute_string(
            "INSERT INTO _exec_str_test VALUES (1); INSERT INTO _exec_str_test VALUES (2)",
            return_cursors=False,
        )

        # Then the result should be empty but statements were executed
        assert list(result) == []
        cursors = connection.execute_string("SELECT COUNT(*) FROM _exec_str_test")
        count = list(cursors)[0].fetchone()[0]
        assert count == 2

    def test_execute_string_with_comments(self, connection):
        """Test execute_string handles SQL comments correctly."""
        sql = """
        -- This is a comment
        SELECT 1;
        /* Block comment */
        SELECT 2
        """
        # When executing SQL with comments
        cursors = connection.execute_string(sql)

        # Then comments should not interfere with statement splitting
        cursors = list(cursors)
        assert len(cursors) == 2
        assert cursors[0].fetchone() == (1,)
        assert cursors[1].fetchone() == (2,)

    def test_execute_string_remove_comments(self, connection):
        """Test execute_string with remove_comments=True."""
        sql = "-- leading comment\nSELECT 1; /* inline */ SELECT 2"
        # When executing with remove_comments
        cursors = connection.execute_string(sql, remove_comments=True)

        # Then statements should still execute correctly
        cursors = list(cursors)
        assert len(cursors) == 2
        assert cursors[0].query == "SELECT 1;"
        assert cursors[1].query == "SELECT 2"

    def test_execute_string_with_quoted_semicolons(self, connection):
        """Test execute_string doesn't split on semicolons inside quotes."""
        sql = "SELECT 'hello;world' AS val"
        # When executing SQL with a semicolon inside a string literal
        cursors = connection.execute_string(sql)

        # Then it should be treated as a single statement
        cursors = list(cursors)
        assert len(cursors) == 1
        assert cursors[0].fetchone() == ("hello;world",)

    def test_execute_string_with_cursor_class(self, connection):
        """Test execute_string with a custom cursor class."""
        cursors = connection.execute_string("SELECT 1 AS id", cursor_class=DictCursor)

        cursors = list(cursors)
        assert len(cursors) == 1
        assert isinstance(cursors[0], DictCursor)
        assert cursors[0].fetchone() == {"ID": 1}


class TestExecuteStream:
    """Integration tests for Connection.execute_stream()."""

    def test_execute_stream_single_statement(self, connection):
        """Test execute_stream with a single statement."""
        stream = StringIO("SELECT 42 AS answer")
        # When executing a stream with a single statement
        cursors = list(connection.execute_stream(stream))

        # Then it should yield one cursor
        assert len(cursors) == 1
        assert cursors[0].fetchone() == (42,)

    def test_execute_stream_multiple_statements(self, connection):
        """Test execute_stream with multiple statements."""
        stream = StringIO("SELECT 1; SELECT 2; SELECT 3")
        # When executing a stream with multiple statements
        cursors = list(connection.execute_stream(stream))

        # Then it should yield one cursor per statement
        assert len(cursors) == 3
        assert cursors[0].fetchone() == (1,)
        assert cursors[1].fetchone() == (2,)
        assert cursors[2].fetchone() == (3,)

    def test_execute_stream_is_lazy_generator(self, connection):
        """Test that execute_stream returns a generator, not a list."""
        stream = StringIO("SELECT 1; SELECT 2")
        result = connection.execute_stream(stream)

        # The result should be a generator
        from collections.abc import Generator

        assert isinstance(result, Generator)

    def test_execute_stream_with_comments_and_mixed_statements(self, connection):
        """Test execute_stream with comments interleaved among statements."""
        sql = """
        -- Setup comment
        SELECT 'first' AS label;
        /* Multi-line
           comment */
        SELECT 'second' AS label
        """
        stream = StringIO(sql)
        cursors = list(connection.execute_stream(stream))

        assert len(cursors) == 2
        assert cursors[0].fetchone() == ("first",)
        assert cursors[1].fetchone() == ("second",)

    def test_execute_stream_with_cursor_class(self, connection):
        """Test execute_stream with a custom cursor class."""
        stream = StringIO("SELECT 1 AS id")
        cursors = list(connection.execute_stream(stream, cursor_class=DictCursor))

        assert len(cursors) == 1
        assert isinstance(cursors[0], DictCursor)
        assert cursors[0].fetchone() == {"ID": 1}


class TestCommitRollback:
    """Integration tests for commit and rollback."""

    def test_commit_persists_inserted_rows(self, connection, connection_factory, tmp_schema):
        """Test that commit() persists data inserted in a transaction."""
        table = f"{tmp_schema}.test_commit"
        connection.autocommit(False)
        cur = connection.cursor()
        cur.execute(f"CREATE TABLE {table} (id INTEGER, name VARCHAR)")
        connection.commit()

        cur.execute(f"INSERT INTO {table} VALUES (1, 'alice')")

        # Before commit, the row should not be visible from another session
        with connection_factory() as other_conn:
            other_cur = other_conn.cursor()
            other_cur.execute(f"SELECT COUNT(*) FROM {table}")
            assert other_cur.fetchone() == (0,)

        connection.commit()

        cur.execute(f"SELECT id, name FROM {table} WHERE id = 1")
        assert cur.fetchone() == (1, "alice")

    def test_rollback_discards_inserted_rows(self, connection, tmp_schema):
        """Test that rollback() discards uncommitted inserts."""
        table = f"{tmp_schema}.test_rollback"
        connection.autocommit(False)
        cur = connection.cursor()
        cur.execute(f"CREATE TABLE {table} (id INTEGER)")
        cur.execute(f"INSERT INTO {table} VALUES (1)")
        connection.commit()

        cur.execute(f"INSERT INTO {table} VALUES (2)")
        connection.rollback()

        cur.execute(f"SELECT COUNT(*) FROM {table}")
        assert cur.fetchone() == (1,)

    def test_rollback_discards_update(self, connection, tmp_schema):
        """Test that rollback() reverts an UPDATE to previously committed data."""
        table = f"{tmp_schema}.test_rb_upd"
        connection.autocommit(False)
        cur = connection.cursor()
        cur.execute(f"CREATE TABLE {table} (id INTEGER, val VARCHAR)")
        cur.execute(f"INSERT INTO {table} VALUES (1, 'original')")
        connection.commit()

        cur.execute(f"UPDATE {table} SET val = 'modified' WHERE id = 1")
        connection.rollback()

        cur.execute(f"SELECT val FROM {table} WHERE id = 1")
        assert cur.fetchone() == ("original",)


class TestAutocommitAlterSession:
    """Integration tests for set_autocommit ALTER SESSION."""

    @pytest.mark.skip_reference(reason="Reference driver has no set_autocommit/_get_session_parameter methods")
    def test_set_autocommit_true_updates_session_parameter(self, connection):
        """Test that set_autocommit(True) sets the AUTOCOMMIT session parameter."""
        connection.set_autocommit(True)
        assert connection._get_session_parameter("AUTOCOMMIT") == "true"

    @pytest.mark.skip_reference(reason="Reference driver has no set_autocommit/_get_session_parameter methods")
    def test_set_autocommit_false_updates_session_parameter(self, connection):
        """Test that set_autocommit(False) sets the AUTOCOMMIT session parameter."""
        connection.set_autocommit(False)
        assert connection._get_session_parameter("AUTOCOMMIT") == "false"

    def test_autocommit_on_persists_without_explicit_commit(self, connection, tmp_schema):
        """Test that with autocommit ON, each statement is committed automatically."""
        table = f"{tmp_schema}.test_ac_on"
        connection.autocommit(True)
        cur = connection.cursor()
        cur.execute(f"CREATE TABLE {table} (id INTEGER)")
        cur.execute(f"INSERT INTO {table} VALUES (1)")
        # No explicit commit — autocommit should handle it

        cur.execute(f"SELECT COUNT(*) FROM {table}")
        assert cur.fetchone() == (1,)


class TestContextManagerAutocommit:
    """Integration tests for context manager with autocommit."""

    def test_context_manager_commits_inserts_on_clean_exit(self, connection_factory, tmp_schema):
        """Test that the context manager commits DML on clean exit when autocommit is off."""
        table = f"{tmp_schema}.test_cm_commit"
        with connection_factory() as conn:
            conn.autocommit(False)
            cur = conn.cursor()
            cur.execute(f"CREATE TABLE {table} (id INTEGER)")
            conn.commit()
            cur.execute(f"INSERT INTO {table} VALUES (1)")
            cur.execute(f"INSERT INTO {table} VALUES (2)")
            # clean exit should trigger commit

        with connection_factory() as conn:
            cur = conn.cursor()
            cur.execute(f"SELECT COUNT(*) FROM {table}")
            assert cur.fetchone() == (2,)

    def test_context_manager_rolls_back_on_exception(self, connection_factory, tmp_schema):
        """Test that the context manager rolls back on exception when autocommit is off."""
        table = f"{tmp_schema}.test_cm_rb"
        with connection_factory() as setup_conn:
            setup_conn.autocommit(False)
            cur = setup_conn.cursor()
            cur.execute(f"CREATE TABLE {table} (id INTEGER)")
            cur.execute(f"INSERT INTO {table} VALUES (1)")
            setup_conn.commit()

        try:
            with connection_factory() as conn:
                conn.autocommit(False)
                cur = conn.cursor()
                cur.execute(f"INSERT INTO {table} VALUES (99)")
                raise ValueError("simulated error")
        except ValueError:
            pass

        with connection_factory() as conn:
            cur = conn.cursor()
            cur.execute(f"SELECT COUNT(*) FROM {table}")
            assert cur.fetchone() == (1,)

    def test_context_manager_with_autocommit_on_does_not_commit_or_rollback(self, connection_factory, tmp_schema):
        """Test that with autocommit ON, __exit__ skips explicit commit/rollback."""
        table = f"{tmp_schema}.test_cm_ac"
        with connection_factory() as conn:
            conn.autocommit(True)
            cur = conn.cursor()
            cur.execute(f"CREATE TABLE {table} (id INTEGER)")
            cur.execute(f"INSERT INTO {table} VALUES (1)")
            cur.execute(f"SELECT COUNT(*) FROM {table}")
            assert cur.fetchone() == (1,)


class TestIsValid:
    """Integration tests for Connection.is_valid()."""

    def test_is_valid_returns_true_on_open_connection(self, connection):
        """is_valid() should return True on a live connection."""
        assert connection.is_valid() is True

    def test_is_valid_returns_false_after_close(self, connection_factory):
        """is_valid() should return False after the connection is closed."""
        conn = connection_factory()
        conn.close()
        assert conn.is_valid() is False
