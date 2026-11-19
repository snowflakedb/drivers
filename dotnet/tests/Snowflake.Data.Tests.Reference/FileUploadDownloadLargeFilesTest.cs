using System.Security.Cryptography;
using System.Text;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

public sealed class FileUploadDownloadLargeFilesTest : ReferenceTestBase
{
    private const string FileName = "large_file_to_test_dotnet_driver.json";
    private static readonly MD5 s_md5 = MD5.Create();
    private static readonly Random s_random = new();

    public FileUploadDownloadLargeFilesTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldDownloadSameLargeFileThatWasUploadedAsync()
    {
        var uniqueId = NextAlphaNumeric(6);
        var localFolderName = Path.Combine(Path.GetTempPath(), uniqueId);
        var remoteFolderName = $"files_to_test_put_get_{uniqueId}";
        var downloadFolderName = Path.Combine(localFolderName, "download");
        var fullFileName = Path.Combine(localFolderName, FileName);
        var fullDownloadedFileName = Path.Combine(downloadFolderName, FileName);

        Directory.CreateDirectory(localFolderName);
        Directory.CreateDirectory(downloadFolderName);
        GenerateLargeFile(fullFileName);
        var ct = TestContext.Current.CancellationToken;

        try
        {
            await UploadFileAsync(fullFileName, remoteFolderName, ct).ConfigureAwait(false);
            await DownloadFileAsync(remoteFolderName, downloadFolderName, FileName, ct).ConfigureAwait(false);

            Assert.Equal(CalculateMd5(fullFileName), CalculateMd5(fullDownloadedFileName));
        }
        finally
        {
            await RemoveFilesFromServerAsync(remoteFolderName, ct).ConfigureAwait(false);
            if (File.Exists(fullDownloadedFileName))
                File.Delete(fullDownloadedFileName);
            if (File.Exists(fullFileName))
                File.Delete(fullFileName);
            if (Directory.Exists(localFolderName))
                Directory.Delete(localFolderName, true);
        }
    }

    private async Task UploadFileAsync(string fullFileName, string remoteFolderName, CancellationToken ct)
    {
        var baseConnectionString = Fixture.Factory.Create(Output).ConnectionString;
        using var conn = Fixture.Factory.Create(
            Output,
            AppendParam(baseConnectionString, "FILE_TRANSFER_MEMORY_THRESHOLD=1048576;"));
        await conn.OpenAsync(ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = $"PUT file://{fullFileName} @~/{remoteFolderName} AUTO_COMPRESS=FALSE";
        command.ExecuteNonQuery();
    }

    private async Task DownloadFileAsync(string remoteFolderName, string downloadFolderName, string fileName, CancellationToken ct)
    {
        var filePattern = $"{remoteFolderName}/{fileName}";
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = $"GET @~/{remoteFolderName} file://{downloadFolderName} PATTERN='{filePattern}'";
        command.ExecuteNonQuery();
    }

    private async Task RemoveFilesFromServerAsync(string remoteFolderName, CancellationToken ct)
    {
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = $"remove @~/{remoteFolderName};";
        await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
    }

    private static string AppendParam(string connectionString, string extra)
    {
        if (connectionString.EndsWith(";", StringComparison.Ordinal))
            return connectionString + extra;
        return connectionString + ";" + extra;
    }

    private static void GenerateLargeFile(string fullFileName)
    {
        if (File.Exists(fullFileName))
            File.Delete(fullFileName);

        using var fileStream = File.Create(fullFileName);
        WriteAscii(fileStream, "{\n");
        var lineCount = 128 * 1024;
        for (var i = 0; i < lineCount; i++)
        {
            var line = $"  \"{NextAlphaNumeric(10)}\" : \"{NextAlphaNumeric(256)}\"";
            WriteAscii(fileStream, i == lineCount - 1 ? line + "\n" : line + ",\n");
        }

        WriteAscii(fileStream, "}\n");
    }

    private static void WriteAscii(Stream outputStream, string value)
    {
        var bytes = Encoding.ASCII.GetBytes(value);
        outputStream.Write(bytes, 0, bytes.Length);
    }

    private static string CalculateMd5(string fullFileName)
    {
        using var fileStream = File.OpenRead(fullFileName);
        var hash = s_md5.ComputeHash(fileStream);
        return BitConverter.ToString(hash);
    }

    private static string NextAlphaNumeric(int length)
    {
        const string Letters = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        const string Chars = Letters + "0123456789";
        var buffer = new char[length];
        buffer[0] = Letters[s_random.Next(Letters.Length)];
        for (var i = 1; i < length; i++)
            buffer[i] = Chars[s_random.Next(Chars.Length)];
        return new string(buffer);
    }
}
