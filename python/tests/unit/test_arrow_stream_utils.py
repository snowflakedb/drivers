from __future__ import annotations

from unittest.mock import MagicMock, patch

import pytest

from snowflake.connector._internal.arrow_context import ArrowConverterContext
from snowflake.connector._internal.arrow_stream_utils import (
    create_row_iterator,
    create_table_iterator,
)
from snowflake.connector.errors import InternalError


def test_forwards_use_dict_result_on_native_path():
    mock_iterator = MagicMock(name="native_iterator")
    mock_class = MagicMock(return_value=mock_iterator)
    mock_core = MagicMock()
    mock_core.native_arrow_enabled.return_value = True
    mock_core.ArrowStreamIterator = mock_class

    with patch(
        "snowflake.connector._internal.arrow_stream_utils.sf_core_python",
        mock_core,
    ):
        result = create_row_iterator(
            99,
            context=ArrowConverterContext(timezone="UTC"),
            use_dict_result=True,
        )

    assert result is mock_iterator
    mock_class.assert_called_once_with(
        99,
        session_timezone="UTC",
        use_dict_result=True,
        use_numpy=False,
    )


def test_forwards_use_numpy_on_native_path():
    mock_iterator = MagicMock(name="native_iterator")
    mock_class = MagicMock(return_value=mock_iterator)
    mock_core = MagicMock()
    mock_core.native_arrow_enabled.return_value = True
    mock_core.ArrowStreamIterator = mock_class

    with patch(
        "snowflake.connector._internal.arrow_stream_utils.sf_core_python",
        mock_core,
    ):
        result = create_row_iterator(
            99,
            context=ArrowConverterContext(timezone="UTC"),
            use_numpy=True,
        )

    assert result is mock_iterator
    mock_class.assert_called_once_with(
        99,
        session_timezone="UTC",
        use_dict_result=False,
        use_numpy=True,
    )


def test_create_table_iterator_forwards_knobs_on_native_path():
    mock_iterator = MagicMock(name="native_table_iterator")
    mock_class = MagicMock(return_value=mock_iterator)
    mock_core = MagicMock()
    mock_core.native_arrow_enabled.return_value = True
    mock_core.ArrowStreamTableIterator = mock_class
    context = ArrowConverterContext(timezone="UTC")

    with patch(
        "snowflake.connector._internal.arrow_stream_utils.sf_core_python",
        mock_core,
    ):
        result = create_table_iterator(
            99,
            context=context,
            number_to_decimal=True,
            force_microsecond_precision=True,
        )

    assert result is mock_iterator
    mock_class.assert_called_once_with(
        99,
        session_timezone="UTC",
        number_to_decimal=True,
        force_microsecond_precision=True,
    )


def test_create_table_iterator_uses_cython_when_native_arrow_is_off():
    mock_cython = MagicMock(name="cython_table_iterator")
    mock_core = MagicMock()
    mock_core.native_arrow_enabled.return_value = False
    context = ArrowConverterContext(timezone="UTC")

    with (
        patch(
            "snowflake.connector._internal.arrow_stream_utils.sf_core_python",
            mock_core,
        ),
        patch(
            "snowflake.connector._internal.arrow_stream_utils.CythonArrowStreamTableIterator",
            return_value=mock_cython,
        ) as mock_cls,
    ):
        result = create_table_iterator(
            99,
            context=context,
            number_to_decimal=True,
            force_microsecond_precision=True,
        )

    assert result is mock_cython
    mock_cls.assert_called_once_with(
        99,
        context,
        number_to_decimal=True,
        force_microsecond_precision=True,
    )


def test_create_table_iterator_releases_stream_when_native_class_is_missing():
    class _CoreWithoutTableIterator:
        def native_arrow_enabled(self) -> bool:
            return True

    context = ArrowConverterContext(timezone="UTC")

    with (
        patch(
            "snowflake.connector._internal.arrow_stream_utils.sf_core_python",
            _CoreWithoutTableIterator(),
        ),
        patch(
            "snowflake.connector._internal.arrow_stream_utils.release_arrow_stream",
        ) as release,
        pytest.raises(InternalError, match="ArrowStreamTableIterator"),
    ):
        create_table_iterator(99, context=context)

    release.assert_called_once_with(99)
