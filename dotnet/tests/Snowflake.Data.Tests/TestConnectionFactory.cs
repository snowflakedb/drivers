using System.Data.Common;
using Snowflake.Data.Tests.Utilities;
using NewSnowflakeDbConnection = Snowflake.Data.SnowflakeDbConnection;
using OldSnowflakeDbConnection = Snowflake.Data.Client.SnowflakeDbConnection;
#pragma warning disable CS0162 // Unreachable code detected

namespace Snowflake.Data.Tests;

public static class TestConnectionFactory
{
    private static readonly bool UseOldDriver =
        Environment.GetEnvironmentVariable("SNOWFLAKE_DOTNET_USE_OLD_DRIVER") == "1";

    public static DbConnection Create(string? connectionStringOverride = null)
    {
        var connStr = connectionStringOverride ?? BuildConnectionString();
        if (UseOldDriver)
            return new OldSnowflakeDbConnection { ConnectionString = connStr };
        else
            return new NewSnowflakeDbConnection(connStr);
    }

    public static bool IsRunningForOldDriver() => UseOldDriver;

    private static string BuildConnectionString()
    {
        var account = ParametersReader.Get("SNOWFLAKE_TEST_ACCOUNT") ?? "";
        var user = ParametersReader.Get("SNOWFLAKE_TEST_USER") ?? "";
        var password = ParametersReader.Get("SNOWFLAKE_TEST_PASSWORD") ?? "";
        var warehouse = ParametersReader.Get("SNOWFLAKE_TEST_WAREHOUSE") ?? "";
        var database = ParametersReader.Get("SNOWFLAKE_TEST_DATABASE") ?? "";
        var schema = ParametersReader.Get("SNOWFLAKE_TEST_SCHEMA") ?? "";
        var role = ParametersReader.Get("SNOWFLAKE_TEST_ROLE") ?? "";
        var pat = ParametersReader.Get("SNOWFLAKE_TEST_PAT") ?? "";

        if (!string.IsNullOrEmpty(pat))
            return $"account={account};role={role};db={database};schema={schema};warehouse={warehouse};authenticator=PROGRAMMATIC_ACCESS_TOKEN;user={user};token={pat}";

        return $"account={account};user={user};password={password};warehouse={warehouse};db={database};schema={schema};role={role}";
    }
}
