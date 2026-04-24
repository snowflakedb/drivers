import pytest

ITERATIONS = 10
WARMUP_ITERATIONS = 2

_JSON_FORMAT_SETUP = [
    "alter session set query_result_format = 'JSON'",
    "alter session set PYTHON_CONNECTOR_QUERY_RESULT_FORMAT = 'JSON'",
    "alter session set ODBC_QUERY_RESULT_FORMAT = 'JSON'",
]


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_number_1M_json(perf_test):
    perf_test(
        sql_command="SELECT L_LINENUMBER::INT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
        setup_queries=_JSON_FORMAT_SETUP,
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_15columns_1M_json(perf_test):
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
        setup_queries=_JSON_FORMAT_SETUP,
    )
