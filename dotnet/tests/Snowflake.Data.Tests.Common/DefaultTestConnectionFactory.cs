using System.Data.Common;
using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests;

public sealed class DefaultTestConnectionFactory : ITestConnectionFactory
{
    public DbConnection Create(ITestOutputHelper? testOutputHelper, string? connectionStringOverride = null) => new SnowflakeDbConnection(connectionStringOverride ?? BuildConnectionString(testOutputHelper));

    public string BuildConnectionString(ITestOutputHelper? testOutputHelper)
    {
        ParametersReader.Init(testOutputHelper);
        var account = ParametersReader.Get("SNOWFLAKE_TEST_ACCOUNT") ?? "";
        var user = ParametersReader.Get("SNOWFLAKE_TEST_USER") ?? "";
        var password = ParametersReader.Get("SNOWFLAKE_TEST_PASSWORD") ?? "";
        var warehouse = ParametersReader.Get("SNOWFLAKE_TEST_WAREHOUSE") ?? "";
        var database = ParametersReader.Get("SNOWFLAKE_TEST_DATABASE") ?? "";
        var schema = ParametersReader.Get("SNOWFLAKE_TEST_SCHEMA") ?? "";
        var role = ParametersReader.Get("SNOWFLAKE_TEST_ROLE") ?? "";
        var pat = ParametersReader.Get("SNOWFLAKE_TEST_PAT") ?? "";

        if (!string.IsNullOrEmpty(pat))
            return $"account={account};role={role};db={database};database={database};schema={schema};warehouse={warehouse};authenticator=PROGRAMMATIC_ACCESS_TOKEN;user={user};token={pat}";

        return $"account={account};user={user};password={password};warehouse={warehouse};db={database};database={database};schema={schema};role={role}";
    }
}
