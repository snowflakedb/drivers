"""
Integration tests for PEP 249 Connection objects.
"""

from io import StringIO
from unittest.mock import Mock

import pytest

from snowflake.connector.cursor import DictCursor
from snowflake.connector.errors import NotSupportedError


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


class TestConnectionMethods:
    """Test Connection object methods."""

    def test_close_connection(self, connection):
        """Test closing a connection."""
        assert not connection.is_closed()
        connection.close()
        assert connection.is_closed()


class TestConnectionOptionalMethods:
    """Test optional Connection methods."""

    @pytest.mark.skip_reference(reason="Reference driver has no cancel method")
    def test_cancel_not_implemented(self, connection):
        """Test that cancel raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.cancel()
        assert "cancel is not implemented" in str(excinfo.value)

    @pytest.mark.skip_reference(reason="Reference driver has no ping method")
    def test_ping_not_implemented(self, connection):
        """Test that ping raises NotSupportedError."""
        with pytest.raises(NotSupportedError) as excinfo:
            connection.ping()
        assert "ping is not implemented" in str(excinfo.value)

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
