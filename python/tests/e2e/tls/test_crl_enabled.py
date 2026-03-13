import pytest

from tests.e2e.types.utils import assert_connection_is_open


@pytest.mark.skip_reference(reason="CRL e2e applies to universal driver")
def test_should_connect_and_select_with_crl_enabled(execute_query, connection_factory):
    # Given Snowflake client is logged in
    assert_connection_is_open(execute_query)
    with connection_factory(crl_check_mode="ENABLED") as conn:
        cur = conn.cursor()

        # When Query "SELECT 1" is executed
        cur.execute("SELECT 1")

        # Then the request attempt should be successful
        row = cur.fetchone()
        assert row is not None
        assert row[0] in (1, "1")
