using System.Data.Common;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

[CollectionDefinition(Name, DisableParallelization = true)]
public sealed class SFReusableChunkTestCollection
{
    public const string Name = "SFReusableChunkTest";
}

// ChunkParserFactory.Instance is process-wide in the old driver, so fault-injecting tests must not overlap other queries.
[Collection(SFReusableChunkTestCollection.Name)]
public sealed class SFReusableChunkTest : ReferenceTestBase
{
    private const string FaultInjectionNotAvailable =
        "Chunk parser fault injection uses the old driver's internal ChunkParserFactory; the new driver parses result chunks in sf_core";

    public SFReusableChunkTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldReadRowsWithDelCharacterAsync()
    {
        // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
        Skip.FutureMilestone();

        const int TestRowCount = 10000;
        var ct = TestContext.Current.CancellationToken;
        var tableName = $"REUSABLE_CHUNK_{Guid.NewGuid():N}";
        using var conn = await OpenJsonConnectionAsync(ct).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        try
        {
            await CreateOrReplaceTableAsync(cmd, tableName, "col STRING", ct).ConfigureAwait(false);

            cmd.CommandText = $"insert into {tableName}(select hex_decode_string(hex_encode('snow') || '7F' || hex_encode('FLAKE')) from table(generator(rowcount => {TestRowCount})))";
            using (var insertReader = cmd.ExecuteReader())
            {
                Assert.Equal(TestRowCount, insertReader.RecordsAffected);
            }

            cmd.CommandText = $"select * from {tableName}";
            Assert.Equal(TestRowCount, CountRowsWithoutEscapedDelCharacter(cmd));
        }
        finally
        {
            await DropTableAsync(cmd, tableName, ct).ConfigureAwait(false);
        }
    }

    [SnowflakeFact]
    public async Task ShouldParseJsonAsync()
    {
        Skip.OnNewDriver("BD#chunk-parser-not-pluggable", FaultInjectionNotAvailable);

        var ct = TestContext.Current.CancellationToken;
        var tableName = $"REUSABLE_CHUNK_{Guid.NewGuid():N}";
        using var injector = FaultInjectingChunkParserFactory.Install(1);
        using var conn = await OpenJsonConnectionAsync(ct).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        try
        {
            await CreateOrReplaceTableAsync(cmd, tableName, "src VARIANT", ct).ConfigureAwait(false);

            cmd.CommandText = $@"
-- borrowed from https://docs.snowflake.com/en/user-guide/querying-semistructured.html#sample-data-used-in-examples
insert into {tableName} (
select parse_json('{{
    ""date"" : ""2017 - 04 - 28"",
    ""dealership"" : ""Valley View Auto Sales"",
    ""salesperson"" : {{
                    ""id"": ""55"",
      ""name"": ""Frank Beasley""
    }},
    ""customer"" : [
      {{ ""name"": ""Joyce Ridgely"", ""phone"": ""16504378889"", ""address"": ""San Francisco, CA""}}
    ],
    ""vehicle"" : [
       {{ ""make"": ""Honda"", ""model"": ""Civic"", ""year"": ""2017"", ""price"": ""20275"", ""extras"":[""ext warranty"", ""paint protection""]}}
    ]
}}') from table(generator(rowcount => 500))
)
";
            using (var insertReader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false))
            {
                Assert.Equal(500, insertReader.RecordsAffected);
            }

            cmd.CommandText = $"select * from {tableName}";
            cmd.CommandType = CommandType.Text;

            var rowCount = 0;
            using (var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false))
            {
                while (await reader.ReadAsync(ct).ConfigureAwait(false))
                {
                    Newtonsoft.Json.JsonConvert.DeserializeObject(reader[0].ToString()!);
                    rowCount++;
                }
            }

            Assert.Equal(500, rowCount);
        }
        finally
        {
            await DropTableAsync(cmd, tableName, ct).ConfigureAwait(false);
        }
    }

    [SnowflakeFact]
    public async Task ShouldRetryChunkParsingWhenParserFailsAsync()
    {
        Skip.OnNewDriver("BD#chunk-parser-not-pluggable", FaultInjectionNotAvailable);

        const int RetryFailureCount = 6;
        const int TestRowCount = 10000;
        var ct = TestContext.Current.CancellationToken;
        var tableName = $"REUSABLE_CHUNK_{Guid.NewGuid():N}";
        using var conn = await OpenJsonConnectionAsync(ct).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        try
        {
            using var injector = FaultInjectingChunkParserFactory.Install(RetryFailureCount);
            await CreateOrReplaceTableAsync(cmd, tableName, "col STRING", ct).ConfigureAwait(false);

            cmd.CommandText = $"insert into {tableName}(select hex_decode_string(hex_encode('snow') || '7F' || hex_encode('FLAKE')) from table(generator(rowcount => {TestRowCount})))";
            using (var insertReader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false))
            {
                Assert.Equal(TestRowCount, insertReader.RecordsAffected);
            }

            cmd.CommandText = $"select * from {tableName}";
            var rowCount = 0;
            using (var reader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false))
            {
                while (await reader.ReadAsync(ct).ConfigureAwait(false))
                {
                    if (!HasEscapedDelCharacter(reader))
                        rowCount++;
                }
            }

            Assert.Equal(TestRowCount, rowCount);
            Assert.True(injector.ParsersRequested >= RetryFailureCount);
        }
        finally
        {
            await DropTableAsync(cmd, tableName, ct).ConfigureAwait(false);
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.Thrice)]
    public async Task ShouldThrowExceptionWhenChunkDownloadRetryCountExceededAsync()
    {
        Skip.OnNewDriver("BD#chunk-parser-not-pluggable", FaultInjectionNotAvailable);

        const int ExcessiveRetryCount = 8;
        const int TestRowCount = 25000;
        var ct = TestContext.Current.CancellationToken;
        var tableName = $"REUSABLE_CHUNK_{Guid.NewGuid():N}";
        using var injector = FaultInjectingChunkParserFactory.Install(ExcessiveRetryCount);
        using var conn = await OpenJsonConnectionAsync(ct).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        try
        {
            await CreateOrReplaceTableAsync(cmd, tableName, "col STRING", ct).ConfigureAwait(false);

            cmd.CommandText = $"insert into {tableName}(select hex_decode_string(hex_encode('snow') || '7F' || hex_encode('FLAKE')) from table(generator(rowcount => {TestRowCount})))";
            using (var insertReader = await cmd.ExecuteReaderAsync(ct).ConfigureAwait(false))
            {
                Assert.Equal(TestRowCount, insertReader.RecordsAffected);
            }

            cmd.CommandText = $"select * from {tableName}";
            var rowCount = 0;
            Assert.Throws<AggregateException>(() =>
            {
                using var reader = cmd.ExecuteReader();
                while (reader.Read())
                {
                    if (!HasEscapedDelCharacter(reader))
                        rowCount++;
                }
            });
            Assert.NotEqual(TestRowCount, rowCount);
        }
        finally
        {
            await DropTableAsync(cmd, tableName, ct).ConfigureAwait(false);
        }
    }

    private async Task<ISnowflakeDbConnection> OpenJsonConnectionAsync(CancellationToken ct)
    {
        var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(ct).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = JSON";
        await cmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        return conn;
    }

    private static async Task CreateOrReplaceTableAsync(
        DbCommand cmd,
        string tableName,
        string columns,
        CancellationToken ct)
    {
        cmd.CommandText = $"CREATE OR REPLACE TABLE {tableName}({columns})";
        await cmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
    }

    private static async Task DropTableAsync(DbCommand cmd, string tableName, CancellationToken ct)
    {
        cmd.CommandText = $"DROP TABLE IF EXISTS {tableName}";
        await cmd.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
    }

    private static int CountRowsWithoutEscapedDelCharacter(DbCommand cmd)
    {
        var rowCount = 0;
        using var reader = cmd.ExecuteReader();
        while (reader.Read())
        {
            if (!HasEscapedDelCharacter(reader))
                rowCount++;
        }

        return rowCount;
    }

    // The stored value holds a raw DEL character; a literal "u007f" in the read value means it was mis-decoded.
    private static bool HasEscapedDelCharacter(DbDataReader reader)
    {
        var values = new object[reader.FieldCount];
        reader.GetValues(values);
        var value = (values[0] ?? string.Empty).ToString()!;
        return value.Contains("u007f") || value.Contains("\u007fu");
    }
}
