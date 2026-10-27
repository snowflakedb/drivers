"""Python-specific E2E tests for the validate_default_parameters connection option."""

import uuid

import pytest

from snowflake.connector.errors import DatabaseError


def _nonexistent_name() -> str:
    return f"NEVEREXISTS_{uuid.uuid4().hex.upper()}"


class TestValidateDefaultParameters:
    @pytest.mark.parametrize("parameter", ["database", "schema", "warehouse"])
    def test_should_fail_login_for_nonexistent_object_when_validation_enabled(self, connection_factory, parameter):
        # Given Snowflake client is configured with a nonexistent <parameter> and validate_default_parameters=True
        params = {parameter: _nonexistent_name(), "validate_default_parameters": True}

        # When Trying to Connect
        with pytest.raises(DatabaseError):
            # Then Login fails
            with connection_factory(**params):
                pass

    @pytest.mark.parametrize("validate_default_parameters", [False, None], ids=["false", "unset"])
    def test_should_connect_with_nonexistent_database_when_validation_disabled(
        self, connection_factory, validate_default_parameters
    ):
        # Given Snowflake client is configured with a nonexistent database
        # And validate_default_parameters is false or unset
        params = {"database": _nonexistent_name()}
        if validate_default_parameters is not None:
            params["validate_default_parameters"] = validate_default_parameters

        # When Trying to Connect
        with connection_factory(**params) as conn, conn.cursor() as cursor:
            cursor.execute("SELECT CURRENT_DATABASE()")

            # Then Login succeeds and no database is active
            assert cursor.fetchone() == (None,)
