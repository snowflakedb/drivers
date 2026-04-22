"""Performance test for 1M rows with WireMock - for stability testing"""
import pytest
from runner.test_types import PerfTestType

ITERATIONS = 10
WARMUP_ITERATIONS = 2


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_string_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT L_COMMENT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_number_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT L_LINENUMBER::INT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_date_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT L_SHIPDATE FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_float_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT L_EXTENDEDPRICE FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_double_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT L_EXTENDEDPRICE::DOUBLE FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_boolean_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT (L_TAX > 0.04)::BOOLEAN FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_timestamp_ntz_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT L_SHIPDATE::TIMESTAMP_NTZ FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_timestamp_tz_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT L_SHIPDATE::TIMESTAMP_TZ FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_time_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT TIME_FROM_PARTS(MOD(L_ORDERKEY, 24), MOD(L_PARTKEY, 60), MOD(L_SUPPKEY, 60)) FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_binary_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT TO_BINARY(L_COMMENT, 'UTF-8') FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_15columns_1M_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
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
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_string_1M_ordered_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT L_COMMENT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM ORDER BY L_ORDERKEY LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_number_1M_ordered_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command="SELECT L_LINENUMBER::INT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM ORDER BY L_ORDERKEY LIMIT 1000000",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
def test_select_15columns_1M_ordered_arrow_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
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
            ORDER BY L_ORDERKEY
            LIMIT 1000000
        """,
    )
