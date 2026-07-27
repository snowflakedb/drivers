# Connection-level QUERY_TAG: set as a session parameter at connect, tagging
# every query in the session. Supported by both the universal and reference
# drivers via `session_parameters`.
class TestQueryTagViaConnectionOption:
    def test_should_tag_queries_when_query_tag_is_set_at_connection_level(self, connection_factory):
        # Given Snowflake client is logged in with connection option QUERY_TAG set to "conn_tag_e2e"
        with (
            connection_factory(session_parameters={"QUERY_TAG": "conn_tag_e2e"}) as conn,
            conn.cursor() as cursor,
        ):
            # When Query "SELECT CURRENT_QUERY_TAG()" is executed
            cursor.execute("SELECT CURRENT_QUERY_TAG()")
            row = cursor.fetchone()
            # Then the result should contain value "conn_tag_e2e"
            assert row[0] == "conn_tag_e2e"


# Statement-level QUERY_TAG via the `_statement_params` execute() kwarg — the
# same API the reference driver exposes, so this runs against both drivers.
class TestQueryTagViaStatementParameters:
    def test_should_tag_a_single_query_via_statement_level_query_tag(self, connection_factory):
        # Given Snowflake client is logged in
        with connection_factory() as conn, conn.cursor() as cursor:
            # When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt_tag_e2e"
            cursor.execute(
                "SELECT CURRENT_QUERY_TAG()",
                _statement_params={"QUERY_TAG": "stmt_tag_e2e"},
            )
            row = cursor.fetchone()
            # Then the result should contain value "stmt_tag_e2e"
            assert row[0] == "stmt_tag_e2e"

    def test_should_not_leak_statement_level_query_tag_into_session_state(self, connection_factory):
        # Given Snowflake client is logged in
        with connection_factory() as conn, conn.cursor() as cursor:
            # When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt_tag_e2e"
            cursor.execute(
                "SELECT CURRENT_QUERY_TAG()",
                _statement_params={"QUERY_TAG": "stmt_tag_e2e"},
            )
            # And Query "SELECT CURRENT_QUERY_TAG()" is executed without a statement-level tag
            cursor.execute("SELECT CURRENT_QUERY_TAG()")
            row = cursor.fetchone()
            # Then the last result should contain empty value
            assert row[0] == ""
