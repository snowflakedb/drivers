"""Unit tests for the ``converter_class`` option and the converters in ``snowflake.connector.converter``."""

import asyncio
import json

from unittest.mock import patch

import pytest

from snowflake.connector._internal.arrow_context import ArrowConverterContext
from snowflake.connector._internal.connection_config_mixin import ConnectionConfigMixin
from snowflake.connector._internal.converting_iterator import wrap_async_row_iterator, wrap_row_iterator
from snowflake.connector._internal.cursor.result_metadata import ResultMetadata
from snowflake.connector._internal.type_codes import ARRAY, FIXED, OBJECT, VARIANT
from snowflake.connector.connection_config import ConnectionConfig
from snowflake.connector.errors import DataError, ProgrammingError


class VariantToDictConverter(ArrowConverterContext):
    deserialized_types = frozenset({VARIANT, OBJECT, ARRAY})

    def deserialize(self, value, column):
        return json.loads(value) if isinstance(value, str) else value


def _column(name, type_code):
    return ResultMetadata(name, type_code, None, None, None, None, True)


DESCRIPTION = [_column("N", FIXED), _column("V", VARIANT), _column("O", OBJECT), _column("A", ARRAY)]


class FakeIterator:
    def __init__(self, rows):
        self._rows = list(rows)

    def __iter__(self):
        return self

    def __next__(self):
        if not self._rows:
            raise StopIteration
        return self._rows.pop(0)

    def fetch_many(self, size):
        rows, self._rows = self._rows[:size], self._rows[size:]
        return rows

    def fetch_all(self):
        return self.fetch_many(len(self._rows))


class FakeAsyncIterator:
    def __init__(self, rows):
        self._inner = FakeIterator(rows)

    async def __anext__(self):
        try:
            return next(self._inner)
        except StopIteration:
            raise StopAsyncIteration from None

    async def fetch_next(self, default=None):
        return next(self._inner, default)

    async def fetch_many(self, size):
        return self._inner.fetch_many(size)

    async def fetch_all(self):
        return self._inner.fetch_all()


ROW = (1, '{"a": 1}', '{"b": [1, 2]}', "[1, null]")
CONVERTED = (1, {"a": 1}, {"b": [1, 2]}, [1, None])


class TestColumnConverters:
    def test_base_context_has_no_converters(self):
        assert ArrowConverterContext()._column_converters(DESCRIPTION) is None

    def test_variant_to_dict_converts_semi_structured_columns_only(self):
        converters = VariantToDictConverter()._column_converters(DESCRIPTION)

        assert converters[0] is None
        assert all(conv is not None for conv in converters[1:])

    def test_variant_to_dict_leaves_non_text_values_unchanged(self):
        converter = VariantToDictConverter()

        assert converter.deserialize({"already": "parsed"}, DESCRIPTION[2]) == {"already": "parsed"}

    def test_custom_converter_deserializes_listed_types(self):
        class FixedAsText(ArrowConverterContext):
            deserialized_types = frozenset({FIXED})

            def deserialize(self, value, column):
                return f"{column.name}={value}"

        assert FixedAsText()._column_converters(DESCRIPTION)[0](7) == "N=7"

    def test_deserialize_exception_is_wrapped_as_data_error(self):
        class Boom(ArrowConverterContext):
            deserialized_types = frozenset({FIXED})

            def deserialize(self, value, column):
                raise ValueError("foreign text must not leak")

        converters = Boom()._column_converters(DESCRIPTION)
        with pytest.raises(DataError, match=r"Boom\.deserialize raised ValueError") as excinfo:
            converters[0](1)

        assert "foreign text must not leak" not in str(excinfo.value)
        assert isinstance(excinfo.value.__cause__, ValueError)

    def test_deserialize_driver_error_is_not_rewrapped(self):
        class Boom(ArrowConverterContext):
            deserialized_types = frozenset({FIXED})

            def deserialize(self, value, column):
                raise DataError("already a driver error")

        converters = Boom()._column_converters(DESCRIPTION)
        with pytest.raises(DataError, match="already a driver error"):
            converters[0](1)


class TestRowIteratorWrappers:
    def test_wrap_is_noop_without_converters(self):
        iterator = FakeIterator([ROW])

        assert wrap_row_iterator(iterator, ArrowConverterContext(), DESCRIPTION) is iterator

    def test_next_fetch_many_and_fetch_all(self):
        iterator = wrap_row_iterator(FakeIterator([ROW] * 4), VariantToDictConverter(), DESCRIPTION)

        assert next(iterator) == CONVERTED
        assert iterator.fetch_many(2) == [CONVERTED] * 2
        assert iterator.fetch_all() == [CONVERTED]

    def test_dict_rows_are_converted_by_position(self):
        row = dict(zip(["N", "V", "O", "A"], ROW, strict=False))
        iterator = wrap_row_iterator(FakeIterator([row]), VariantToDictConverter(), DESCRIPTION)

        assert next(iterator) == dict(zip(["N", "V", "O", "A"], CONVERTED, strict=False))

    def test_none_values_bypass_converters(self):
        iterator = wrap_row_iterator(FakeIterator([(None, None, None, None)]), VariantToDictConverter(), DESCRIPTION)

        assert next(iterator) == (None, None, None, None)

    def test_async_iterator(self):
        async def run():
            iterator = wrap_async_row_iterator(FakeAsyncIterator([ROW] * 4), VariantToDictConverter(), DESCRIPTION)
            return [
                await iterator.__anext__(),
                await iterator.fetch_next(),
                await iterator.fetch_many(1),
                await iterator.fetch_all(),
                await iterator.fetch_next("done"),
            ]

        assert asyncio.run(run()) == [CONVERTED, CONVERTED, [CONVERTED], [CONVERTED], "done"]


class TestConverterClassOption:
    def test_is_retained(self):
        config = ConnectionConfig.from_connection_args(user="u", converter_class=VariantToDictConverter)

        assert config.converter_class is VariantToDictConverter

    def test_logs_class_name_when_set(self):
        with patch("snowflake.connector._internal.connection_config_mixin.logger") as mock_logger:
            ConnectionConfig.from_connection_args(user="u", converter_class=VariantToDictConverter)

        mock_logger.debug.assert_called_with("using converter_class %s", "VariantToDictConverter")

    def test_is_python_only(self):
        assert "converter_class" in ConnectionConfigMixin._PYTHON_ONLY

    @pytest.mark.parametrize("value", [object, "VariantToDictConverter", VariantToDictConverter()])
    def test_rejects_non_converter_classes(self, value):
        with pytest.raises(ProgrammingError, match="Invalid converter_class parameter"):
            ConnectionConfig.from_connection_args(user="u", converter_class=value)

    def test_for_connection_uses_configured_class(self):
        class Connection:
            config = ConnectionConfig.from_connection_args(user="u", converter_class=VariantToDictConverter)
            _session_parameters = None

        assert type(ArrowConverterContext._for_connection(Connection())) is VariantToDictConverter
