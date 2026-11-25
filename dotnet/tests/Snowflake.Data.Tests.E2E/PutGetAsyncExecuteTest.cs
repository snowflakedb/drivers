using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests.E2E;

// TODO(SNOW-4382567): cover PUT and GET through ExecuteScalarAsync.
[Trait("Category", "E2E")]
public class PutGetAsyncExecuteTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    private const string OldDriverAsyncPutGetMessage =
        "Get and Put are not supported in async calls.  Use Execute() instead of ExecuteAsync().";

    public PutGetAsyncExecuteTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should upload a file with PUT through async non-query execution
    [SnowflakeFact]
    public async Task ShouldUploadAFileWithPutThroughAsyncNonQueryExecutionAsync()
    {
        Skip.FutureMilestone("TODO SNOW-4483010 next milestone");
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // And a temporary stage and a local file exist
        var stageName = CreateTemporaryStage(connection);
        using var localFile = new TemporaryFile("1,one\n2,two\n", ".csv");

        // When the file is uploaded with PUT through ExecuteNonQueryAsync
        using var put = connection.CreateCommand();
        put.CommandText = $"PUT '{FileUri(localFile.Path)}' @{stageName} AUTO_COMPRESS = FALSE";

        if (BehaviorDifference.OldDriverOnly("BD#async-execute-supports-put-get"))
        {
            var exception = await Assert.ThrowsAsync<NotImplementedException>(
                () => put.ExecuteNonQueryAsync()).ConfigureAwait(false);
            Assert.Equal(OldDriverAsyncPutGetMessage, exception.Message);
            return;
        }

        var nonQueryResult = await put.ExecuteNonQueryAsync().ConfigureAwait(false);

        // Then the non-query result is 0
        Assert.Equal(0, nonQueryResult);

        // And the stage lists the uploaded file
        using var list = connection.CreateCommand();
        list.CommandText = $"LIST @{stageName}";
        using var reader = await list.ExecuteReaderAsync().ConfigureAwait(false);
        Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected the uploaded file in the stage");
        Assert.EndsWith(localFile.Name, reader.GetString(0), StringComparison.OrdinalIgnoreCase);
        Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one file in the stage");
    }

    // Scenario: should download a file with GET through async reader execution
    [SnowflakeFact]
    public async Task ShouldDownloadAFileWithGetThroughAsyncReaderExecutionAsync()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync().ConfigureAwait(false);

        // And a temporary stage holding an uploaded file and an empty local directory exist
        const string contents = "1,one\n2,two\n";
        var stageName = CreateTemporaryStage(connection);
        using var localFile = new TemporaryFile(contents, ".csv");
        using (var put = connection.CreateCommand())
        {
            put.CommandText = $"PUT '{FileUri(localFile.Path)}' @{stageName} AUTO_COMPRESS = FALSE";
            put.ExecuteNonQuery();
        }
        using var downloadDirectory = new TemporaryDirectory();

        // When the file is downloaded with GET through ExecuteReaderAsync
        using var get = connection.CreateCommand();
        get.CommandText = $"GET @{stageName}/{localFile.Name} '{FileUri(downloadDirectory.Path)}/'";

        if (BehaviorDifference.OldDriverOnly("BD#async-execute-supports-put-get"))
        {
            var exception = await Assert.ThrowsAsync<NotImplementedException>(
                () => get.ExecuteReaderAsync()).ConfigureAwait(false);
            Assert.Equal(OldDriverAsyncPutGetMessage, exception.Message);
            return;
        }

        using (var reader = await get.ExecuteReaderAsync().ConfigureAwait(false))
        {
            Assert.True(await reader.ReadAsync().ConfigureAwait(false), "Expected a GET result row for the file");
            Assert.False(await reader.ReadAsync().ConfigureAwait(false), "Expected exactly one GET result row");
        }

        // Then the local directory holds the file with its original contents
        var downloaded = Path.Combine(downloadDirectory.Path, localFile.Name);
        Assert.True(File.Exists(downloaded), $"Expected {localFile.Name} in the download directory");
        Assert.Equal(contents, File.ReadAllText(downloaded));
    }

    private static string CreateTemporaryStage(ISnowflakeDbConnection connection)
    {
        var stageName = $"UD_PUT_GET_ASYNC_STAGE_{Guid.NewGuid():N}";
        using var command = connection.CreateCommand();
        command.CommandText = $"CREATE TEMPORARY STAGE {stageName}";
        command.ExecuteNonQuery();
        return stageName;
    }

    private static string FileUri(string path) => $"file://{path.Replace('\\', '/')}";
}
