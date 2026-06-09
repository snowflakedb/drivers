"""Waiter that polls query status until a query completes or errors."""

from __future__ import annotations

import asyncio
import time

from typing import TYPE_CHECKING

from ...constants import QueryStatus
from ...errors import DatabaseError


if TYPE_CHECKING:
    from ...connection import Connection

_RETRY_PATTERN = [1, 1, 2, 3, 4, 8, 10]
_NO_DATA_MAX_RETRY = 24


def _no_data_error(sfqid: str) -> DatabaseError:
    return DatabaseError(
        f"Cannot retrieve data on the status of this query. No information returned from server for query '{sfqid}'"
    )


def _advance_no_data_counter(status: QueryStatus, no_data_counter: int, sfqid: str) -> int:
    if status != QueryStatus.NO_DATA:
        return no_data_counter
    no_data_counter += 1
    if no_data_counter > _NO_DATA_MAX_RETRY:
        raise _no_data_error(sfqid)
    return no_data_counter


def _poll_step(
    connection: Connection,
    sfqid: str,
    status: QueryStatus,
    no_data_counter: int,
    retry_idx: int,
) -> tuple[float | None, int, int]:
    """Advance one poll iteration.

    Returns ``(sleep_seconds, no_data_counter, retry_idx)``. ``sleep_seconds``
    is ``None`` when polling should stop.
    """
    if not connection.is_still_running(status):
        return None, no_data_counter, retry_idx
    no_data_counter = _advance_no_data_counter(status, no_data_counter, sfqid)
    delay = 0.5 * _RETRY_PATTERN[retry_idx]
    if retry_idx < len(_RETRY_PATTERN) - 1:
        retry_idx += 1
    return delay, no_data_counter, retry_idx


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
            status = self._connection.get_query_status_throw_if_error(self._sfqid)
            delay, no_data_counter, retry_idx = _poll_step(
                self._connection, self._sfqid, status, no_data_counter, retry_idx
            )
            if delay is None:
                return
            time.sleep(delay)

    async def wait_async(self) -> None:
        """Await query completion. Raise on terminal error status."""
        no_data_counter = 0
        retry_idx = 0
        while True:
            # TODO(SNOW-3487088): replace asyncio.to_thread with a native async
            # connection call once the connection layer is ported to async.
            status = await asyncio.to_thread(self._connection.get_query_status_throw_if_error, self._sfqid)
            delay, no_data_counter, retry_idx = _poll_step(
                self._connection, self._sfqid, status, no_data_counter, retry_idx
            )
            if delay is None:
                return
            await asyncio.sleep(delay)
