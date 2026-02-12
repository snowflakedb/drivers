"""
Integration tests for session parameters API.
"""


class TestSessionParametersGet:
    """Test getting session parameter values."""

    def test_get_parameter_after_alter_session(self, connection):
        """Test retrieving a session parameter after setting it via ALTER SESSION."""
        cursor = connection.cursor()
        cursor.execute("ALTER SESSION SET QUERY_TAG = 'test_tag_123'")
        value = connection._get_session_parameter("QUERY_TAG")
        assert value == "test_tag_123"

    def test_get_parameter_case_insensitive(self, connection):
        """Test that parameter names are case-insensitive."""
        cursor = connection.cursor()
        cursor.execute("ALTER SESSION SET QUERY_TAG = 'test_tag_456'")

        # All variations should return the same value
        assert connection._get_session_parameter("QUERY_TAG") == "test_tag_456"
        assert connection._get_session_parameter("query_tag") == "test_tag_456"
        assert connection._get_session_parameter("Query_Tag") == "test_tag_456"

    def test_get_nonexistent_parameter(self, connection):
        """Test getting a parameter that doesn't exist returns None."""
        value = connection._get_session_parameter("NONEXISTENT_PARAM_XYZ")
        assert value is None


class TestSessionParametersInit:
    """Test setting session parameters at connection initialization."""

    def test_session_parameters_at_connect(self, connection_factory):
        """Test setting session parameters during connection initialization."""
        with connection_factory(
            session_parameters={"QUERY_TAG": "init_test", "TIMEZONE": "America/Los_Angeles"}
        ) as conn:
            assert conn._get_session_parameter("QUERY_TAG") == "init_test"
            assert conn._get_session_parameter("TIMEZONE") == "America/Los_Angeles"

    def test_session_parameters_single_param(self, connection_factory):
        """Test setting a single session parameter at connection time."""
        with connection_factory(session_parameters={"QUERY_TAG": "single_param_test"}) as conn:
            assert conn._get_session_parameter("QUERY_TAG") == "single_param_test"

    def test_session_parameters_empty_dict(self, connection_factory):
        """Test that empty session_parameters dict works correctly."""
        with connection_factory(session_parameters={}) as conn:
            # Should not cause any errors
            # Query a known parameter to verify connection works
            value = conn._get_session_parameter("QUERY_TAG")
            # Value could be None or empty depending on server defaults
            assert value is None or isinstance(value, str)

    def test_no_session_parameters(self, connection_factory):
        """Test connection without session_parameters still works."""
        with connection_factory() as conn:
            # Should not cause any errors
            value = conn._get_session_parameter("QUERY_TAG")
            assert value is None or isinstance(value, str)


class TestSessionParametersRoundtrip:
    """Test session parameters roundtrip (set via ALTER then get)."""

    def test_multiple_parameters_roundtrip(self, connection):
        """Test setting and getting multiple session parameters."""
        cursor = connection.cursor()

        # Set multiple parameters
        cursor.execute("ALTER SESSION SET QUERY_TAG = 'multi_test'")
        cursor.execute("ALTER SESSION SET TIMEZONE = 'UTC'")
        cursor.execute("ALTER SESSION SET TIMESTAMP_OUTPUT_FORMAT = 'YYYY-MM-DD HH24:MI:SS'")

        # Verify all can be retrieved
        assert connection._get_session_parameter("QUERY_TAG") == "multi_test"
        assert connection._get_session_parameter("TIMEZONE") == "UTC"
        assert connection._get_session_parameter("TIMESTAMP_OUTPUT_FORMAT") == "YYYY-MM-DD HH24:MI:SS"

    def test_parameter_update(self, connection):
        """Test updating a session parameter value."""
        cursor = connection.cursor()

        # Set initial value
        cursor.execute("ALTER SESSION SET QUERY_TAG = 'initial_value'")
        assert connection._get_session_parameter("QUERY_TAG") == "initial_value"

        # Update value
        cursor.execute("ALTER SESSION SET QUERY_TAG = 'updated_value'")
        assert connection._get_session_parameter("QUERY_TAG") == "updated_value"

    def test_parameter_with_special_characters(self, connection):
        """Test session parameter values with special characters."""
        cursor = connection.cursor()

        # Test with quotes and spaces
        cursor.execute("ALTER SESSION SET QUERY_TAG = 'test with spaces and \"quotes\"'")
        value = connection._get_session_parameter("QUERY_TAG")
        assert "test with spaces" in value


class TestSessionParametersIntegration:
    """Integration tests combining init-time and runtime parameter operations."""

    def test_init_params_persist_across_queries(self, connection_factory):
        """Test that init-time parameters persist across multiple queries."""
        with connection_factory(session_parameters={"QUERY_TAG": "persistent_test"}) as conn:
            cursor = conn.cursor()

            # Execute multiple queries
            cursor.execute("SELECT 1")
            cursor.execute("SELECT 2")
            cursor.execute("SELECT 3")

            # Parameter should still be set
            assert conn._get_session_parameter("QUERY_TAG") == "persistent_test"

    def test_init_params_can_be_overridden(self, connection_factory):
        """Test that init-time parameters can be overridden at runtime."""
        with connection_factory(session_parameters={"QUERY_TAG": "initial"}) as conn:
            # Verify initial value
            assert conn._get_session_parameter("QUERY_TAG") == "initial"

            # Override at runtime
            cursor = conn.cursor()
            cursor.execute("ALTER SESSION SET QUERY_TAG = 'overridden'")

            # Verify new value
            assert conn._get_session_parameter("QUERY_TAG") == "overridden"
