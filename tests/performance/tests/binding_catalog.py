"""Parameter-binding perf scenario definitions."""

from __future__ import annotations

from typing import Any

from catalog import TYPE_KEYS

INLINE_ROW_COUNT = 1000
STAGE_ROW_COUNT = 1000
# One SELECT row with N same-type placeholders - N cells in the inline JSON bind payload.
SCALAR_BIND_CELL_COUNT = 2000

BIND_PERF_TABLE = "bind_perf_wide_t"

SCALAR_BIND_TYPE_KEYS = tuple(key for key in TYPE_KEYS if key != "15columns")

_SCALAR_SAMPLE: dict[str, Any] = {
    "string": 'a"b,c\n',
    "number": 42,
    "date": "1998-01-15",
    "float": 3.14,
    "double": 1.2345678901234567,
    "boolean": True,
    "timestamp_ntz": "2024-06-15 13:45:00",
    "timestamp_tz": "2024-06-15 13:45:00+02:00",
    "time": "13:45:00",
    "binary": [0, 1, 2, 255],
    "null": None,
}

SCALAR_TYPE_NAMES: tuple[str, ...] = SCALAR_BIND_TYPE_KEYS + ("null",)


def scalar_select_sql(cell_count: int = SCALAR_BIND_CELL_COUNT) -> str:
    return "SELECT " + ", ".join("?" for _ in range(cell_count))


def scalar_binding_params(
    type_name: str,
    cell_count: int = SCALAR_BIND_CELL_COUNT,
) -> list[Any]:
    sample = _SCALAR_SAMPLE[type_name]
    return [sample for _ in range(cell_count)]

# TPCH LINEITEM column layout (matches select_15columns_* perf tests)
LINEITEM_15_COLUMNS_DDL = f"""
CREATE OR REPLACE TEMPORARY TABLE {BIND_PERF_TABLE} (
    L_ORDERKEY INT,
    L_PARTKEY INT,
    L_SUPPKEY INT,
    L_LINENUMBER INT,
    L_QUANTITY FLOAT,
    L_EXTENDEDPRICE FLOAT,
    L_DISCOUNT FLOAT,
    L_TAX FLOAT,
    L_RETURNFLAG VARCHAR(1),
    L_LINESTATUS VARCHAR(1),
    L_SHIPDATE DATE,
    L_COMMITDATE DATE,
    L_RECEIPTDATE DATE,
    L_SHIPINSTRUCT VARCHAR(25),
    L_COMMENT VARCHAR(44)
)
"""

EXECUTEMANY_15COL_INSERT_SQL = (
    f"INSERT INTO {BIND_PERF_TABLE} VALUES ("
    + ", ".join("?" for _ in range(15))
    + ")"
)

INLINE_15COL_INSERT_SETUP = (
    LINEITEM_15_COLUMNS_DDL,
    "ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = 100000",
)

STAGE_15COL_INSERT_SETUP = (
    LINEITEM_15_COLUMNS_DDL,
    "ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = 1000",
)


def lineitem_15col_row(i: int, comment: str) -> tuple[Any, ...]:
    day = (i % 28) + 1
    return (
        i,
        i % 200_000,
        i % 10_000,
        (i % 7) + 1,
        float((i % 50) + 1),
        round(i * 1.5, 2),
        0.05,
        0.02,
        "N",
        "O",
        f"1998-01-{day:02d}",
        f"1998-02-{day:02d}",
        f"1998-03-{day:02d}",
        "DELIVER IN PERSON",
        comment,
    )


def inline_executemany_15col_rows(count: int = INLINE_ROW_COUNT) -> list[tuple[Any, ...]]:
    return [lineitem_15col_row(i, f"row-{i}") for i in range(count)]


def stage_executemany_15col_rows(count: int = STAGE_ROW_COUNT) -> list[tuple[Any, ...]]:
    return [
        lineitem_15col_row(i, f"val,{i}" if i % 10 == 0 else f"row-{i}")
        for i in range(count)
    ]
