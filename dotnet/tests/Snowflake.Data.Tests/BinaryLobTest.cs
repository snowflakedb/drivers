using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class BinaryLobTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public BinaryLobTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should handle maximum default binary size
    [SnowflakeFact]
    public void ShouldHandleMaximumDefaultBinarySize()
    {
        Skip.FutureMilestone();

        const int lobSize = 8 * 1024 * 1024; // 8MB = 8,388,608 bytes

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with BINARY column exists
        var tableName = $"UD_BIN_LOB_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BINARY)";
        createCmd.ExecuteNonQuery();

        // When Binary value of 8MB size (8,388,608 bytes) is inserted
        var data = new byte[lobSize];
        var rng = new Random(42);
        rng.NextBytes(data);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
        var param = insertCmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.Binary;
        param.Value = data;
        insertCmd.Parameters.Add(param);
        insertCmd.ExecuteNonQuery();

        // And Query "SELECT * FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = selectCmd.ExecuteReader();

        Assert.True(reader.Read(), "Expected one row");
        var result = (byte[])reader.GetValue(0);

        // Then the retrieved value size should be 8MB (8,388,608 bytes)
        Assert.Equal(lobSize, result.Length);

        // And data integrity should be maintained
        Assert.Equal(data, result);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle extended maximum binary size
    [SnowflakeFact]
    public void ShouldHandleExtendedMaximumBinarySize()
    {
        Skip.FutureMilestone();

        const int lobSize = 64 * 1024 * 1024; // 64MB = 67,108,864 bytes

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with BINARY(67108864) column exists
        var tableName = $"UD_BIN_EXT_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BINARY({lobSize}))";
        createCmd.ExecuteNonQuery();

        // When Binary value of 64MB size (67,108,864 bytes) is inserted
        var data = new byte[lobSize];
        var rng = new Random(42);
        rng.NextBytes(data);

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
        var param = insertCmd.CreateParameter();
        param.ParameterName = "1";
        param.DbType = DbType.Binary;
        param.Value = data;
        insertCmd.Parameters.Add(param);
        insertCmd.ExecuteNonQuery();

        // And Query "SELECT * FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = selectCmd.ExecuteReader();

        Assert.True(reader.Read(), "Expected one row");
        var result = (byte[])reader.GetValue(0);

        // Then the retrieved value size should be 64MB (67,108,864 bytes)
        Assert.Equal(lobSize, result.Length);

        // And data integrity should be maintained
        Assert.Equal(data, result);
        Assert.False(reader.Read(), "Expected exactly one row");
    }
}
