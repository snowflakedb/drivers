namespace Snowflake.Data.Tests;

public interface ITestConnectionFactory
{
    ISnowflakeDbConnection Create(ITestOutputHelper? testOutputHelper, string? connectionStringOverride = null);
}
