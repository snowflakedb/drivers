using Snowflake.Data.Client;
using Snowflake.Data.Core;
using Snowflake.Data.Core.ResultSet;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class ResultSetReaderFactoryTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CreateOpensAReaderOverTheBatchesWithTheGivenCount()
    {
        var source = new FakeRecordBatchSource(UnitBatches.Batch(UnitBatches.Int64(42)));

        using var reader = ResultSetReaderFactory.Instance.Create(source, UnitBatches.Descriptor(UnitBatches.FixedColumn()), 1);

        var snowflakeReader = Assert.IsType<SnowflakeDbDataReader>(reader);
        Assert.Equal("C1", snowflakeReader.GetName(0));
        Assert.Equal(1, snowflakeReader.RecordsAffected);
        Assert.True(snowflakeReader.Read());
        Assert.Equal(42L, snowflakeReader.GetValue(0));
        Assert.False(snowflakeReader.Read());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ReaderFromCreateDisposesTheBatchesWhenClosed()
    {
        var source = new FakeRecordBatchSource();
        var reader = ResultSetReaderFactory.Instance.Create(source, UnitBatches.Descriptor(UnitBatches.FixedColumn()), -1);
        Assert.Equal(0, source.DisposeCalls);

        reader.Close();

        Assert.Equal(1, source.DisposeCalls);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CreateForSchemaOnlyReportsTheColumnsAndNoRows()
    {
        using var reader = ResultSetReaderFactory.Instance.CreateForSchemaOnly(UnitBatches.Descriptor(UnitBatches.FixedColumn(), UnitBatches.TextColumn("C2")));

        var snowflakeReader = Assert.IsType<SnowflakeDbDataReader>(reader);
        Assert.Equal(2, snowflakeReader.FieldCount);
        Assert.Equal("C2", snowflakeReader.GetName(1));
        Assert.Equal(typeof(long), snowflakeReader.GetFieldType(0));
        Assert.Equal(-1, snowflakeReader.RecordsAffected);
        Assert.False(snowflakeReader.HasRows);
        Assert.False(snowflakeReader.Read());
    }
}
