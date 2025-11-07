import pytest


def test_select_string_1000000_rows(perf_test):
    """Test with default iterations (2 iterations, 1 warmup)"""
    perf_test(
        sql_command="select L_COMMENT from SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM limit 1000000"
    )


@pytest.mark.iterations(3)
@pytest.mark.warmup_iterations(1)
def test_select_number_1000000_rows(perf_test):
    perf_test(
        sql_command="select L_LINENUMBER::int from SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM limit 1000000"
    )
