"""Fetch performance for 1M-row result sets.

Bind-mode matrix (ODBC):
  * no suffix / existing names — SQL_C_CHAR (to_string); historical BenchDash baselines
  * `_default` suffix           — SQL_C_DEFAULT (driver-chosen C type); separate charts

BenchDash `test_name` stays `select_{name}` / `select_{name}_recorded_http` so
existing charts keep their series. Pytest node ids are parametrized
(`test_select_1M[string_1M_arrow]`).

SQL API and ADBC opt in here (nodejs-style extra `cases()` + `id_suffix`) so
they share 1M `test_name`s without entering `test_select_1M_recorded_http`.
"""
import pytest
from catalog import NODEJS_UNSUPPORTED_TYPES, TYPE_KEYS, get_sql
from matrix import cases, with_mark
from runner.test_types import PerfTestType

SIZES = ((1_000_000, "1M"),)

SUFFIXES = {
    "python": ("", "_fetchall", "_pandas", "_arrow_batches"),
    "jdbc": ("",),
    "odbc": ("", "_default"),
    "core": ("",),
}

# Separate from SUFFIXES: nodejs has no WireMock/recorded_http support, and
# this dict also feeds test_select_1M_recorded_http, so nodejs must never
# enter it. nodejs also doesn't decode every TYPE_KEYS type yet.
NODEJS_SUFFIXES = {"nodejs": ("",)}
NODEJS_TYPE_KEYS = tuple(t for t in TYPE_KEYS if t not in NODEJS_UNSUPPORTED_TYPES)

CASES = cases(SIZES, SUFFIXES, infix="_arrow", types=TYPE_KEYS)
NODEJS_CASES = cases(
    SIZES, NODEJS_SUFFIXES, infix="_arrow", types=NODEJS_TYPE_KEYS, id_suffix="_nodejs"
)

# Sequential HTTP vs python/jdbc/odbc fetchmany. Same test_name;
# pytest id gets `_sqlapi` so it does not collide with CASES. supports_json
# so the JSON session still covers 1M when Arrow format is unavailable.
SQLAPI_SUFFIXES = {"sqlapi": ("",)}
SQLAPI_CASES = with_mark(
    cases(SIZES, SQLAPI_SUFFIXES, infix="_arrow", types=TYPE_KEYS, id_suffix="_sqlapi"),
    pytest.mark.supports_json,
)

# Arrow-native vs python fetch_arrow_batches. Same test_name; id `_adbc`.
ADBC_SUFFIXES = {"adbc": ("_arrow_batches",)}
ADBC_CASES = cases(
    SIZES, ADBC_SUFFIXES, infix="_arrow", types=TYPE_KEYS, id_suffix="_adbc"
)

ORDERED_SUFFIXES = {
    "python": ("", "_fetchall", "_arrow_batches"),
    "jdbc": ("",),
    "odbc": ("",),
    "core": ("",),
}

ORDERED_CASES = cases(
    SIZES, ORDERED_SUFFIXES, infix="_ordered_arrow", types=("string", "number"),
)

E2E_CASES = CASES + NODEJS_CASES + SQLAPI_CASES + ADBC_CASES


@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", E2E_CASES)
def test_select_1M(perf_test, row_count, dtype, name, fetch_mode, bind_mode):
    perf_test(
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}",
    )


@pytest.mark.skip(reason="ORDER BY SELECT cases disabled for now")
@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", ORDERED_CASES)
def test_select_1M_ordered(perf_test, row_count, dtype, name, fetch_mode, bind_mode):
    perf_test(
        sql_command=get_sql(dtype, row_count, ordered=True),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}",
    )


@pytest.mark.iterations(5)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", CASES)
def test_select_1M_recorded_http(perf_test, row_count, dtype, name, fetch_mode, bind_mode):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql(dtype, row_count),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}_recorded_http",
    )


@pytest.mark.skip(reason="ORDER BY SELECT cases disabled for now")
@pytest.mark.iterations(5)
@pytest.mark.warmup_iterations(1)
@pytest.mark.parametrize("row_count,dtype,name,fetch_mode,bind_mode", ORDERED_CASES)
def test_select_1M_ordered_recorded_http(
    perf_test, row_count, dtype, name, fetch_mode, bind_mode
):
    perf_test(
        test_type=PerfTestType.SELECT_RECORDED_HTTP,
        sql_command=get_sql(dtype, row_count, ordered=True),
        fetch_mode=fetch_mode,
        bind_mode=bind_mode,
        test_name=f"select_{name}_recorded_http",
    )
