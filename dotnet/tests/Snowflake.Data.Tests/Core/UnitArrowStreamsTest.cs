using Apache.Arrow;
using Apache.Arrow.C;
using Apache.Arrow.Ipc;
using Apache.Arrow.Types;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
[Trait("Category", "TestingInfrastructure")]
public sealed class UnitArrowStreamsTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void Int64ColumnEncodesTheStreamAddressAsEightBytes()
    {
        var stream = UnitArrowStreams.Int64Column(1);

        Assert.Equal(sizeof(long), stream.Value.Length);
        Assert.NotEqual(0L, BitConverter.ToInt64(stream.Value.ToByteArray(), 0));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task Int64ColumnExportsOneBatchWithTheValuesInOrderAsync()
    {
        using var stream = Import(UnitArrowStreams.Int64Column(1, null, -5, long.MaxValue));

        var field = Assert.Single(stream.Schema.FieldsList);
        Assert.Equal("C1", field.Name);
        Assert.True(field.IsNullable);
        Assert.IsType<Int64Type>(field.DataType);

        using var batch = await stream.ReadNextRecordBatchAsync().ConfigureAwait(false);
        var column = Assert.IsType<Int64Array>(batch.Column(0));
        Assert.Equal([1L, null, -5L, long.MaxValue], Enumerable.Range(0, column.Length).Select(column.GetValue));
        Assert.Null(await stream.ReadNextRecordBatchAsync().ConfigureAwait(false));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public async Task Int64ColumnWithoutValuesExportsOneEmptyBatchAsync()
    {
        using var stream = Import(UnitArrowStreams.Int64Column());

        Assert.Equal("C1", Assert.Single(stream.Schema.FieldsList).Name);
        using var batch = await stream.ReadNextRecordBatchAsync().ConfigureAwait(false);
        Assert.Equal(0, batch.Length);
        Assert.Null(await stream.ReadNextRecordBatchAsync().ConfigureAwait(false));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EachCallExportsItsOwnStream()
    {
        var first = UnitArrowStreams.Int64Column(1);
        var second = UnitArrowStreams.Int64Column(1);

        Assert.NotEqual(first.Value, second.Value);
        using var firstImported = Import(first);
        using var secondImported = Import(second);
    }

    private static unsafe IArrowArrayStream Import(ArrowArrayStreamPtr stream)
    {
        var address = (nint)BitConverter.ToInt64(stream.Value.ToByteArray(), 0);
        return CArrowArrayStreamImporter.ImportArrayStream((CArrowArrayStream*)address);
    }
}
