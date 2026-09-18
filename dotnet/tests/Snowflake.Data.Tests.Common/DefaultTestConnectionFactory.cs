using System.Data.Common;
using Snowflake.Data.Tests.Config;
using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests;

public sealed class DefaultTestConnectionFactory : ITestConnectionFactory
{
    public DbConnection Create(ITestOutputHelper? testOutputHelper, string? connectionStringOverride = null) => new SnowflakeDbConnection(connectionStringOverride ?? BuildConnectionString(testOutputHelper));

    private static string BuildConnectionString(ITestOutputHelper? testOutputHelper)
    {
        ParametersReader.Init(testOutputHelper);
        RsaKeyUtils.Init(testOutputHelper);
        var builder = new ConnectionStringBuilder();
        builder
            .WithAccount(ParametersReader.Get("SNOWFLAKE_TEST_ACCOUNT"))
            .WithUser(ParametersReader.Get("SNOWFLAKE_TEST_USER"))
            .WithWarehouse(ParametersReader.Get("SNOWFLAKE_TEST_WAREHOUSE"))
            .WithDatabase(ParametersReader.Get("SNOWFLAKE_TEST_DATABASE"))
            .WithSchema(ParametersReader.Get("SNOWFLAKE_TEST_SCHEMA"))
            .WithRole(ParametersReader.Get("SNOWFLAKE_TEST_ROLE"));

        var pat = ParametersReader.Get("SNOWFLAKE_TEST_PAT");
        if (!string.IsNullOrEmpty(pat))
        {
            var authBuilder = builder.WithPat(pat);
            return authBuilder.Build();
        }

        if (string.IsNullOrEmpty(pat) && RsaKeyUtils.TryDiscoverRsaKeyFile(out var path))
        {
            var authBuilder = builder.WithKeyFile(path);
            return authBuilder.Build();
        }

        throw new InvalidOperationException("Neither key nor PAT is available!");
    }
}
