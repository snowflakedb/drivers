using Snowflake.Data.Interfaces;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
public class PutGetOverwriteTest : IClassFixture<ITFixture>, IDisposable
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    private const string TestFileName = "test_data.csv";
    private const int PutStatusColumn = 6;

    private readonly string _workDir;

    public PutGetOverwriteTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
        _workDir = Path.Combine(Path.GetTempPath(), $"ud_dotnet_put_get_{Guid.NewGuid():N}");
        Directory.CreateDirectory(Path.Combine(_workDir, "original"));
        Directory.CreateDirectory(Path.Combine(_workDir, "updated"));
        File.WriteAllText(OriginalFilePath, "original,test,data\n");
        File.WriteAllText(UpdatedFilePath, "updated,test,data\n");
    }

    private string OriginalFilePath => Path.Combine(_workDir, "original", TestFileName);

    private string UpdatedFilePath => Path.Combine(_workDir, "updated", TestFileName);

    public void Dispose()
    {
        if (Directory.Exists(_workDir))
            Directory.Delete(_workDir, recursive: true);
    }

    // Scenario: should overwrite file when OVERWRITE is set to true
    [SnowflakeFact]
    public async Task ShouldOverwriteFileWhenOverwriteIsSetToTrueAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        // Given File is uploaded to stage
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        var stageName = await CreateTemporaryStageAsync(connection, "TEST_PUT_GET_OVERWRITE_TRUE", ct).ConfigureAwait(false);
        Assert.Equal("UPLOADED", await UploadFileAsync(connection, stageName, OriginalFilePath, overwrite: true, ct).ConfigureAwait(false));

        // When Updated file is uploaded with OVERWRITE set to true
        var status = await UploadFileAsync(connection, stageName, UpdatedFilePath, overwrite: true, ct).ConfigureAwait(false);

        // Then UPLOADED status is returned
        Assert.Equal("UPLOADED", status);

        // And File was overwritten
        var row = Assert.Single(await ExecuteRowsAsync(connection, $"SELECT $1, $2, $3 FROM @{stageName}", ct).ConfigureAwait(false));
        Assert.Equal(new object[] { "updated", "test", "data" }, row);
    }

    // Scenario: should not overwrite file when OVERWRITE is set to false
    [SnowflakeFact]
    public async Task ShouldNotOverwriteFileWhenOverwriteIsSetToFalseAsync()
    {
        var ct = TestContext.Current.CancellationToken;
        // Given File is uploaded to stage
        using var connection = Fixture.Factory.Create(Output);
        await connection.OpenAsync(ct).ConfigureAwait(false);
        var stageName = await CreateTemporaryStageAsync(connection, "TEST_PUT_GET_OVERWRITE_FALSE", ct).ConfigureAwait(false);
        Assert.Equal("UPLOADED", await UploadFileAsync(connection, stageName, OriginalFilePath, overwrite: true, ct).ConfigureAwait(false));

        // When Updated file is uploaded with OVERWRITE set to false
        var status = await UploadFileAsync(connection, stageName, UpdatedFilePath, overwrite: false, ct).ConfigureAwait(false);

        // Then SKIPPED status is returned
        Assert.Equal("SKIPPED", status);

        // And File was not overwritten
        var row = Assert.Single(await ExecuteRowsAsync(connection, $"SELECT $1, $2, $3 FROM @{stageName}", ct).ConfigureAwait(false));
        Assert.Equal(new object[] { "original", "test", "data" }, row);
    }

    private static async Task<string> CreateTemporaryStageAsync(ISnowflakeDbConnection connection, string prefix, CancellationToken cancellationToken)
    {
        var stageName = $"{prefix}_{Guid.NewGuid():N}".ToUpperInvariant();
        using var command = connection.CreateCommand();
        command.CommandText = $"CREATE TEMPORARY STAGE {stageName}";
        await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
        return stageName;
    }

    private static async Task<string> UploadFileAsync(ISnowflakeDbConnection connection, string stageName, string filePath, bool overwrite, CancellationToken cancellationToken)
    {
        var sql = $"PUT '{FileUri(filePath)}' @{stageName} AUTO_COMPRESS=FALSE OVERWRITE={(overwrite ? "TRUE" : "FALSE")}";
        var row = Assert.Single(await ExecuteRowsAsync(connection, sql, cancellationToken).ConfigureAwait(false));
        return (string)row[PutStatusColumn];
    }

    private static async Task<List<object[]>> ExecuteRowsAsync(ISnowflakeDbConnection connection, string sql, CancellationToken cancellationToken)
    {
        using var command = connection.CreateCommand();
        command.CommandText = sql;
        using var reader = command.ExecuteReader();
        var rows = new List<object[]>();
        while (await reader.ReadAsync(cancellationToken).ConfigureAwait(false))
        {
            var row = new object[reader.FieldCount];
            reader.GetValues(row);
            rows.Add(row);
        }
        return rows;
    }

    private static string FileUri(string path) => $"file://{path.Replace('\\', '/')}";
}
