using System.Data.Common;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Reference;

public sealed class SFMaxLobSizeSwitchTest : ReferenceTestBase
{
    private const string SqlSelectLargeString = "select randstr(20000000, random()) as large_str";

    public SFMaxLobSizeSwitchTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldSelectLargeStringOnlyWhenLargeVarcharAndBinaryInResultIsEnabledAsync()
    {
        Skip.When(true, "TODO: Enable when Max LOB size is available on the automated tests environment");

        var baseConnectionString = Fixture.Factory.Create(Output).ConnectionString;
        using var conn = Fixture.Factory.Create(Output, AppendParam(baseConnectionString, "poolingEnabled=false"));
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "alter session set ENABLE_LARGE_VARCHAR_AND_BINARY_IN_RESULT=false";
        cmd.ExecuteNonQuery();

        cmd.CommandText = SqlSelectLargeString;
        var thrown = ExecuteReaderThrowsSnowflakeDbException(cmd);
        Assert.Contains("exceeds supported length", thrown.Message);

        cmd.CommandText = "alter session set ENABLE_LARGE_VARCHAR_AND_BINARY_IN_RESULT=true";
        cmd.ExecuteNonQuery();
        cmd.CommandText = SqlSelectLargeString;
        using var reader = cmd.ExecuteReader();
        Assert.True(reader.Read());
    }

    private static string AppendParam(string connectionString, string extra)
    {
        if (connectionString.EndsWith(";", StringComparison.Ordinal))
            return connectionString + extra;
        return connectionString + ";" + extra;
    }

    private static Exception ExecuteReaderThrowsSnowflakeDbException(DbCommand cmd)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            return Assert.Throws<OldSnowflakeDbException>(() => cmd.ExecuteReader());
        return Assert.Throws<NewSnowflakeDbException>(() => cmd.ExecuteReader());
    }
}
