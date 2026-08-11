"""1M-row pandas fetch performance (Python driver).

Datatypes match test_select_1M_recorded_http.py. Each query runs e2e and
recorded_http with fetch_mode=pandas (cursor.fetch_pandas_all()).
"""
import pytest
from runner.test_types import PerfTestType

# FETCH_MODE is read only by the Python driver app; other drivers ignore it and
# would report identical timings under misleading *_pandas names.
pytestmark = pytest.mark.supported_drivers("python")

ITERATIONS = 10
WARMUP_ITERATIONS = 2

_TABLE = "SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM"

_15_COLUMNS = """
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
            FROM {table}
            {order_by}
            LIMIT 1000000
        """

# (name_stem, sql) — name_stem becomes select_{stem}_pandas[_recorded_http]
# Same queries as test_select_1M_recorded_http.py
QUERIES = [
    (
        "string_1M_arrow",
        f"SELECT L_COMMENT FROM {_TABLE} LIMIT 1000000",
    ),
    (
        "number_1M_arrow",
        f"SELECT L_LINENUMBER::INT FROM {_TABLE} LIMIT 1000000",
    ),
    (
        "date_1M_arrow",
        f"SELECT L_SHIPDATE FROM {_TABLE} LIMIT 1000000",
    ),
    (
        "float_1M_arrow",
        f"SELECT L_EXTENDEDPRICE FROM {_TABLE} LIMIT 1000000",
    ),
    (
        "double_1M_arrow",
        f"SELECT L_EXTENDEDPRICE::DOUBLE FROM {_TABLE} LIMIT 1000000",
    ),
    (
        "boolean_1M_arrow",
        f"SELECT (L_TAX > 0.04)::BOOLEAN FROM {_TABLE} LIMIT 1000000",
    ),
    (
        "timestamp_ntz_1M_arrow",
        f"SELECT L_SHIPDATE::TIMESTAMP_NTZ FROM {_TABLE} LIMIT 1000000",
    ),
    (
        "timestamp_tz_1M_arrow",
        f"SELECT L_SHIPDATE::TIMESTAMP_TZ FROM {_TABLE} LIMIT 1000000",
    ),
    (
        "time_1M_arrow",
        f"SELECT TIME_FROM_PARTS(MOD(L_ORDERKEY, 24), MOD(L_PARTKEY, 60), MOD(L_SUPPKEY, 60)) FROM {_TABLE} LIMIT 1000000",
    ),
    (
        "binary_1M_arrow",
        f"SELECT TO_BINARY(L_COMMENT, 'UTF-8') FROM {_TABLE} LIMIT 1000000",
    ),
    (
        "15columns_1M_arrow",
        _15_COLUMNS.format(table=_TABLE, order_by=""),
    ),
    (
        "string_1M_ordered_arrow",
        f"SELECT L_COMMENT FROM {_TABLE} ORDER BY L_ORDERKEY LIMIT 1000000",
    ),
    (
        "number_1M_ordered_arrow",
        f"SELECT L_LINENUMBER::INT FROM {_TABLE} ORDER BY L_ORDERKEY LIMIT 1000000",
    ),
    (
        "15columns_1M_ordered_arrow",
        _15_COLUMNS.format(table=_TABLE, order_by="ORDER BY L_ORDERKEY"),
    ),
]

IDS = [stem for stem, _ in QUERIES]


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
@pytest.mark.parametrize("stem,sql", QUERIES, ids=IDS)
def test_select_1M_pandas(perf_test, stem, sql):
    perf_test(
        sql_command=sql,
        fetch_mode="pandas",
        test_name=f"select_{stem}_pandas",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
@pytest.mark.parametrize("stem,sql", QUERIES, ids=IDS)
def test_select_1M_pandas_recorded_http(perf_test, stem, sql):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=sql,
        fetch_mode="pandas",
        test_name=f"select_{stem}_pandas_recorded_http",
    )
