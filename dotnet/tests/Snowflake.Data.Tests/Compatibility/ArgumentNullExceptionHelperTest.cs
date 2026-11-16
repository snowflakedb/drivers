using Snowflake.Data.Compatibility;

namespace Snowflake.Data.Tests.Compatibility;

[Trait("Category", "Unit")]
public sealed class ArgumentNullExceptionHelperTest
{
    public static TheoryData<object> NonNullValues =>
    [
        string.Empty,
        0,
        Array.Empty<byte>(),
    ];

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullThrowsArgumentNullException()
    {
        var exception = Assert.Throws<ArgumentNullException>(() => ArgumentNullExceptionHelper.ThrowIfNull(null!, "request"));

        Assert.Equal("request", exception.ParamName);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void NullableValueTypeWithoutAValueThrowsArgumentNullException()
    {
        int? missing = null;

        var exception = Assert.Throws<ArgumentNullException>(() => ArgumentNullExceptionHelper.ThrowIfNull(missing!, nameof(missing)));

        Assert.Equal(nameof(missing), exception.ParamName);
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [MemberData(nameof(NonNullValues))]
    public void NonNullValueDoesNotThrow(object value)
    {
        var exception = Record.Exception(() => ArgumentNullExceptionHelper.ThrowIfNull(value, nameof(value)));

        Assert.Null(exception);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void DbNullIsNotNull() // not all nulls are created equal
    {
        var exception = Record.Exception(() => ArgumentNullExceptionHelper.ThrowIfNull(DBNull.Value, nameof(DBNull)));

        Assert.Null(exception);
    }
}
