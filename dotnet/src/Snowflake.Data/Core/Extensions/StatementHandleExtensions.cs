using Snowflake.Data.Core.Handles;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.Extensions;

/// <summary>
/// Builds the sf_core requests that target one statement handle.
/// </summary>
internal static class StatementHandleExtensions
{
    extension(NativeStatementHandle handle)
    {
        internal StatementSetOptionsRequestBuilder BuildSetOptionsRequest() => new(handle.Handle);

        internal StatementSetSqlQueryRequest BuildSetSqlQueryRequest(string sql) =>
            new() { StmtHandle = handle.Handle, Query = sql };

        internal StatementExecuteQueryRequest BuildExecuteQueryRequest() =>
            new() { StmtHandle = handle.Handle };

        internal StatementPrepareRequest BuildPrepareRequest() =>
            new() { StmtHandle = handle.Handle };

        internal StatementReleaseRequest BuildReleaseRequest() =>
            new() { StmtHandle = handle.Handle };
    }
}
