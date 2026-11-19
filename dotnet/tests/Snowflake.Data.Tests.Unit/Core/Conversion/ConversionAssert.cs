using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Tests.Core.Conversion;

/// <summary>Assertions on the conversion carrier converters throw.</summary>
internal static class ConversionAssert
{
    /// <summary>
    /// Asserts <paramref name="read"/> throws <see cref="SFError.INVALID_DATA_CONVERSION"/> naming
    /// <paramref name="snowflakeType"/> and <paramref name="target"/>, and returns the carrier.
    /// </summary>
    internal static SnowflakeDriverException Fails(Func<object?> read, string snowflakeType, Type target)
    {
        var exception = Assert.Throws<SnowflakeDriverException>(() => read());

        Assert.Equal(SFError.INVALID_DATA_CONVERSION, exception.Error);
        Assert.Equal(snowflakeType, exception.Args[1]);
        Assert.Equal(target.Name, exception.Args[2]);
        return exception;
    }

    /// <summary>Asserts <paramref name="read"/> fails as <see cref="Fails"/> does, for a NULL value.</summary>
    internal static void FailsForNull(Func<object?> read, string snowflakeType, Type target) =>
        Assert.Equal("NULL", Fails(read, snowflakeType, target).Args[0]);
}
