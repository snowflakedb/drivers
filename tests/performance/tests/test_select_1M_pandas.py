"""1M-row pandas / arrow-batch fetch performance (Python driver).

Datatypes match test_select_1M_recorded_http.py. Each query runs e2e and
recorded_http for fetch_mode=pandas (fetch_pandas_all) and arrow_batches
(iterate fetch_arrow_batches).
"""
import pytest
from catalog import TYPE_KEYS, get_sql
from runner.test_types import PerfTestType

# FETCH_MODE is read only by the Python driver app; other drivers ignore it and
# would report identical timings under misleading mode-suffixed names.
pytestmark = pytest.mark.supported_drivers("python")

ITERATIONS = 10
WARMUP_ITERATIONS = 2

FETCH_MODES = ["pandas", "arrow_batches"]
ORDERED_TYPES = ("string", "number", "15columns")


def _stem(type_key: str, *, ordered: bool = False) -> str:
    if ordered:
        return f"{type_key}_1M_ordered_arrow"
    return f"{type_key}_1M_arrow"


# (name_stem, sql) — name_stem becomes select_{stem}_{fetch_mode}[_recorded_http]
QUERIES = [(_stem(k), get_sql(k, 1_000_000)) for k in TYPE_KEYS] + [
    (_stem(k, ordered=True), get_sql(k, 1_000_000, ordered=True)) for k in ORDERED_TYPES
]

CASES = [
    (stem, sql, fetch_mode)
    for stem, sql in QUERIES
    for fetch_mode in FETCH_MODES
]
IDS = [f"{stem}_{fetch_mode}" for stem, _, fetch_mode in CASES]


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
@pytest.mark.parametrize("stem,sql,fetch_mode", CASES, ids=IDS)
def test_select_1M_pandas(perf_test, stem, sql, fetch_mode):
    perf_test(
        sql_command=sql,
        fetch_mode=fetch_mode,
        test_name=f"select_{stem}_{fetch_mode}",
    )


@pytest.mark.iterations(ITERATIONS)
@pytest.mark.warmup_iterations(WARMUP_ITERATIONS)
@pytest.mark.parametrize("stem,sql,fetch_mode", CASES, ids=IDS)
def test_select_1M_pandas_recorded_http(perf_test, stem, sql, fetch_mode):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=sql,
        fetch_mode=fetch_mode,
        test_name=f"select_{stem}_{fetch_mode}_recorded_http",
    )
