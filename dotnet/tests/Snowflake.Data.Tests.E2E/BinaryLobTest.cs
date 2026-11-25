using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
[Trait("Category", "Conversion")]
public class BinaryLobTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    private const int PatternBlockSize = 64 * 1024;

    private static readonly byte[] PatternBlock = CreatePatternBlock();

    public BinaryLobTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should handle maximum default binary size
    [SnowflakeFact]
    public async Task ShouldHandleMaximumDefaultBinarySizeAsync()
    {
        Skip.FutureMilestone("Commands with bound parameters are not implemented (SNOW-4382583)");
        var ct = TestContext.Current.CancellationToken;

        const int lobSize = 8 * 1024 * 1024; // 8MB = 8,388,608 bytes

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And Table with BINARY column exists
        var tableName = $"UD_BIN_LOB_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BINARY)";
        await createCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When Binary value of 8MB size (8,388,608 bytes) is inserted
        var data = CreatePatternedBytes(lobSize);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
        var param = insertCmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.Binary;
        param.Value = data;
        insertCmd.Parameters.Add(param);
        await insertCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // And Query "SELECT * FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = await selectCmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        var result = (byte[])reader.GetValue(0);

        // Then the retrieved value size should be 8MB (8,388,608 bytes)
        Assert.Equal(lobSize, result.Length);

        // And data integrity should be maintained
        AssertMatchesPattern(result);
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    // Scenario: should handle extended maximum binary size
    [SnowflakeFact]
    public async Task ShouldHandleExtendedMaximumBinarySizeAsync()
    {
        Skip.FutureMilestone("Commands with bound parameters are not implemented (SNOW-4382583)");
        var ct = TestContext.Current.CancellationToken;

        const int lobSize = 64 * 1024 * 1024; // 64MB = 67,108,864 bytes

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);

        // And Table with BINARY(67108864) column exists
        var tableName = $"UD_BIN_EXT_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BINARY({lobSize}))";
        await createCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // When Binary value of 64MB size (67,108,864 bytes) is inserted
        var data = CreatePatternedBytes(lobSize);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
        var param = insertCmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.Binary;
        param.Value = data;
        insertCmd.Parameters.Add(param);
        await insertCmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

        // And Query "SELECT * FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = await selectCmd.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected one row");
        var result = (byte[])reader.GetValue(0);

        // Then the retrieved value size should be 64MB (67,108,864 bytes)
        Assert.Equal(lobSize, result.Length);

        // And data integrity should be maintained
        AssertMatchesPattern(result);
        Assert.False(await reader.ReadAsync(ct).ConfigureAwait(false), "Expected exactly one row");
    }

    private static byte[] CreatePatternBlock()
    {
        var block = new byte[PatternBlockSize];
        new Random(42).NextBytes(block);
        return block;
    }

    private static byte[] CreatePatternedBytes(int length)
    {
        var data = new byte[length];
        for (var offset = 0; offset < length; offset += PatternBlockSize)
        {
            PatternBlock.AsSpan(0, Math.Min(PatternBlockSize, length - offset)).CopyTo(data.AsSpan(offset));
        }
        return data;
    }

    private static void AssertMatchesPattern(byte[] actual)
    {
        for (var offset = 0; offset < actual.Length; offset += PatternBlockSize)
        {
            var count = Math.Min(PatternBlockSize, actual.Length - offset);
            Assert.True(
                actual.AsSpan(offset, count).SequenceEqual(PatternBlock.AsSpan(0, count)),
                $"Binary value differs from the expected pattern in the block starting at byte {offset}");
        }
    }
}
