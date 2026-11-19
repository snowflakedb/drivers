using Snowflake.Data.Core.Session;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class ResultSetInterpreterTest
{
    private readonly IResultSetInterpreter _interpreter = ResultSetInterpreter.Instance;

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(0L, 0)]
    [InlineData(1L, 1)]
    [InlineData(5L, 5)]
    [InlineData((long)int.MaxValue, int.MaxValue)]
    [InlineData(int.MaxValue + 1L, -1)]
    [InlineData(long.MaxValue, -1)]
    [InlineData(-1L, -1)]
    [InlineData(long.MinValue, -1)]
    public void ReportedCountIsUsedWhenItFitsInAnInt(long rowsAffected, int expected)
    {
        var descriptor = new ResultSetDescriptor { RowsAffected = rowsAffected };

        Assert.Equal(expected, _interpreter.CalculateUpdateCount(descriptor));
        Assert.Equal(expected, _interpreter.CalculateRecordsAffected(descriptor));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void MissingCountIsZeroForUpdatesAndMinusOneForRecordsAffected()
    {
        var descriptor = new ResultSetDescriptor();

        Assert.Equal(0, _interpreter.CalculateUpdateCount(descriptor));
        Assert.Equal(-1, _interpreter.CalculateRecordsAffected(descriptor));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void AnExactMatchWinsOverAnEarlierCaseInsensitiveOne()
    {
        var descriptor = UnitBatches.Descriptor(UnitBatches.TextColumn("name"), UnitBatches.TextColumn("NAME"), UnitBatches.TextColumn("Other"));

        Assert.True(_interpreter.TryGetOrdinal(descriptor, "name", out var lower));
        Assert.True(_interpreter.TryGetOrdinal(descriptor, "NAME", out var upper));
        Assert.Equal(0, lower);
        Assert.Equal(1, upper);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void WithoutAnExactMatchTheFirstCaseInsensitiveMatchWins()
    {
        var descriptor = UnitBatches.Descriptor(UnitBatches.TextColumn("name"), UnitBatches.TextColumn("NAME"), UnitBatches.TextColumn("Other"));

        Assert.True(_interpreter.TryGetOrdinal(descriptor, "Name", out var mixed));
        Assert.True(_interpreter.TryGetOrdinal(descriptor, "OTHER", out var other));
        Assert.Equal(0, mixed);
        Assert.Equal(2, other);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void AMissingNameIsNotFound()
    {
        var descriptor = UnitBatches.Descriptor(UnitBatches.TextColumn("ID"));

        Assert.False(_interpreter.TryGetOrdinal(descriptor, "missing", out var ordinal));
        Assert.Equal(-1, ordinal);
        Assert.False(_interpreter.TryGetOrdinal(UnitBatches.Descriptor(), "ID", out _));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void InstanceIsShared()
    {
        Assert.Same(ResultSetInterpreter.Instance, ResultSetInterpreter.Instance);
    }
}
