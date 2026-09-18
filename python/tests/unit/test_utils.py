"""Unit tests for snowflake.connector._internal.utils."""

import pytest

from snowflake.connector._internal.errorcode import ER_INVALID_VALUE
from snowflake.connector._internal.utils import _coerce_executemany_params
from snowflake.connector.errors import ProgrammingError


class TestCoerceExecutemanyParams:
    def test_none_becomes_empty_tuple(self):
        assert _coerce_executemany_params(None) == ()

    def test_list_is_returned_unchanged(self):
        rows = [(1,), (2,)]
        assert _coerce_executemany_params(rows) is rows

    def test_tuple_is_returned_unchanged(self):
        rows = ((1,), (2,))
        assert _coerce_executemany_params(rows) is rows

    def test_generator_is_materialized(self):
        assert _coerce_executemany_params(row for row in [(1,), (2,)]) == [(1,), (2,)]

    def test_empty_generator_is_materialized_to_empty_list(self):
        assert _coerce_executemany_params(row for row in []) == []

    def test_map_is_materialized(self):
        def as_row(i: int) -> tuple[int]:
            return (i,)

        assert _coerce_executemany_params(map(as_row, (1, 2))) == [(1,), (2,)]

    def test_non_iterable_raises_programming_error(self):
        with pytest.raises(ProgrammingError, match="seq_of_parameters must be an iterable") as exc_info:
            _coerce_executemany_params(1)
        assert exc_info.value.errno == ER_INVALID_VALUE
