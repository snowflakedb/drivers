using System.Data;
using System.Data.Common;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Core.ResultSet;
using Snowflake.Data.Proto;
using Snowflake.Data.Tests.Core;

namespace Snowflake.Data.Tests.Unit.Core;

[Trait("Category", "Unit")]
public sealed class SchemaTableBuilderTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TableHasOldDriverColumnsInOrder()
    {
        var table = SchemaTableBuilder.Build(UnitBatches.Descriptor());

        Assert.Equal("SchemaTable", table.TableName);
        Assert.Equal(
            [
                (SchemaTableColumn.ColumnName, typeof(string)),
                (SchemaTableColumn.ColumnOrdinal, typeof(int)),
                (SchemaTableColumn.ColumnSize, typeof(int)),
                (SchemaTableColumn.NumericPrecision, typeof(int)),
                (SchemaTableColumn.NumericScale, typeof(int)),
                (SchemaTableColumn.DataType, typeof(Type)),
                (SchemaTableColumn.AllowDBNull, typeof(bool)),
                (SchemaTableColumn.ProviderType, typeof(SFDataType)),
            ],
            table.Columns.Cast<DataColumn>().Select(column => (column.ColumnName, column.DataType)));
        Assert.Empty(table.Rows);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void EachColumnBecomesRowWithItsMetadata()
    {
        var descriptor = UnitBatches.Descriptor(
            new ColumnMetadata { Name = "AMOUNT", Type = "fixed", Precision = 20, Scale = 4, Nullable = true },
            new ColumnMetadata { Name = "LABEL", Type = "text", Length = 100, Nullable = false });

        var table = SchemaTableBuilder.Build(descriptor);

        Assert.Equal(2, table.Rows.Count);
        var amount = table.Rows[0];
        Assert.Equal("AMOUNT", amount[SchemaTableColumn.ColumnName]);
        Assert.Equal(0, amount[SchemaTableColumn.ColumnOrdinal]);
        Assert.Equal(0, amount[SchemaTableColumn.ColumnSize]);
        Assert.Equal(20, amount[SchemaTableColumn.NumericPrecision]);
        Assert.Equal(4, amount[SchemaTableColumn.NumericScale]);
        Assert.Equal(typeof(decimal), amount[SchemaTableColumn.DataType]);
        Assert.True((bool)amount[SchemaTableColumn.AllowDBNull]);
        Assert.Equal(SFDataType.FIXED, (SFDataType)amount[SchemaTableColumn.ProviderType]);

        var label = table.Rows[1];
        Assert.Equal("LABEL", label[SchemaTableColumn.ColumnName]);
        Assert.Equal(1, label[SchemaTableColumn.ColumnOrdinal]);
        Assert.Equal(100, label[SchemaTableColumn.ColumnSize]);
        Assert.Equal(typeof(string), label[SchemaTableColumn.DataType]);
        Assert.False((bool)label[SchemaTableColumn.AllowDBNull]);
        Assert.Equal(0, label[SchemaTableColumn.NumericPrecision]);
        Assert.Equal(0, label[SchemaTableColumn.NumericScale]);
        Assert.Equal(SFDataType.TEXT, (SFDataType)label[SchemaTableColumn.ProviderType]);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("fixed", 0L, SFDataType.FIXED, typeof(long))]
    [InlineData("FIXED", 1L, SFDataType.FIXED, typeof(decimal))]
    [InlineData("FIXED", 2L, SFDataType.FIXED, typeof(decimal))]
    [InlineData("real", 0L, SFDataType.REAL, typeof(double))]
    [InlineData("text", 0L, SFDataType.TEXT, typeof(string))]
    [InlineData("variant", 0L, SFDataType.VARIANT, typeof(string))]
    [InlineData("object", 0L, SFDataType.OBJECT, typeof(string))]
    [InlineData("array", 0L, SFDataType.ARRAY, typeof(string))]
    [InlineData("map", 0L, SFDataType.MAP, typeof(string))]
    [InlineData("vector", 0L, SFDataType.VECTOR, typeof(string))]
    [InlineData("decfloat", 0L, SFDataType.DECFLOAT, typeof(string))]
    [InlineData("date", 0L, SFDataType.DATE, typeof(DateTime))]
    [InlineData("time", 9L, SFDataType.TIME, typeof(DateTime))]
    [InlineData("timestamp_ntz", 9L, SFDataType.TIMESTAMP_NTZ, typeof(DateTime))]
    [InlineData("timestamp_ltz", 9L, SFDataType.TIMESTAMP_LTZ, typeof(DateTimeOffset))]
    [InlineData("timestamp_tz", 9L, SFDataType.TIMESTAMP_TZ, typeof(DateTimeOffset))]
    [InlineData("binary", 0L, SFDataType.BINARY, typeof(byte[]))]
    [InlineData("boolean", 0L, SFDataType.BOOLEAN, typeof(bool))]
    public void EverySnowflakeTypeMapsToProviderTypeAndClrType(string type, long scale, SFDataType providerType, Type clrType)
    {
        var table = SchemaTableBuilder.Build(UnitBatches.Descriptor(new ColumnMetadata { Name = "C1", Type = type, Scale = scale }));

        var row = Assert.Single(table.Rows.Cast<DataRow>());
        Assert.Equal(providerType, (SFDataType)row[SchemaTableColumn.ProviderType]);
        Assert.Equal(clrType, row[SchemaTableColumn.DataType]);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("variant", 0L, 16777216)]
    [InlineData("object", 0L, 16777216)]
    [InlineData("array", 0L, 16777216)]
    [InlineData("map", 0L, 16777216)]
    [InlineData("VARIANT", 0L, 16777216)]
    [InlineData("variant", 64L, 64)]
    [InlineData("text", 0L, 0)]
    [InlineData("binary", 0L, 0)]
    [InlineData("vector", 0L, 0)]
    [InlineData("timestamp_ntz", 0L, 0)]
    [InlineData("array", 32L, 32)]
    [InlineData("object", 32L, 32)]
    [InlineData("map", 32L, 32)]
    public void SemiStructuredColumnsWithoutALengthGetTheMaximumStringLength(string type, long length, int expectedSize)
    {
        var table = SchemaTableBuilder.Build(UnitBatches.Descriptor(new ColumnMetadata { Name = "C1", Type = type, Length = length }));

        Assert.Equal(expectedSize, table.Rows[0][SchemaTableColumn.ColumnSize]);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("geography")]
    [InlineData("none")]
    [InlineData("")]
    [InlineData("3")]
    [InlineData("-1")]
    [InlineData("99")]
    [InlineData("fixed,real")]
    [InlineData("fixed ")]
    [InlineData(" fixed")]
    public void UnknownSnowflakeTypeIsInternalError(string type)
    {
        var descriptor = UnitBatches.Descriptor(UnitBatches.TextColumn("C1"), new ColumnMetadata { Name = "C2", Type = type });

        var exception = Assert.Throws<SnowflakeDriverException>(() => SchemaTableBuilder.Build(descriptor));

        Assert.Equal(SFError.INTERNAL_ERROR, exception.Error);
        Assert.Equal(descriptor.QueryId, exception.QueryId);
        Assert.Contains($"Unknown column type: {type}", Assert.Single(exception.Args).ToString());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void MissingQueryIdIsNotReportedAsEmpty()
    {
        var descriptor = new ResultSetDescriptor();
        descriptor.Columns.Add(new ColumnMetadata { Name = "C1", Type = "geography" });

        var exception = Assert.Throws<SnowflakeDriverException>(() => SchemaTableBuilder.Build(descriptor));

        Assert.Null(exception.QueryId);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ColumnsWithTheSameNameGetOneRowEach()
    {
        var descriptor = UnitBatches.Descriptor(UnitBatches.TextColumn("C1"), UnitBatches.FixedColumn("C1"));

        var table = SchemaTableBuilder.Build(descriptor);

        Assert.Equal(["C1", "C1"], table.Rows.Cast<DataRow>().Select(row => (string)row[SchemaTableColumn.ColumnName]));
        Assert.Equal([0, 1], table.Rows.Cast<DataRow>().Select(row => (int)row[SchemaTableColumn.ColumnOrdinal]));
    }
}
