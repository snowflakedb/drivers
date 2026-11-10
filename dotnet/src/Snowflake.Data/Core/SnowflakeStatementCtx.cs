namespace Snowflake.Data.Core;

/// <summary>
/// What one execution of a <see cref="SnowflakeDbCommand"/> needs: the SQL text, the per-statement
/// query tag (null keeps the connection's tag), the client-side timeout (null means none) and
/// whether only the result schema is wanted.
/// </summary>
internal readonly record struct SnowflakeStatementCtx(string CommandText, string? QueryTag, TimeSpan? CommandTimeout, bool DescribeOnly = false);
