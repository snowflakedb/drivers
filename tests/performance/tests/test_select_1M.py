import pytest

ITERATIONS = 10
WARMUP_ITERATIONS = 2

_ARROW_FORMAT_SETUP = [
    "alter session set query_result_format = 'ARROW'",
    "alter session set PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = 'ARROW'",
    "alter session set ODBC_QUERY_RESULT_FORMAT = 'ARROW'",
]


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_string_1M_arrow(perf_test):
    perf_test(
        sql_command="SELECT L_COMMENT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
        setup_queries=_ARROW_FORMAT_SETUP,
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_number_1M_arrow(perf_test):
    perf_test(
        sql_command="SELECT L_LINENUMBER::INT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
        setup_queries=_ARROW_FORMAT_SETUP,
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_date_1M_arrow(perf_test):
    perf_test(
        sql_command="SELECT L_SHIPDATE FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
        setup_queries=_ARROW_FORMAT_SETUP,
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_timestamp_ntz_1M_arrow(perf_test):
    perf_test(
        sql_command="SELECT L_SHIPDATE::TIMESTAMP_NTZ FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
        setup_queries=_ARROW_FORMAT_SETUP,
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_15columns_1M_arrow(perf_test):
    perf_test(
        sql_command="""
            SELECT
                L_ORDERKEY,
                L_PARTKEY,
                L_SUPPKEY,
                L_LINENUMBER,
                L_QUANTITY,
                L_EXTENDEDPRICE,
                L_DISCOUNT,
                L_TAX,
                L_RETURNFLAG,
                L_LINESTATUS,
                L_SHIPDATE,
                L_COMMITDATE,
                L_RECEIPTDATE,
                L_SHIPINSTRUCT,
                L_COMMENT
            FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM
            LIMIT 1000000
        """,
        setup_queries=_ARROW_FORMAT_SETUP,
    )
