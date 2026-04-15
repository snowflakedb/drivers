"""Unit tests for ResultBatch and related enums."""

import pickle

from types import SimpleNamespace
from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ResultChunk
from snowflake.connector.errors import InterfaceError
from snowflake.connector.result_batch import (
    IterTableStructure,
    IterUnit,
    ResultBatch,
)


# ------------------------------------------------------------------
# Helpers
# ------------------------------------------------------------------


def _make_description(*names: str) -> list:
    """Build a minimal pickleable description list with .name attributes."""
    return [SimpleNamespace(name=n) for n in names]


def _make_batch(connection=None, description=None) -> ResultBatch:
    return ResultBatch(
        chunk=ResultChunk(),
        description=description or _make_description("ID"),
        connection=connection,
    )


# ------------------------------------------------------------------
# StringEnum.of()
# ------------------------------------------------------------------


class TestIterUnit:
    def test_of_returns_member_unchanged(self):
        assert IterUnit.of(IterUnit.ROW_UNIT) is IterUnit.ROW_UNIT
        assert IterUnit.of(IterUnit.TABLE_UNIT) is IterUnit.TABLE_UNIT

    def test_of_coerces_valid_string(self):
        assert IterUnit.of("row") is IterUnit.ROW_UNIT
        assert IterUnit.of("table") is IterUnit.TABLE_UNIT

    def test_of_rejects_invalid_string(self):
        with pytest.raises(ValueError):
            IterUnit.of("invalid")

    def test_of_rejects_none(self):
        with pytest.raises(ValueError):
            IterUnit.of(None)

    def test_of_rejects_wrong_enum_type(self):
        with pytest.raises(ValueError):
            IterUnit.of(IterTableStructure.ARROW)


class TestIterTableStructure:
    def test_of_returns_member_unchanged(self):
        assert IterTableStructure.of(IterTableStructure.ARROW) is IterTableStructure.ARROW
        assert IterTableStructure.of(IterTableStructure.PANDAS) is IterTableStructure.PANDAS

    def test_of_coerces_valid_string(self):
        assert IterTableStructure.of("arrow") is IterTableStructure.ARROW
        assert IterTableStructure.of("pandas") is IterTableStructure.PANDAS

    def test_of_rejects_invalid_string(self):
        with pytest.raises(ValueError):
            IterTableStructure.of("parquet")


# ------------------------------------------------------------------
# ResultBatch.from_chunks()
# ------------------------------------------------------------------


class TestFromChunks:
    def test_returns_none_when_chunks_is_none(self):
        assert ResultBatch.from_chunks(None, _make_description("ID"), MagicMock()) is None

    def test_returns_none_when_description_is_none(self):
        assert ResultBatch.from_chunks([ResultChunk()], None, MagicMock()) is None

    def test_returns_none_when_both_none(self):
        assert ResultBatch.from_chunks(None, None, MagicMock()) is None

    def test_returns_list_of_batches(self):
        chunks = [ResultChunk(), ResultChunk(), ResultChunk()]
        desc = _make_description("A", "B")
        conn = MagicMock()
        batches = ResultBatch.from_chunks(chunks, desc, conn)

        assert len(batches) == 3
        for batch in batches:
            assert isinstance(batch, ResultBatch)
            assert batch.connection is conn

    def test_returns_empty_list_for_empty_chunks(self):
        batches = ResultBatch.from_chunks([], _make_description("ID"), MagicMock())
        assert batches == []


# ------------------------------------------------------------------
# Properties
# ------------------------------------------------------------------


class TestProperties:
    def test_column_names(self):
        desc = _make_description("COL_A", "COL_B", "COL_C")
        batch = _make_batch(description=desc)
        assert batch.column_names == ["COL_A", "COL_B", "COL_C"]

    def test_connection_property_getter(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        assert batch.connection is conn

    def test_connection_property_setter(self):
        batch = _make_batch()
        assert batch.connection is None
        conn = MagicMock()
        batch.connection = conn
        assert batch.connection is conn

    def test_connection_setter_accepts_none(self):
        batch = _make_batch(connection=MagicMock())
        batch.connection = None
        assert batch.connection is None

    def test_rowcount_raises(self):
        with pytest.raises(NotImplementedError):
            _ = _make_batch().rowcount

    def test_compressed_size_raises(self):
        with pytest.raises(NotImplementedError):
            _ = _make_batch().compressed_size

    def test_uncompressed_size_raises(self):
        with pytest.raises(NotImplementedError):
            _ = _make_batch().uncompressed_size


# ------------------------------------------------------------------
# _resolve_connection
# ------------------------------------------------------------------


class TestResolveConnection:
    def test_prefers_explicit_connection(self):
        stored = MagicMock()
        explicit = MagicMock()
        batch = _make_batch(connection=stored)
        assert batch._resolve_connection(explicit) is explicit

    def test_falls_back_to_stored_connection(self):
        stored = MagicMock()
        batch = _make_batch(connection=stored)
        assert batch._resolve_connection() is stored

    def test_raises_when_no_connection(self):
        batch = _make_batch()
        with pytest.raises(InterfaceError, match="not connected"):
            batch._resolve_connection()

    def test_raises_when_explicit_is_none_and_stored_is_none(self):
        batch = _make_batch()
        with pytest.raises(InterfaceError):
            batch._resolve_connection(None)


# ------------------------------------------------------------------
# Pickle round-trip
# ------------------------------------------------------------------


class TestPickle:
    def test_pickle_round_trip_preserves_description(self):
        desc = _make_description("X", "Y")
        batch = _make_batch(connection=MagicMock(), description=desc)

        restored = pickle.loads(pickle.dumps(batch))

        assert restored.column_names == ["X", "Y"]

    def test_pickle_clears_connection(self):
        batch = _make_batch(connection=MagicMock())
        restored = pickle.loads(pickle.dumps(batch))
        assert restored.connection is None

    def test_pickle_round_trip_preserves_chunk(self):
        chunk = ResultChunk()
        batch = ResultBatch(chunk=chunk, description=_make_description("ID"), connection=None)

        restored = pickle.loads(pickle.dumps(batch))
        assert restored._chunk.SerializeToString() == chunk.SerializeToString()


# ------------------------------------------------------------------
# create_iter validation
# ------------------------------------------------------------------


class TestCreateIterValidation:
    def test_rejects_invalid_iter_unit_string(self):
        batch = _make_batch(connection=MagicMock())
        with pytest.raises(ValueError):
            batch.create_iter(iter_unit="bad")

    def test_rejects_invalid_structure_string(self):
        batch = _make_batch(connection=MagicMock())
        with pytest.raises(ValueError):
            batch.create_iter(iter_unit="table", structure="parquet")

    def test_accepts_string_iter_unit(self):
        batch = _make_batch(connection=MagicMock())
        with (
            patch.object(batch, "_fetch_arrow_stream_ptr", return_value=0),
            patch("snowflake.connector.result_batch.create_row_iterator") as mock_iter,
        ):
            mock_iter.return_value = iter([])
            batch.create_iter(iter_unit="row")
            mock_iter.assert_called_once()

    def test_accepts_string_structure(self):
        batch = _make_batch(connection=MagicMock())
        with patch.object(batch, "to_arrow", return_value=MagicMock()) as mock_to_arrow:
            result = batch.create_iter(iter_unit="table", structure="arrow")
            mock_to_arrow.assert_called_once()
            assert list(result)  # should have one element


# ------------------------------------------------------------------
# create_iter dispatch
# ------------------------------------------------------------------


class TestCreateIterDispatch:
    def test_row_unit_calls_create_row_iterator(self):
        batch = _make_batch(connection=MagicMock())
        with (
            patch.object(batch, "_fetch_arrow_stream_ptr", return_value=0),
            patch("snowflake.connector.result_batch.create_row_iterator") as mock_iter,
        ):
            mock_iter.return_value = iter([(1,), (2,)])
            rows = list(batch.create_iter(iter_unit=IterUnit.ROW_UNIT))
        assert rows == [(1,), (2,)]
        mock_iter.assert_called_once_with(0, use_dict_result=False)

    def test_row_unit_with_dict_result(self):
        batch = _make_batch(connection=MagicMock())
        with (
            patch.object(batch, "_fetch_arrow_stream_ptr", return_value=0),
            patch("snowflake.connector.result_batch.create_row_iterator") as mock_iter,
        ):
            mock_iter.return_value = iter([{"id": 1}])
            batch.create_iter(iter_unit=IterUnit.ROW_UNIT, use_dict_result=True)
        mock_iter.assert_called_once_with(0, use_dict_result=True)

    def test_table_unit_pandas_calls_to_pandas(self):
        batch = _make_batch(connection=MagicMock())
        sentinel = MagicMock()
        with patch.object(batch, "to_pandas", return_value=sentinel):
            result = list(batch.create_iter(iter_unit=IterUnit.TABLE_UNIT, structure=IterTableStructure.PANDAS))
        assert result == [sentinel]

    def test_table_unit_arrow_calls_to_arrow(self):
        batch = _make_batch(connection=MagicMock())
        sentinel = MagicMock()
        with patch.object(batch, "to_arrow", return_value=sentinel):
            result = list(batch.create_iter(iter_unit=IterUnit.TABLE_UNIT, structure=IterTableStructure.ARROW))
        assert result == [sentinel]

    def test_table_unit_forwards_conversion_params(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        with patch.object(batch, "to_arrow", return_value=MagicMock()) as mock_to_arrow:
            batch.create_iter(
                iter_unit=IterUnit.TABLE_UNIT,
                structure=IterTableStructure.ARROW,
                number_to_decimal=True,
                force_microsecond_precision=True,
            )
        mock_to_arrow.assert_called_once_with(
            connection=conn,
            number_to_decimal=True,
            force_microsecond_precision=True,
        )


# ------------------------------------------------------------------
# to_arrow / to_pandas connection parameter
# ------------------------------------------------------------------


class TestToArrowConnection:
    @pytest.fixture(autouse=True)
    def _patch_deps(self):
        with patch("snowflake.connector._internal.extras.check_dependency"):
            yield

    def test_to_arrow_uses_explicit_connection(self):
        batch = _make_batch()
        explicit = MagicMock()
        with (
            patch.object(batch, "_fetch_arrow_stream_ptr", return_value=0) as mock_fetch,
            patch("snowflake.connector.result_batch.create_table_iterator"),
            patch("snowflake.connector.result_batch.collect_arrow_table"),
        ):
            batch.to_arrow(connection=explicit)
        mock_fetch.assert_called_once_with(explicit)

    def test_to_arrow_raises_without_connection(self):
        batch = _make_batch()
        with pytest.raises(InterfaceError):
            batch.to_arrow()

    def test_to_arrow_forwards_conversion_params(self):
        batch = _make_batch(connection=MagicMock())
        with (
            patch.object(batch, "_fetch_arrow_stream_ptr", return_value=0),
            patch("snowflake.connector.result_batch.create_table_iterator") as mock_table_iter,
            patch("snowflake.connector.result_batch.collect_arrow_table"),
        ):
            batch.to_arrow(number_to_decimal=True, force_microsecond_precision=True)
        mock_table_iter.assert_called_once_with(0, number_to_decimal=True, force_microsecond_precision=True)


class TestToPandasConnection:
    @pytest.fixture(autouse=True)
    def _patch_deps(self):
        with patch("snowflake.connector._internal.extras.check_dependency"):
            yield

    def test_to_pandas_delegates_to_to_arrow(self):
        conn = MagicMock()
        batch = _make_batch(connection=conn)
        mock_table = MagicMock()
        with patch.object(batch, "to_arrow", return_value=mock_table) as mock_to_arrow:
            batch.to_pandas(connection=conn, number_to_decimal=True, force_microsecond_precision=True)
        mock_to_arrow.assert_called_once_with(
            connection=conn,
            number_to_decimal=True,
            force_microsecond_precision=True,
        )
        mock_table.to_pandas.assert_called_once()
