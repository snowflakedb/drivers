using System.Globalization;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
public class StringLobTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    private const int Lob16MbSize = 16_777_216;
    private const int Lob128MbSize = 134_217_728;
    private const string Chars64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789()";

    public StringLobTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should handle LOB string at historical 16 MB limit
    [SnowflakeFact]
    public async Task ShouldHandleLobStringAtHistorical16MbLimitAsync()
    {
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And A temporary table with VARCHAR column is created
        var tableName = await CreateTemporaryTableAsync(connection, "val VARCHAR", ct).ConfigureAwait(false);

        // When A string of 16777216 ASCII characters is generated and inserted
        await InsertGeneratedStringAsync(connection, tableName, Lob16MbSize, ct).ConfigureAwait(false);

        // And Query "SELECT val, LENGTH(val) as len FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT val, LENGTH(val) as len FROM {tableName}";
        using var reader = await selectCmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");

        // Then the result should show length 16777216
        Assert.Equal(Lob16MbSize, Convert.ToInt64(reader.GetValue(1), CultureInfo.InvariantCulture));

        // And the returned string should exactly match the generated string
        AssertGeneratedLob(reader.GetString(0), Lob16MbSize);
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should handle LOB string at maximum 128 MB limit with increased LOB size
    [SnowflakeFact]
    public async Task ShouldHandleLobStringAtMaximum128MbLimitWithIncreasedLobSizeAsync()
    {
        Skip.FutureMilestone("SELECT of a 128 MB VARCHAR value fails, and the error surfaces as NotImplementedException from the SnowflakeDbException constructor (SNOW-4382591)");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And A temporary table with VARCHAR column is created
        var tableName = await CreateTemporaryTableAsync(connection, $"val VARCHAR({Lob128MbSize})", ct).ConfigureAwait(false);

        // When A string of 134217728 ASCII characters is generated and inserted
        await InsertGeneratedStringAsync(connection, tableName, Lob128MbSize, ct).ConfigureAwait(false);

        // And Query "SELECT val, LENGTH(val) as len FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT val, LENGTH(val) as len FROM {tableName}";
        using var reader = await selectCmd.ExecuteReaderAsync(ct).ConfigureAwait(false);
        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");

        // Then the result should show length 134217728
        Assert.Equal(Lob128MbSize, Convert.ToInt64(reader.GetValue(1), CultureInfo.InvariantCulture));

        // And the returned string should exactly match the generated string
        AssertGeneratedLob(reader.GetString(0), Lob128MbSize);
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    private static async Task<string> CreateTemporaryTableAsync(ISnowflakeDbConnection connection, string columnDefinition, CancellationToken cancellationToken)
    {
        var tableName = $"UD_STR_LOB_{Guid.NewGuid():N}";
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} ({columnDefinition})";
        await createCmd.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
        return tableName;
    }

    private static async Task InsertGeneratedStringAsync(ISnowflakeDbConnection connection, string tableName, int length, CancellationToken cancellationToken)
    {
        var repeatCount = length / Chars64.Length;
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} SELECT REPEAT('{Chars64}', {repeatCount})";
        await insertCmd.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
    }

    private static void AssertGeneratedLob(string actual, int expectedLength)
    {
        Assert.Equal(expectedLength, actual.Length);
        for (var offset = 0; offset < actual.Length; offset += Chars64.Length)
        {
            Assert.Equal(Chars64, actual.Substring(offset, Chars64.Length));
        }
    }
}
