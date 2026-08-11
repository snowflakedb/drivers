"""Fetch performance for mid-size result sets (Python driver).

10k / 100k rows sit between percentile customer sizes (1/15/400) and the 1M
throughput suite. Datatypes match the core 1M SELECT matrix.
Modes: fetchall, fetchone, pandas. Each case runs e2e and recorded_http.

FETCH_MODE is read only by the Python driver app — Core/ODBC/JDBC ignore it.
"""
import pytest
from runner.test_types import PerfTestType

ITERATIONS = 10
WARMUP_ITERATIONS = 2

ROW_COUNTS = [(10_000, "10k"), (100_000, "100k")]
FETCH_MODES = ["fetchall", "fetchone", "pandas"]

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
            LIMIT {n}
        """

# (dtype_label, sql_template) — core 1M SELECT datatype set
QUERIES = [
    ("string", f"SELECT L_COMMENT FROM {_TABLE} LIMIT {{n}}"),
    ("number", f"SELECT L_LINENUMBER::INT FROM {_TABLE} LIMIT {{n}}"),
    ("date", f"SELECT L_SHIPDATE FROM {_TABLE} LIMIT {{n}}"),
    ("timestamp_ntz", f"SELECT L_SHIPDATE::TIMESTAMP_NTZ FROM {_TABLE} LIMIT {{n}}"),
    ("15columns", _15_COLUMNS.format(table=_TABLE, n="{n}")),
]

CASES = [
    (row_count, dtype, fetch_mode, sql_template)
    for row_count, _ in ROW_COUNTS
    for dtype, sql_template in QUERIES
    for fetch_mode in FETCH_MODES
]
IDS = [
    f"{dtype}_{label}_arrow_{fetch_mode}"
    for _, label in ROW_COUNTS
    for dtype, _ in QUERIES
    for fetch_mode in FETCH_MODES
]


def _size_label(row_count: int) -> str:
    return "10k" if row_count == 10_000 else "100k"


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
@pytest.mark.parametrize("row_count,dtype,fetch_mode,sql_template", CASES, ids=IDS)
def test_select_mid(perf_test, row_count, dtype, fetch_mode, sql_template):
    label = _size_label(row_count)
    perf_test(
        sql_command=sql_template.format(n=row_count),
        fetch_mode=fetch_mode,
        test_name=f"select_{dtype}_{label}_arrow_{fetch_mode}",
    )


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
@pytest.mark.parametrize("row_count,dtype,fetch_mode,sql_template", CASES, ids=IDS)
def test_select_mid_recorded_http(perf_test, row_count, dtype, fetch_mode, sql_template):
    label = _size_label(row_count)
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=sql_template.format(n=row_count),
        fetch_mode=fetch_mode,
        test_name=f"select_{dtype}_{label}_arrow_{fetch_mode}_recorded_http",
    )
