using Apache.Arrow;
using Google.Protobuf;
using Snowflake.Data.Client;
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

        return NativeWidth(ArrowStreamExporter.Export(batch.Schema, batch));
    }

    /// <summary><paramref name="address"/> as sf_core sends the address of a stream it allocated: native-width bytes.</summary>
    internal static ArrowArrayStreamPtr NativeWidth(nint address)
    {
        var bytes = nint.Size == sizeof(int) ? BitConverter.GetBytes((int)address) : BitConverter.GetBytes((long)address);
        return new ArrowArrayStreamPtr { Value = ByteString.CopyFrom(bytes) };
    }
}
