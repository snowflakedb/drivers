namespace Snowflake.Data.Tests;

public interface ITestConnectionFactory
{
    TestDriver.Kind Driver { get; }

    ISnowflakeDbConnection Create(ITestOutputHelper? testOutputHelper, string? connectionStringOverride = null);
}
