using System.Data.Common;
using Snowflake.Data.Core.DataConversion.ResultSetConversion;
using Snowflake.Data.Core.Session;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.ResultSet;

/// <summary>
/// Creates <see cref="SnowflakeDbDataReader"/> instances over a <see cref="SnowflakeResultSet"/> that converts values
/// through <see cref="ColumnConverterFactory"/>.
/// </summary>
internal sealed class ResultSetReaderFactory : IResultSetReaderFactory
{
    /// <summary>The shared stateless instance.</summary>
    internal static IResultSetReaderFactory Instance { get; } = new ResultSetReaderFactory();

    public DbDataReader Create(IRecordBatchSource batches, ResultSetDescriptor descriptor, int recordsAffected) =>
        new SnowflakeDbDataReader(ResultSet(batches, descriptor, recordsAffected), LoggerAndTelemetryStubs.GetLogger<SnowflakeDbDataReader>());

    public DbDataReader CreateForSchemaOnly(ResultSetDescriptor descriptor) =>
        new SnowflakeDbDataReader(ResultSet(EmptyRecordBatchSource.Instance, descriptor, recordsAffected: -1), LoggerAndTelemetryStubs.GetLogger<SnowflakeDbDataReader>());

    private static SnowflakeResultSet ResultSet(IRecordBatchSource batches, ResultSetDescriptor descriptor, int recordsAffected) =>
        new(ResultSetInterpreter.Instance, batches, ColumnConverterFactory.Instance, descriptor, recordsAffected,
            LoggerAndTelemetryStubs.GetLogger<SnowflakeResultSet>());
}
