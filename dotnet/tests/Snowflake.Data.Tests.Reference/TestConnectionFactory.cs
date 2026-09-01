using System.Data.Common;
using Snowflake.Data.Tests.Config;
using Snowflake.Data.Tests.Reference.Config;
using Snowflake.Data.Tests.Utilities;
using NewSnowflakeDbConnection = Snowflake.Data.SnowflakeDbConnection;
using OldSnowflakeDbConnection = Snowflake.Data.Client.SnowflakeDbConnection;
namespace Snowflake.Data.Tests.Reference;

public sealed class TestConnectionFactory : ITestConnectionFactory
{
    public DbConnection Create(ITestOutputHelper? testOutputHelper, string? connectionStringOverride = null)
    {
        var connStr = connectionStringOverride ?? BuildConnectionString(testOutputHelper);

        if (GlobalState.UseOldDriver)
            return new OldSnowflakeDbConnection { ConnectionString = connStr };

        return new NewSnowflakeDbConnection(connStr);
    }

    private static string BuildConnectionString(ITestOutputHelper? testOutputHelper)
    {
        ParametersReader.Init(testOutputHelper);

        IConnectionStringBuilder builder = new ConnectionStringBuilder();
        builder
            .WithAccount(ParametersReader.Get("SNOWFLAKE_TEST_ACCOUNT"))
            .WithUser(ParametersReader.Get("SNOWFLAKE_TEST_USER"))
            .WithPassword(ParametersReader.Get("SNOWFLAKE_TEST_PASSWORD"))
            .WithWarehouse(ParametersReader.Get("SNOWFLAKE_TEST_WAREHOUSE"))
            .WithDatabase(ParametersReader.Get("SNOWFLAKE_TEST_DATABASE"))
            .WithSchema(ParametersReader.Get("SNOWFLAKE_TEST_SCHEMA"))
            .WithRole(ParametersReader.Get("SNOWFLAKE_TEST_ROLE"))
            .WithPat(ParametersReader.Get("SNOWFLAKE_TEST_PAT"));

        return builder.Build();
    }
}
