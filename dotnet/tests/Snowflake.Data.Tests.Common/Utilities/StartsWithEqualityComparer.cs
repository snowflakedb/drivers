namespace Snowflake.Data.Tests.Utilities;

public sealed class StartsWithEqualityComparer : IEqualityComparer<string>
{
    public bool Equals(string? x, string? y)
    {
        if (x is null && y is null)
            return true;

        if (x is null || y is null)
            return false;

        return x.StartsWith(y) || y.StartsWith(x);
    }

    public int GetHashCode(string obj) => 0;
}
