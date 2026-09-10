"""Parameter-binding performance: live e2e plus recorded HTTP (stage is e2e only)."""

import pytest
from binding_catalog import (
    EXECUTEMANY_15COL_INSERT_SQL,
    INLINE_15COL_INSERT_SETUP,
    INLINE_ROW_COUNT,
    SCALAR_BIND_CELL_COUNT,
    SCALAR_TYPE_NAMES,
    STAGE_15COL_INSERT_SETUP,
    STAGE_ROW_COUNT,
    inline_executemany_15col_rows,
    scalar_binding_params,
    scalar_select_sql,
    stage_executemany_15col_rows,
)
from runner.test_types import PerfTestType


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(2)
@pytest.mark.parametrize("type_name", SCALAR_TYPE_NAMES)
def test_bind_scalar_select(perf_test, type_name):
    perf_test(
        test_type=PerfTestType.PARAMETER_BINDING,
        sql_command=scalar_select_sql(SCALAR_BIND_CELL_COUNT),
        binding_mode="execute",
        binding_params=scalar_binding_params(type_name, SCALAR_BIND_CELL_COUNT),
        expected_row_count=1,
        test_name=f"bind_scalar_{type_name}_select_n{SCALAR_BIND_CELL_COUNT}",
    )


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(2)
@pytest.mark.parametrize("type_name", SCALAR_TYPE_NAMES)
def test_bind_scalar_select_recorded_http(perf_test, type_name):
    perf_test(
        test_type=PerfTestType.PARAMETER_BINDING_RECORDED_HTTP,
        sql_command=scalar_select_sql(SCALAR_BIND_CELL_COUNT),
        binding_mode="execute",
        binding_params=scalar_binding_params(type_name, SCALAR_BIND_CELL_COUNT),
        expected_row_count=1,
        test_name=f"bind_scalar_{type_name}_select_n{SCALAR_BIND_CELL_COUNT}_recorded_http",
    )


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(2)
def test_bind_executemany_inline_insert_15columns(perf_test):
    perf_test(
        test_type=PerfTestType.PARAMETER_BINDING,
        sql_command=EXECUTEMANY_15COL_INSERT_SQL,
        setup_queries=list(INLINE_15COL_INSERT_SETUP),
        binding_mode="executemany",
        binding_params=inline_executemany_15col_rows(INLINE_ROW_COUNT),
        expected_row_count=INLINE_ROW_COUNT,
        test_name="bind_executemany_inline_insert_15columns",
    )


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(2)
def test_bind_executemany_inline_insert_15columns_recorded_http(perf_test):
    perf_test(
        test_type=PerfTestType.PARAMETER_BINDING_RECORDED_HTTP,
        sql_command=EXECUTEMANY_15COL_INSERT_SQL,
        setup_queries=list(INLINE_15COL_INSERT_SETUP),
        binding_mode="executemany",
        binding_params=inline_executemany_15col_rows(INLINE_ROW_COUNT),
        expected_row_count=INLINE_ROW_COUNT,
        test_name="bind_executemany_inline_insert_15columns_recorded_http",
    )


@pytest.mark.supported_drivers("python")
@pytest.mark.iterations(8)
@pytest.mark.warmup_iterations(2)
def test_bind_executemany_stage_insert_15columns(perf_test):
    perf_test(
        test_type=PerfTestType.PARAMETER_BINDING,
        sql_command=EXECUTEMANY_15COL_INSERT_SQL,
        setup_queries=list(STAGE_15COL_INSERT_SETUP),
        binding_mode="executemany",
        binding_params=stage_executemany_15col_rows(STAGE_ROW_COUNT),
        expected_row_count=STAGE_ROW_COUNT,
        test_name="bind_executemany_stage_insert_15columns",
    )
