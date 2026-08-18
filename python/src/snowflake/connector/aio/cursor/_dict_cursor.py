"""Concrete DictCursor — returns dict rows."""

from __future__ import annotations

from typing import cast

from ..._internal.cursor import DictRow
from ..._internal.errorhandler import simplified_error_handling
from ._base import SnowflakeCursorBase


class DictCursor(SnowflakeCursorBase):
    """Cursor returning results as dictionaries with column names as keys.

    Usage::

        async with connection.cursor(DictCursor) as cur:
            await cur.execute("SELECT 1 AS id, 'hello' AS name")
            row = await cur.fetchone()
            # row == {"ID": 1, "NAME": "hello"}
    """

    @property
    def _use_dict_result(self) -> bool:
        return True

    @simplified_error_handling
    async def fetchone(self) -> DictRow | None:
        """
        Fetch the next row of a query result set as a dictionary.

        Returns:
            dict: Next row as a dictionary with column names as keys,
                  or None when no more data is available
        """
        return cast(DictRow | None, await self._fetchone())

    @simplified_error_handling
    async def fetchmany(self, size: int | None = None) -> list[DictRow]:
        """
        Fetch the next set of rows as dictionaries.

        Args:
            size (int): Number of rows to fetch (defaults to arraysize)

        Returns:
            list[dict]: List of rows as dictionaries
        """
        return await super().fetchmany(size)

    async def fetchall(self) -> list[DictRow]:
        """
        Fetch all (remaining) rows as dictionaries.

        Returns:
            list[dict]: List of all remaining rows as dictionaries
        """
        return await super().fetchall()
