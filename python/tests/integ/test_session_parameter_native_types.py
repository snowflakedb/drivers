"""Native session-parameter types compared against the reference connector.

This file is intentionally not marked ``skip_reference``: both UD and legacy
``snowflake-connector-python`` expose ``_session_parameters["AUTOCOMMIT"]`` as
a native bool after ``ALTER SESSION SET AUTOCOMMIT = true``.
"""


def test_session_parameter_values_match_legacy_types(function_connection):
    cursor = function_connection.cursor()
    cursor.execute("ALTER SESSION SET AUTOCOMMIT = true")

    autocommit = function_connection._session_parameters["AUTOCOMMIT"]

    assert autocommit is True
    assert isinstance(autocommit, bool)
