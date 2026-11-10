using Apache.Arrow;
using Google.Protobuf;
using Snowflake.Data.Proto;
using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests.Core;

/// <summary>
/// Arrow C streams for unit tests that open a <see cref="SnowflakeDbDataReader"/>, encoded the way sf_core
/// sends the stream address.
/// </summary>
internal static class UnitArrowStreams
{
    /// <summary>An exported stream of one batch with a single nullable int64 column named "C1".</summary>
    internal static ArrowArrayStreamPtr Int64Column(params long?[] values) =>
        Int64Column("C1", values);

    /// <summary>An exported stream of one batch with a single nullable int64 column</summary>
    internal static ArrowArrayStreamPtr Int64Column(string columnName, params long?[] values)
    {
        var batch = new RecordBatch.Builder()
            .Append(columnName, nullable: true, column => column.Int64(builder =>
            {
                foreach (var value in values)
                {
                    if (value is { } present)
                        builder.Append(present);
                    else
                        builder.AppendNull();
                }
            }))
            .Build();

        var address = ArrowStreamExporter.Export(batch.Schema, batch);
        var bytes = BitConverter.GetBytes(address);
        var byteString = ByteString.CopyFrom(bytes);
        return new ArrowArrayStreamPtr { Value = byteString };
    }
}
