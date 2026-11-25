namespace Snowflake.Data.Tests.Session;

/// <summary>
/// Tests that call the process-wide <see cref="Snowflake.Data.Client.SnowflakeDbConnectionPool"/> statics.
/// They run alone, and each test clears the default registry when it finishes.
/// </summary>
[CollectionDefinition(Name, DisableParallelization = true)]
public sealed class SnowflakeDbConnectionPoolStaticsCollection
{
    public const string Name = "SnowflakeDbConnectionPoolStatics";
}
