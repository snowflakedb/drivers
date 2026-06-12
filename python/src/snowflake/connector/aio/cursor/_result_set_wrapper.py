from __future__ import annotations

import logging

from ..._internal.api_client.client_api import async_core_driver
from ..._internal.cursor.result_set_wrapper import ResultSetWrapperBase
from ..._internal.protobuf_gen.database_driver_v1_pb2 import (
    ResultSetGetChunksResponse,
    ResultSetHandle,
)
from ..._internal.statement_utils import get_stream_ptr


logger = logging.getLogger(__name__)


class _ResultSetWrapper(ResultSetWrapperBase):
    """Async result-set handle owner."""

    async def replace(self, new_handle: ResultSetHandle | None) -> None:
        """Release the current handle (if any) and adopt *new_handle*."""
        await self._do_release()
        self._handle = new_handle

    async def release(self) -> None:
        """Explicitly release the handle."""
        await self._do_release()

    async def _do_release(self) -> None:
        handle = self._take_handle()
        if handle is None:
            return
        try:
            await async_core_driver.result_set_release(result_set_handle=handle)
        except Exception:
            logger.warning("Failed to release ResultSet handle", exc_info=True)

    async def get_arrow_stream_ptr(self) -> int:
        """Fetch the Arrow stream and return the raw C pointer.

        Each call builds a fresh stream from the stored result set data.

        Raises:
            ProgrammingError: If no handle is held.
        """
        handle = self._require_handle()
        response = await async_core_driver.result_set_get_stream(result_set_handle=handle)
        return get_stream_ptr(response)

    async def get_chunks(self) -> ResultSetGetChunksResponse | None:
        """Call ``result_set_get_chunks`` on the held handle, or return ``None``.

        Safe to call multiple times and independently of :meth:`get_arrow_stream_ptr` —
        the core layer re-fetches chunk metadata from Snowflake if the cached copy
        was already consumed.
        """
        if self._handle is None:
            return None
        return await async_core_driver.result_set_get_chunks(result_set_handle=self._handle)
