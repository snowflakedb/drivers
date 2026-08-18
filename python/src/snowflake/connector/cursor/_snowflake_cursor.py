"""Concrete SnowflakeCursor — returns tuple rows."""

from __future__ import annotations

from typing import cast

from .._internal.cursor import Row
from .._internal.errorhandler import simplified_error_handling
from ._base import SnowflakeCursorBase


class SnowflakeCursor(SnowflakeCursorBase):
    """Cursor returning results as tuples (default).

    This is the standard cursor returned by ``connection.cursor()``.
    """

    @property
    def _use_dict_result(self) -> bool:
        return False

    @simplified_error_handling
    def fetchone(self) -> Row | None:
        """
        Fetch the next row of a query result set.

        Returns:
            tuple: Next row, or None when no more data is available
        """
        return cast(Row | None, self._fetchone())

    @simplified_error_handling
    def fetchmany(self, size: int | None = None) -> list[Row]:
        """
        Fetch the next set of rows of a query result.

        Args:
            size (int): Number of rows to fetch (defaults to arraysize)

        Returns:
            list[tuple]: List of rows as tuples
        """
        return super().fetchmany(size)

    def fetchall(self) -> list[Row]:
        """
        Fetch all (remaining) rows of a query result.

        Returns:
            list[tuple]: List of all remaining rows as tuples
        """
        return super().fetchall()
