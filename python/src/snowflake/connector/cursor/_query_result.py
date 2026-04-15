from __future__ import annotations

from .._internal.arrow_stream_utils import release_arrow_stream
from .._internal.protobuf_gen.database_driver_v1_pb2 import ExecuteResult, PrepareResult
from .._internal.statement_utils import extract_rowcount, extract_sqlstate, get_stream_ptr
from ..errors import ProgrammingError
from ._result_metadata import QueryResultStats, ResultMetadata


class _QueryResult:
    __slots__ = ("description", "sqlstate", "sfqid", "query", "stats", "rowcount", "_stream_ptr")

    def __init__(
        self,
        *,
        description: list[ResultMetadata] | None = None,
        sqlstate: str | None = None,
        sfqid: str | None = None,
        query: str | None = None,
        stats: QueryResultStats | None = None,
        rowcount: int | None = None,
        _stream_ptr: int | None = None,
    ) -> None:
        self.description = description
        self.sqlstate = sqlstate
        self.sfqid = sfqid
        self.query = query
        self.stats = stats if stats is not None else QueryResultStats()
        self.rowcount = rowcount
        self._stream_ptr = _stream_ptr

    def __del__(self) -> None:
        # Safety net: release the native ArrowArrayStream if it was neither
        # consumed (via consume_stream) nor explicitly freed (via reset).
        # This guards against leaks when a _QueryResult is replaced on the
        # cursor (e.g. executemany loop, error paths) without a prior reset().
        # The try/except is intentional — during interpreter shutdown, modules
        # and builtins referenced by release_arrow_stream may already be torn
        # down, so any call here is best-effort only.
        try:
            if self._stream_ptr:
                release_arrow_stream(self._stream_ptr)
        except Exception:
            pass

    def consume_stream(self) -> int:
        """Take ownership of the arrow stream pointer.

        Returns the stream pointer and clears it from this result.
        After this call the stream is the caller's responsibility;
        reset() will not attempt to release it.

        Raises:
            ProgrammingError: If no stream is available (already consumed,
                never present, or result was from a non-query statement).
        """
        ptr = self._stream_ptr
        if not ptr:
            raise ProgrammingError("No arrow stream available (already consumed or not produced by this query)")
        self._stream_ptr = None
        return ptr

    def reset(self, closing: bool = False) -> None:
        """Release the arrow stream and optionally clear rowcount.

        Only stream and rowcount are reset — description, sqlstate, sfqid,
        query, and stats are left intact for backward compatibility (callers
        may read them after close/reset).  They are overwritten wholesale
        when the cursor's _query_result is replaced on the next execute().
        """
        release_arrow_stream(self._stream_ptr)
        self._stream_ptr = None

        if not closing:
            self.rowcount = None

    @staticmethod
    def from_execute_result(result: ExecuteResult | None) -> _QueryResult:
        return _QueryResult(
            description=ResultMetadata.create_description(result),
            sqlstate=extract_sqlstate(result),
            sfqid=(result.query_id if result.query_id else None) if result else None,
            query=(result.query if result.query else None) if result else None,
            rowcount=extract_rowcount(result),
            _stream_ptr=get_stream_ptr(result),
            stats=(
                QueryResultStats.from_query_stats(result.stats)
                if (result and result.HasField("stats"))
                else QueryResultStats()
            ),
        )

    @staticmethod
    def from_prepare_result(result: PrepareResult | None) -> _QueryResult:
        stream_ptr = get_stream_ptr(result)
        release_arrow_stream(stream_ptr)

        description = ResultMetadata.create_description(result)
        return _QueryResult(
            description=description,
            sqlstate=extract_sqlstate(result),
            sfqid=(result.query_id if result.query_id else None) if result else None,
            query=(result.query if result.query else None) if result else None,
            rowcount=0 if description else None,
        )

    @staticmethod
    def from_programming_error(exc: ProgrammingError) -> _QueryResult:
        return _QueryResult(
            sqlstate=exc.sqlstate or None,
            sfqid=exc.sfqid or None,
            query=exc.query or None,
        )
