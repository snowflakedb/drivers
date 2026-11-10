using Snowflake.Data.Core.Handles;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.Extensions;

/// <summary>
/// Builds the sf_core requests that target one result set handle.
/// </summary>
internal static class ResultSetHandleExtensions
{
    extension(NativeResultSetHandle resultSet)
    {
        /// <summary>A request for the Arrow stream over the result set's rows.</summary>
        internal ResultSetGetStreamRequest BuildGetStreamRequest() =>
            new() { ResultSetHandle = resultSet.Handle };

        /// <summary>A request that releases the result set handle in sf_core.</summary>
        internal ResultSetReleaseRequest BuildReleaseRequest() =>
            new() { ResultSetHandle = resultSet.Handle };
    }
}
