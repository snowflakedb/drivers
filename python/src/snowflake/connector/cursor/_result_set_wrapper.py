from __future__ import annotations

import logging

from .._internal.api_client.client_api import core_driver
from .._internal.errorcode import ER_NO_DATA_FOUND
from .._internal.protobuf_gen.database_driver_v1_pb2 import (
    ResultSetGetChunksResponse,
    ResultSetHandle,
)
from .._internal.statement_utils import get_stream_ptr
from ..errors import ProgrammingError


logger = logging.getLogger(__name__)


class _ResultSetWrapper:
    """Owns a ``ResultSetHandle`` and guards against misuse.

    Lifecycle: ``replace()`` releases the previous handle before adopting a
    new one; ``release()`` frees the current handle.  ``__del__`` acts as a
    safety net for handles that were never explicitly released.

    The wrapper tracks whether the Arrow stream has already been consumed.
    A second call to :meth:`get_arrow_stream_ptr` raises instead of
    silently re-fetching.  (The core layer *does* support re-fetching via a
    slow path, but the Python wrapper prevents it to catch accidental
    double-consumption.)
    """

    __slots__ = ("_handle", "_stream_consumed")

    def __init__(self, handle: ResultSetHandle | None = None) -> None:
        self._handle: ResultSetHandle | None = handle
        self._stream_consumed: bool = False

    # -- handle management --------------------------------------------------

    def replace(self, new_handle: ResultSetHandle | None) -> None:
        """Release the current handle (if any) and adopt *new_handle*."""
        self._do_release()
        self._handle = new_handle
        self._stream_consumed = False

    def release(self) -> None:
        """Explicitly release the handle."""
        self._do_release()

    def __del__(self) -> None:
        self._do_release()

    def _do_release(self) -> None:
        handle = self._handle
        if handle is None:
            return
        self._handle = None
        try:
            core_driver.result_set_release(result_set_handle=handle)
        except Exception:
            logger.warning("Failed to release ResultSet handle", exc_info=True)

    # -- data access --------------------------------------------------------

    def get_arrow_stream_ptr(self) -> int:
        """Fetch the Arrow stream and return the raw C pointer.

        The stream can only be consumed once per result set.  A second call
        raises ``ProgrammingError`` instead of silently hitting Snowflake.

        Raises:
            ProgrammingError: If no handle is held or the stream was already consumed.
        """
        if self._handle is None:
            raise ProgrammingError(
                msg="No results available (not produced by this query)",
                errno=ER_NO_DATA_FOUND,
            )
        if self._stream_consumed:
            raise ProgrammingError(
                msg="No results available (arrow stream already consumed)",
                errno=ER_NO_DATA_FOUND,
            )
        response = core_driver.result_set_get_stream(result_set_handle=self._handle)
        self._stream_consumed = True
        return get_stream_ptr(response)

    def get_chunks(self) -> ResultSetGetChunksResponse | None:
        """Call ``result_set_get_chunks`` on the held handle, or return ``None``.

        Safe to call multiple times and independently of :meth:`get_arrow_stream_ptr` —
        the core layer re-fetches chunk metadata from Snowflake if the cached copy
        was already consumed.
        """
        if self._handle is None:
            return None
        return core_driver.result_set_get_chunks(result_set_handle=self._handle)
