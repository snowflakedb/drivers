using Microsoft.Extensions.Logging.Abstractions;
using NSubstitute;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Handles;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class ResultSetReaderFactoryTest
{
    private static readonly ResultSetHandle ResultHandle = new() { Id = 4, Magic = 404 };

    private readonly IDatabaseDriverService _driver = Substitute.For<IDatabaseDriverService>();

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CreateOpensAReaderOverTheStreamWithTheGivenCounts()
    {
        using var reader = ResultSetReaderFactory.Instance.Create(ResultSet(), UnitArrowStreams.Int64Column(42), Int64Descriptor(rowsAffected: 1), 1);

        var snowflakeReader = Assert.IsType<SnowflakeDbDataReader>(reader);
        Assert.Equal(1, snowflakeReader.FieldCount);
        Assert.Equal("C1", snowflakeReader.GetName(0));
        Assert.Equal(1, snowflakeReader.RecordsAffected);
        Assert.True(snowflakeReader.HasRows);
        Assert.True(snowflakeReader.Read());
        Assert.Equal(42L, snowflakeReader.GetInt64(0));
        Assert.False(snowflakeReader.Read());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ReaderFromCreateReleasesTheResultSetWhenClosed()
    {
        var reader = ResultSetReaderFactory.Instance.Create(ResultSet(), UnitArrowStreams.Int64Column(42), Int64Descriptor(), -1);
        _driver.DidNotReceiveWithAnyArgs().ResultSetRelease(default!);

        reader.Close();

        _driver.Received(1).ResultSetRelease(Arg.Is<ResultSetReleaseRequest>(request => request.ResultSetHandle.Equals(ResultHandle)));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void CreateForSchemaOnlyReportsTheColumnsAndNoRowsAndReleasesNothing()
    {
        var reader = ResultSetReaderFactory.Instance.CreateForSchemaOnly(UnitArrowStreams.Int64Column(), Int64Descriptor(), -1);

        var snowflakeReader = Assert.IsType<SnowflakeDbDataReader>(reader);
        Assert.Equal(1, snowflakeReader.FieldCount);
        Assert.Equal("C1", snowflakeReader.GetName(0));
        Assert.Equal(-1, snowflakeReader.RecordsAffected);
        Assert.False(snowflakeReader.HasRows);

        snowflakeReader.Close();

        _driver.DidNotReceiveWithAnyArgs().ResultSetRelease(default!);
    }

    private NativeResultSetHandle ResultSet() =>
        new(_driver, ResultHandle, NullLogger.Instance);

    private static ResultSetDescriptor Int64Descriptor(long? rowsAffected = null)
    {
        var descriptor = new ResultSetDescriptor();
        descriptor.Columns.Add(new ColumnMetadata { Name = "C1", Type = "FIXED" });
        if (rowsAffected is { } rows)
            descriptor.RowsAffected = rows;
        return descriptor;
    }
}
