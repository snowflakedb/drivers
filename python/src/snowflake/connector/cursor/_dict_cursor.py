"""Concrete DictCursor — returns dict rows."""

from __future__ import annotations

from ._base import DictRow, SnowflakeCursorBase


class DictCursor(SnowflakeCursorBase):
    """Cursor returning results as dictionaries with column names as keys.

    Usage::

        with connection.cursor(DictCursor) as cur:
            cur.execute("SELECT 1 AS id, 'hello' AS name")
            row = cur.fetchone()
            # row == {"ID": 1, "NAME": "hello"}
    """

    @property
    def _use_dict_result(self) -> bool:
        return True

    def fetchone(self) -> DictRow | None:
        """
        Fetch the next row of a query result set as a dictionary.

        Returns:
            dict: Next row as a dictionary with column names as keys,
                  or None when no more data is available
        """
        row = self._fetchone()
        if not (row is None or isinstance(row, dict)):
            raise TypeError(f"fetchone got unexpected result: {row}")
        return row

    def fetchmany(self, size: int | None = None) -> list[DictRow]:
        """
        Fetch the next set of rows as dictionaries.

        Args:
            size (int): Number of rows to fetch (defaults to arraysize)

        Returns:
            list[dict]: List of rows as dictionaries
        """
        return super().fetchmany(size)

    def fetchall(self) -> list[DictRow]:
        """
        Fetch all (remaining) rows as dictionaries.

        Returns:
            list[dict]: List of all remaining rows as dictionaries
        """
        return super().fetchall()
