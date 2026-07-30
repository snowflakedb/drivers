from __future__ import annotations

import io

from typing import TYPE_CHECKING

from .._internal.api_client.client_api import CHUNK_SIZE, core_driver


if TYPE_CHECKING:
    from _typeshed import WriteableBuffer

    from .._internal.protobuf_gen.database_driver_v1_pb2 import DownloadStreamHandle


class _ChunkedDownloadReader(io.RawIOBase):
    """Lazily reads a stage file via the Begin/Chunk/Close RPCs.

    Nothing beyond Begin fires until read; each ``read()`` pulls one chunk
    via :func:`readinto`, so at most one chunk sits in memory. This holds for
    iteration and for ``read(size)`` with a non-negative *size* — ``read(-1)``
    / ``readall()`` (the ``io.RawIOBase`` default for a full read) loops
    ``readinto`` to EOF and accumulates the whole file in memory, inherent to
    a read-all. The session is released exactly once, on EOF, on error, or on
    close (including via ``io.IOBase.__del__`` as a safety net) — whichever
    happens first.
    """

    def __init__(self, download_handle: DownloadStreamHandle) -> None:
        super().__init__()
        self._download_handle = download_handle
        self._pending = b""
        self._eof = False
        self._handle_released = False

    def readable(self) -> bool:
        return True

    def readinto(self, b: WriteableBuffer) -> int:
        # Loop rather than pull-once: a chunk with data=b"" and eof=False is a
        # legal (if unusual) core response, and returning 0 here would make
        # io.RawIOBase.readall() treat it as EOF and silently truncate.
        while not self._pending and not self._eof:
            self._pull_chunk()
        view = memoryview(b)
        n = min(len(view), len(self._pending))
        view[:n] = self._pending[:n]
        self._pending = self._pending[n:]
        return n

    def _pull_chunk(self) -> None:
        """Fetch one core-side chunk into ``_pending``; release on EOF/error."""
        try:
            response = core_driver.download_stream_chunk(self._download_handle, CHUNK_SIZE)
        except BaseException:
            self._release_handle()
            raise
        self._pending = response.data
        if response.eof:
            self._eof = True
            self._release_handle()

    def _release_handle(self) -> None:
        if self._handle_released:
            return
        self._handle_released = True
        core_driver.download_stream_close(self._download_handle)

    def close(self) -> None:
        if self.closed:
            return
        try:
            self._release_handle()
        finally:
            super().close()
