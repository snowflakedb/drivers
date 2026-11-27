namespace Snowflake.Data.Client;

#if NETFRAMEWORK
public sealed partial class SnowflakeDbException
{
    /// <summary>The ANSI SQL state of the failure, or <see langword="null"/> when it has none.</summary>
    public string? SqlState => _sqlState;
}
#else
public sealed partial class SnowflakeDbException
{
    /// <summary>The ANSI SQL state of the failure, or <see langword="null"/> when it has none.</summary>
    /// <remarks>Overrides <see cref="System.Data.Common.DbException.SqlState"/>, so code that reads it through the base type sees the same value.</remarks>
    public override string? SqlState => _sqlState;
}
#endif
