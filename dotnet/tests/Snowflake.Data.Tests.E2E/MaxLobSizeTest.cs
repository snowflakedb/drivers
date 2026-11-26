using System.Data.Common;
using System.Globalization;
using Snowflake.Data.Core;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "Conversion")]
public sealed class MaxLobSizeTest : LiveE2ETestBase
{
    private const int MaxLobSize = 16 * 1024 * 1024;
    private const int MediumSize = MaxLobSize / 4;
    private const int OriginSize = MediumSize / 2;
    private const int LobRandomRange = 100000 + 1;
    private static readonly string[] s_colName = ["C1", "C2", "C3"];
    private static readonly Random s_random = new();

    public MaxLobSizeTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeTheory]
    [InlineData("ARROW", MaxLobSize)]
    [InlineData("ARROW", MediumSize)]
    [InlineData("ARROW", OriginSize)]
    [InlineData("JSON", MaxLobSize)]
    [InlineData("JSON", MediumSize)]
    [InlineData("JSON", OriginSize)]
    public async Task ShouldSelectStringOfSpecifiedLobSizeAsync(string resultFormat, int size)
    {
        Skip.FutureMilestone("TODO SNOW-4483010 next milestone");
        Skip.When(
            !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("JENKINS_HOME")),
            "Test is skipped on Jenkins.");
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        AlterSessionSettings(conn, resultFormat);
        using var command = conn.CreateCommand();
        command.CommandText = $"SELECT RANDSTR({size.ToString(CultureInfo.InvariantCulture)}, 124)";
        var row = (string)(await command.ExecuteScalarAsync(TestContext.Current.CancellationToken).ConfigureAwait(false))!;
        Assert.Equal(size, row.Length);
    }

    [SnowflakeTheory]
    [InlineData("ARROW", MaxLobSize, SkipCondition.SkipOn32Bit)]
    [InlineData("ARROW", MediumSize, SkipCondition.None)]
    [InlineData("ARROW", OriginSize, SkipCondition.None)]
    [InlineData("JSON", MaxLobSize, SkipCondition.SkipOn32Bit)]
    [InlineData("JSON", MediumSize, SkipCondition.None)]
    [InlineData("JSON", OriginSize, SkipCondition.None)]
    public async Task ShouldInsertLobValuesUsingLiteralsAsync(string resultFormat, int lobSize, SkipCondition skipCondition)
    {
        Skip.When(
            !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("JENKINS_HOME")),
            "Test is skipped on Jenkins.");
        Skip.For(
            skipCondition,
            "Two max-size literals make a 32 MiB SQL text that a 32-bit process cannot reliably allocate under parallel E2E load.");
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var tableName = UniqueTableName();
        var c1 = GenerateRandomString(lobSize);
        var c2 = GenerateRandomString(lobSize);
        var c3 = s_random.Next(LobRandomRange);

        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateLobTable(conn, tableName);
            AlterSessionSettings(conn, resultFormat);

            using var command = conn.CreateCommand();
            command.CommandText =
                $"INSERT INTO {tableName}({string.Join(", ", s_colName)}) VALUES ('{c1}', '{c2}', '{c3.ToString(CultureInfo.InvariantCulture)}')";
            await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            command.CommandText = $"SELECT * FROM {tableName}";
            using var reader = await command.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.Equal(c1, reader.GetString(0));
            Assert.Equal(c2, reader.GetString(1));
            Assert.Equal(c3, reader.GetInt64(2));
            CheckColumnMetadata(reader);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("ARROW", MaxLobSize)]
    [InlineData("ARROW", MediumSize)]
    [InlineData("ARROW", OriginSize)]
    [InlineData("JSON", MaxLobSize)]
    [InlineData("JSON", MediumSize)]
    [InlineData("JSON", OriginSize)]
    public async Task ShouldInsertLobValuesUsingPositionalBindingAsync(string resultFormat, int lobSize)
    {
        Skip.When(
            !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("JENKINS_HOME")),
            "Test is skipped on Jenkins.");
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        var c1 = GenerateRandomString(lobSize);
        var c2 = GenerateRandomString(lobSize);
        var c3 = s_random.Next(LobRandomRange);

        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateLobTable(conn, tableName);
            AlterSessionSettings(conn, resultFormat);

            using var command = conn.CreateCommand();
            command.CommandText = $"INSERT INTO {tableName}({string.Join(", ", s_colName)}) VALUES (?, ?, ?)";

            AddParameter(command, "1", DbType.String, c1);
            AddParameter(command, "2", DbType.String, c2);
            AddParameter(command, "3", DbType.UInt32, c3);

            await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            command.CommandText = $"SELECT * FROM {tableName}";
            using var reader = await command.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.Equal(c1, reader.GetString(0));
            Assert.Equal(c2, reader.GetString(1));
            Assert.Equal(c3, reader.GetInt64(2));
            CheckColumnMetadata(reader);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("ARROW", MaxLobSize)]
    [InlineData("ARROW", MediumSize)]
    [InlineData("ARROW", OriginSize)]
    [InlineData("JSON", MaxLobSize)]
    [InlineData("JSON", MediumSize)]
    [InlineData("JSON", OriginSize)]
    public async Task ShouldInsertLobValuesUsingNamedBindingAsync(string resultFormat, int lobSize)
    {
        Skip.When(
            !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("JENKINS_HOME")),
            "Test is skipped on Jenkins.");
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        var c1 = GenerateRandomString(lobSize);
        var c2 = GenerateRandomString(lobSize);
        var c3 = s_random.Next(LobRandomRange);

        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateLobTable(conn, tableName);
            AlterSessionSettings(conn, resultFormat);

            using var command = conn.CreateCommand();
            command.CommandText = $"INSERT INTO {tableName}({string.Join(", ", s_colName)}) VALUES (:1, :2, :3)";

            AddParameter(command, "1", DbType.String, c1);
            AddParameter(command, "2", DbType.String, c2);
            AddParameter(command, "3", DbType.UInt32, c3);

            await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            command.CommandText = $"SELECT * FROM {tableName}";
            using var reader = await command.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.Equal(c1, reader.GetString(0));
            Assert.Equal(c2, reader.GetString(1));
            Assert.Equal(c3, reader.GetInt64(2));
            CheckColumnMetadata(reader);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("ARROW", MaxLobSize)]
    [InlineData("ARROW", MediumSize)]
    [InlineData("ARROW", OriginSize)]
    [InlineData("JSON", MaxLobSize)]
    [InlineData("JSON", MediumSize)]
    [InlineData("JSON", OriginSize)]
    public async Task ShouldPutCopyAndGetLobValuesAsync(string resultFormat, int lobSize)
    {
        Skip.When(
            !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("JENKINS_HOME")),
            "Test is skipped on Jenkins.");
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): GetString on a FIXED column
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        var c1 = GenerateRandomString(lobSize);
        var c2 = GenerateRandomString(lobSize);
        var c3 = s_random.Next(LobRandomRange);
        var colData = new[] { c1, c2, c3.ToString(CultureInfo.InvariantCulture) };

        var fileName = $"{Guid.NewGuid():N}.csv";
        var inputFilePath = Path.Combine(Path.GetTempPath(), fileName);
        var outputDirectory = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(outputDirectory);
        var outputFilePath = Path.Combine(outputDirectory, fileName);
        File.WriteAllText(inputFilePath, $"{colData[0]},{colData[1]},{colData[2]}");

        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateLobTable(conn, tableName);
            AlterSessionSettings(conn, resultFormat);

            using (var putCommand = conn.CreateCommand())
            {
                putCommand.CommandText = $"PUT file://{inputFilePath} @%{tableName} AUTO_COMPRESS=FALSE";
                using var putReader = putCommand.ExecuteReader();
                Assert.True(await putReader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
                Assert.Equal("UPLOADED", putReader.GetString(6));
            }

            using (var copyCommand = conn.CreateCommand())
            {
                copyCommand.CommandText = $"COPY INTO {tableName}";
                await copyCommand.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

                copyCommand.CommandText = $"SELECT * FROM {tableName}";
                using var reader = await copyCommand.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
                AssertOnColData(reader, colData, 0);
                AssertOnColData(reader, colData, 1);
                AssertOnColData(reader, colData, 2);
                CheckColumnMetadata(reader);
            }

            using (var getCommand = conn.CreateCommand())
            {
                getCommand.CommandText = $"GET @%{tableName}/{fileName} file://{outputDirectory}";
                using var getReader = getCommand.ExecuteReader();
                Assert.True(await getReader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
                Assert.Equal("DOWNLOADED", getReader.GetString(6));

                var content = File.ReadAllText(outputFilePath).Split(',');
                Assert.Equal(colData[0], content[0]);
                Assert.Equal(colData[1], content[1]);
                Assert.Equal(colData[2], content[2]);
            }
        }
        finally
        {
            DropTableIfExists(conn, tableName);
            if (File.Exists(inputFilePath))
                File.Delete(inputFilePath);
            if (File.Exists(outputFilePath))
                File.Delete(outputFilePath);
            if (Directory.Exists(outputDirectory))
                Directory.Delete(outputDirectory, true);
        }
    }

    private static string UniqueTableName() => $"LOB_TABLE_{Guid.NewGuid():N}";

    private static void CreateLobTable(ISnowflakeDbConnection conn, string tableName)
    {
        using var command = conn.CreateCommand();
        var columnNamesWithTypes =
            $"{s_colName[0]} VARCHAR({MaxLobSize.ToString(CultureInfo.InvariantCulture)}), {s_colName[1]} VARCHAR({MaxLobSize.ToString(CultureInfo.InvariantCulture)}), {s_colName[2]} INT";
        command.CommandText = $"CREATE OR REPLACE TABLE {tableName} ({columnNamesWithTypes})";
        command.ExecuteNonQuery();
    }

    private static void DropTableIfExists(ISnowflakeDbConnection conn, string tableName)
    {
        using var command = conn.CreateCommand();
        command.CommandText = $"DROP TABLE IF EXISTS {tableName}";
        command.ExecuteNonQuery();
    }

    private static void AlterSessionSettings(ISnowflakeDbConnection conn, string resultFormat)
    {
        using var command = conn.CreateCommand();
        command.CommandText = $"ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = {resultFormat}";
        command.ExecuteNonQuery();
    }

    private static void AddParameter(DbCommand command, string name, DbType dbType, object value)
    {
        var parameter = command.CreateParameter();
        parameter.ParameterName = name;
        parameter.DbType = dbType;
        parameter.Value = value;
        command.Parameters.Add(parameter);
    }

    private static void AssertOnColData(DbDataReader reader, string[] colData, int index)
    {
        var actual = reader.GetString(index);
        var substringLength = colData[index].Length > 20 ? 20 : colData[index].Length;
        Assert.True(
            colData[index] == actual,
            $"Expected strings to be equal. Expected: '{colData[index].Substring(0, substringLength)}' [...], got '{actual}' instead.");
    }

    private static void CheckColumnMetadata(DbDataReader reader)
    {
        var dataTable = reader.GetSchemaTable();
        Assert.NotNull(dataTable);

        var row = dataTable.Rows[0];
        Assert.Equal(s_colName[0], row[SchemaTableColumn.ColumnName]);
        Assert.Equal(0, row[SchemaTableColumn.ColumnOrdinal]);
        Assert.Equal(MaxLobSize, row[SchemaTableColumn.ColumnSize]);
        Assert.Equal(SFDataType.TEXT, (SFDataType)row[SchemaTableColumn.ProviderType]);

        row = dataTable.Rows[1];
        Assert.Equal(s_colName[1], row[SchemaTableColumn.ColumnName]);
        Assert.Equal(1, row[SchemaTableColumn.ColumnOrdinal]);
        Assert.Equal(MaxLobSize, row[SchemaTableColumn.ColumnSize]);
        Assert.Equal(SFDataType.TEXT, (SFDataType)row[SchemaTableColumn.ProviderType]);

        row = dataTable.Rows[2];
        Assert.Equal(s_colName[2], row[SchemaTableColumn.ColumnName]);
        Assert.Equal(2, row[SchemaTableColumn.ColumnOrdinal]);
        Assert.Equal(0, row[SchemaTableColumn.ColumnSize]);
        Assert.Equal(SFDataType.FIXED, (SFDataType)row[SchemaTableColumn.ProviderType]);
    }

    private static string GenerateRandomString(int sizeInBytes)
    {
        var bufferSize = sizeInBytes / 2;
        var bytes = new byte[bufferSize];
        s_random.NextBytes(bytes);
        return BitConverter.ToString(bytes).Replace("-", string.Empty);
    }
}
