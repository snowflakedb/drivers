using Snowflake.Data.Core.Handles;
using Snowflake.Data.Interop;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.Extensions;

/// <summary>
/// Builds the sf_core requests that target one result set handle.
/// </summary>
internal static class ResultSetHandleExtensions
{
    extension(NativeResultSetHandle resultSet)
    {
        /// <summary>
        /// A request that opens an Arrow C stream over the result set's rows; sf_core fills the
        /// caller-allocated stream at <paramref name="nativeArrowStream"/>.
        /// </summary>
        internal ResultSetExportStreamRequest BuildGetStreamRequest(out NativeArrowStream nativeArrowStream)
        {
            nativeArrowStream = new NativeArrowStream();

            try
            {
                return new ResultSetExportStreamRequest { ResultSetHandle = resultSet.Handle, OutStream = nativeArrowStream.ToProto() };
            }
            catch
            {
                nativeArrowStream.Dispose();
                throw;
            }
        }

        /// <summary>A request that releases the result set handle in sf_core.</summary>
        internal ResultSetReleaseRequest BuildReleaseRequest() =>
            new() { ResultSetHandle = resultSet.Handle };
    }
}
