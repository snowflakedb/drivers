from __future__ import annotations

from ...errors import ProgrammingError
from ..arrow_stream_utils import release_arrow_stream
from ..protobuf_gen.database_driver_v1_pb2 import (
    MultiStatementResult,
    PrepareResult,
    ResultSetResponse,
)
from ..statement_utils import (
    extract_rowcount,
    extract_sqlstate,
    get_stream_ptr,
)
from .result_metadata import QueryResultStats, ResultMetadata


class MultiStatementQueryResultState:
    """Cursor-level navigation state for multi-statement query results.

    Tracks the child query IDs returned by the server, which child
    we're currently positioned on, and the parent query ID that
    groups them together.
    """

    __slots__ = ("parent_qid", "child_query_ids", "_next_index")

    def __init__(
        self,
        parent_qid: str | None,
        child_query_ids: list[str],
    ) -> None:
        self.parent_qid = parent_qid
        self.child_query_ids = child_query_ids
        self._next_index = 0

    def advance(self) -> str | None:
        """Return the next child query ID and advance the index, or None if exhausted."""
        if self._next_index >= len(self.child_query_ids):
            return None
        qid = self.child_query_ids[self._next_index]
        self._next_index += 1
        return qid

    def current_child_query_id(self) -> str | None:
        """Return the query ID of the currently active child result set."""
        if self._next_index == 0:
            return None
        return self.child_query_ids[self._next_index - 1]

    @staticmethod
    def from_result(multi_result: MultiStatementResult) -> MultiStatementQueryResultState | None:
        """Create from a proto MultiStatementResult, or None if there are no children."""
        query_ids = list(multi_result.query_ids)
        if not query_ids:
            return None
        parent = multi_result.parent
        parent_qid = parent.query_id if parent.query_id else None
        return MultiStatementQueryResultState(
            parent_qid=parent_qid,
            child_query_ids=query_ids,
        )


class QueryResult:
    """Pure metadata about a query execution result."""

    __slots__ = ("description", "sqlstate", "sfqid", "query", "stats", "rowcount", "is_file_transfer")

    def __init__(
        self,
        *,
        description: list[ResultMetadata] | None = None,
        sqlstate: str | None = None,
        sfqid: str | None = None,
        query: str | None = None,
        stats: QueryResultStats | None = None,
        rowcount: int | None = None,
        is_file_transfer: bool = False,
    ) -> None:
        self.description = description
        self.sqlstate = sqlstate
        self.sfqid = sfqid
        self.query = query
        self.stats = stats if stats is not None else QueryResultStats()
        self.rowcount = rowcount
        self.is_file_transfer = is_file_transfer

    def reset(self, closing: bool = False) -> None:
        """Optionally clear the rowcount.

        Only rowcount is reset — description, sqlstate, sfqid, query, and stats
        are left intact for backward compatibility (callers may read them after close/reset).
        They are overwritten wholesale when the cursor's _query_result is replaced on the next execute().
        """
        if not closing:
            self.rowcount = None

    @staticmethod
    def from_prepare_result(result: PrepareResult | None) -> QueryResult:
        stream_ptr = get_stream_ptr(result)
        release_arrow_stream(stream_ptr)

        description = ResultMetadata.create_description(result)
        return QueryResult(
            description=description,
            sqlstate=extract_sqlstate(result),
            sfqid=(result.query_id if result.query_id else None) if result else None,
            query=(result.query if result.query else None) if result else None,
            rowcount=0 if description else None,
        )

    @staticmethod
    def from_programming_error(exc: ProgrammingError) -> QueryResult:
        return QueryResult(
            sqlstate=exc.sqlstate or None,
            sfqid=exc.sfqid or None,
            query=exc.query or None,
        )

    @staticmethod
    def from_result_set_response(
        response: ResultSetResponse,
        query: str | None = None,
    ) -> QueryResult:
        """Create QueryResult from a ResultSetResponse (metadata only).

        Args:
            response: ResultSetResponse containing descriptor metadata.
            query: Optional query text (not available in proto, must be passed separately).

        Returns:
            QueryResult instance with metadata populated.
        """
        descriptor = response.result_descriptor

        return QueryResult(
            description=ResultMetadata.create_description(descriptor),
            sqlstate=extract_sqlstate(descriptor),
            sfqid=descriptor.query_id if descriptor.query_id else None,
            query=query,
            rowcount=extract_rowcount(descriptor),
            is_file_transfer=(
                descriptor.statement_type_id in (0x7101, 0x7102) if descriptor.HasField("statement_type_id") else False
            ),
            stats=(
                QueryResultStats.from_query_stats(descriptor.stats)
                if descriptor.HasField("stats")
                else QueryResultStats()
            ),
        )
