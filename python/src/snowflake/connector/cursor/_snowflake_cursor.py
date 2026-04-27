"""Concrete SnowflakeCursor — returns tuple rows."""

from __future__ import annotations

from ._base import Row, SnowflakeCursorBase


class SnowflakeCursor(SnowflakeCursorBase):
    """Cursor returning results as tuples (default).

    This is the standard cursor returned by ``connection.cursor()``.
    """

    @property
    def _use_dict_result(self) -> bool:
        return False

    def fetchone(self) -> Row | None:
        """
        Fetch the next row of a query result set.

        Returns:
            tuple: Next row, or None when no more data is available
        """
        row = self._fetchone()
        if not (row is None or isinstance(row, tuple)):
            raise TypeError(f"fetchone got unexpected result: {row}")
        return row

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
