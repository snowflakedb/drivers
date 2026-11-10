using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Core;

/// <summary>
/// A caller-supplied query ID that has the UUID format <c>xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx</c>.
/// </summary>
/// <remarks>
/// <see cref="Value"/> is the caller's string unchanged, so its letter case reaches sf_core as given.
/// Only <see cref="Parse"/> produces a valid instance; <c>default</c> has a null <see cref="Value"/>.
/// </remarks>
internal readonly record struct QueryId(string Value)
{
    public override string ToString() => Value;

    /// <summary>Accepts <paramref name="value"/> when it is a UUID in the hyphenated 8-4-4-4-12 form.</summary>
    /// <exception cref="SnowflakeDriverException">
    /// <see cref="SFError.INVALID_QUERY_ID_FORMAT"/> when <paramref name="value"/> is null or not in that form.
    /// </exception>
    internal static QueryId Parse(string? value) =>
        TryParse(value, out var queryId)
            ? queryId
            : throw new SnowflakeDriverException(SFError.INVALID_QUERY_ID_FORMAT, null);

    /// <summary>Accepts <paramref name="value"/> when it is a UUID in the hyphenated 8-4-4-4-12 form.</summary>
    /// <returns>False, with <paramref name="queryId"/> set to <c>default</c>, when <paramref name="value"/> is null or not in that form.</returns>
    internal static bool TryParse(string? value, out QueryId queryId)
    {
        if (value is not null && Guid.TryParseExact(value, "D", out _))
        {
            queryId = new QueryId(value);
            return true;
        }

        queryId = default;
        return false;
    }
}
