using System.Data.Common;

namespace Snowflake.Data.Tests;

// TODO these tests are just PoC and will undergo heavy refactoring.
[Trait("Category", "E2E")]
public sealed class DataReaderTests : IClassFixture<ITFixture>
{
    private readonly ITestOutputHelper _testOutputHelper;

    public DataReaderTests(ITestOutputHelper testOutputHelper)
    {
        _testOutputHelper = testOutputHelper;
    }

    [SnowflakeFact]
    public void ExecuteReader_ArrowFormat_ReadsValueCorrectly()
    {
        using var connection = TestConnectionFactory.Create(_testOutputHelper);
        connection.Open();

        ExecuteNonQuery(connection, "alter session set DOTNET_QUERY_RESULT_FORMAT = ARROW");

        var tableName = $"TEST_READER_ARROW_{Guid.NewGuid():N}";
        try
        {
            ExecuteNonQuery(connection, $"CREATE TABLE {tableName} (ColA NUMBER)");
            ExecuteNonQuery(connection, $"INSERT INTO {tableName} VALUES (1)");

            using var selectCmd = connection.CreateCommand();
            selectCmd.CommandText = $"SELECT ColA FROM {tableName}";

            using var reader = selectCmd.ExecuteReader();

            Assert.False(reader.IsClosed);
            Assert.Equal(1, reader.FieldCount);
            Assert.True(reader.HasRows);
            Assert.Equal("COLA", reader.GetName(0));

            Assert.True(reader.Read());
            Assert.False(reader.IsDBNull(0));
            Assert.Equal(1L, reader.GetInt64(0));
            Assert.Equal(1L, reader.GetValue(0));

            Assert.False(reader.Read());
        }
        finally
        {
            ExecuteNonQuery(connection, $"DROP TABLE IF EXISTS {tableName}");
        }
    }

    [SnowflakeFact]
    public void ExecuteReader_JsonFormat_ReadsValueCorrectly()
    {
        using var connection = TestConnectionFactory.Create(_testOutputHelper);
        connection.Open();

        ExecuteNonQuery(connection, "alter session set DOTNET_QUERY_RESULT_FORMAT = JSON");

        var tableName = $"TEST_READER_JSON_{Guid.NewGuid():N}";
        try
        {
            ExecuteNonQuery(connection, $"CREATE TABLE {tableName} (ColA NUMBER)");
            ExecuteNonQuery(connection, $"INSERT INTO {tableName} VALUES (1)");

            using var selectCmd = connection.CreateCommand();
            selectCmd.CommandText = $"SELECT ColA FROM {tableName}";

            using var reader = selectCmd.ExecuteReader();

            Assert.False(reader.IsClosed);
            Assert.Equal(1, reader.FieldCount);
            Assert.True(reader.HasRows);
            Assert.Equal("COLA", reader.GetName(0));

            Assert.True(reader.Read());
            Assert.False(reader.IsDBNull(0));
            Assert.Equal(1L, reader.GetInt64(0));
            Assert.Equal(1L, reader.GetValue(0));

            Assert.False(reader.Read());
        }
        finally
        {
            ExecuteNonQuery(connection, $"DROP TABLE IF EXISTS {tableName}");
        }
    }

    private static void ExecuteNonQuery(DbConnection connection, string sql)
    {
        using var cmd = connection.CreateCommand();
        cmd.CommandText = sql;
        cmd.ExecuteNonQuery();
    }
}
