import pytest


@pytest.mark.skip_reference(reason="CRL e2e applies to universal driver")
def test_connect_and_select_with_crl_enabled(connection_factory):
    # Given Snowflake client is logged in
    # When Query "SELECT 1" is executed
    # Then the request attempt should complete
    with connection_factory(crl_check_mode="ENABLED") as conn:
        cur = conn.cursor()
        cur.execute("SELECT 1")
        row = cur.fetchone()
        assert row is not None
        assert row[0] in (1, "1")

