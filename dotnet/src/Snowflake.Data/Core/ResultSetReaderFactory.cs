using System.Data.Common;
using Snowflake.Data.Core.Handles;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core;

internal sealed class ResultSetReaderFactory : IResultSetReaderFactory
{
    /// <summary>The shared stateless instance.</summary>
    internal static IResultSetReaderFactory Instance { get; } = new ResultSetReaderFactory();

    public DbDataReader Create(NativeResultSetHandle resultSetHandle, ArrowArrayStreamPtr stream,
        ResultSetDescriptor descriptor, int recordsAffected) =>
        new SnowflakeDbDataReader(resultSetHandle, stream, descriptor, recordsAffected, schemaOnly: false);

    public DbDataReader CreateForSchemaOnly(ArrowArrayStreamPtr stream, ResultSetDescriptor descriptor, int recordsAffected) =>
        new SnowflakeDbDataReader(null, stream, descriptor, recordsAffected, schemaOnly: true);
}
