from __future__ import annotations

from unittest.mock import MagicMock, patch

from snowflake.connector._internal.arrow_context import ArrowConverterContext
from snowflake.connector._internal.arrow_stream_utils import create_row_iterator


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
    )
