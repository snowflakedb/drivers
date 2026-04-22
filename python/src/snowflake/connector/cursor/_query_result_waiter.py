"""Waiter that polls query status until a query completes or errors."""

from __future__ import annotations

import time

from typing import TYPE_CHECKING

from ..constants import QueryStatus
from ..errors import DatabaseError


if TYPE_CHECKING:
    from ..connection import Connection

_RETRY_PATTERN = [1, 1, 2, 3, 4, 8, 10]
_NO_DATA_MAX_RETRY = 24


class QueryResultWaiter:
    """Polls query status with capped exponential backoff until completion."""

    def __init__(self, connection: Connection, sfqid: str) -> None:
        self._connection = connection
        self._sfqid = sfqid

    def wait(self) -> None:
        """Block until the query completes. Raise on terminal error status."""
        no_data_counter = 0
        retry_idx = 0
        while True:
            status = self._check_status()
            if not self._connection.is_still_running(status):
                break
            if status == QueryStatus.NO_DATA:
                no_data_counter += 1
                if no_data_counter > _NO_DATA_MAX_RETRY:
                    raise DatabaseError(
                        f"Cannot retrieve data on the status of this query. "
                        f"No information returned from server for query '{self._sfqid}'"
                    )
            time.sleep(0.5 * _RETRY_PATTERN[retry_idx])
            if retry_idx < len(_RETRY_PATTERN) - 1:
                retry_idx += 1

    def _check_status(self) -> QueryStatus:
        return self._connection.get_query_status_throw_if_error(self._sfqid)
