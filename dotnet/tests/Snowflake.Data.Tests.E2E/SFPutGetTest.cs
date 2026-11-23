extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using System.IO.Compression;
using System.Text;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using NewSnowflakeDbCommand = Snowflake.Data.Client.SnowflakeDbCommand;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbCommand = OldDriver::Snowflake.Data.Client.SnowflakeDbCommand;
using OldSnowflakeDbDataReader = OldDriver::Snowflake.Data.Client.SnowflakeDbDataReader;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.E2E;

public sealed class SFPutGetTest : LiveE2ETestBase
{
    private const long NumberOfRows = 4;
    private const int SourceCompressionTypeOrdinal = 4;
    private const int DestinationCompressionTypeOrdinal = 5;
    private const int ResultStatusOrdinal = 6;
    private const int ErrorDetailsOrdinal = 7;
    private const int ClosedConnectionErrorCode = 270059;
    private const int IoErrorOnGetPutErrorCode = 270058;
    private const int SyntaxErrorCode = 1003;
    private const int MatchingDirectoryCount = 3;
    private const string NewDriverCompressionTypeCasing =
        "The new driver reports PUT compression types as uppercase labels";
    private const string NewDriverErrorDetails =
        "The new driver reports an empty error-details string for a successful PUT";
    private const string OldDriverCompressionTypeCasing =
        "The old driver reports PUT compression types as lowercase labels";
    private const string OldDriverErrorDetails =
        "The old driver reports null error details for a successful PUT";
    private static readonly string[] s_colName = ["C1", "C2", "C3"];
    private static readonly string[] s_colData = ["FIRST", "SECOND", "THIRD"];

    public SFPutGetTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    public enum StageType
    {
        USER,
        TABLE,
        NAMED
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldUploadFilesMatchingAsteriskWildcardAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await UploadFilesMatchingAsteriskWildcardAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldUploadFilesMatchingAsteriskWildcardReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await UploadFilesMatchingAsteriskWildcardAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadOnlyFilesMatchingAsteriskWildcardWithExtensionAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await UploadOnlyFilesMatchingAsteriskWildcardWithExtensionAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadOnlyFilesMatchingAsteriskWildcardWithExtensionReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await UploadOnlyFilesMatchingAsteriskWildcardWithExtensionAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldUploadFilesMatchingQuestionMarkWildcardAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await UploadFilesMatchingQuestionMarkWildcardAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldUploadFilesMatchingQuestionMarkWildcardReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await UploadFilesMatchingQuestionMarkWildcardAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFileFromRelativePathWithoutDirectoryAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await UploadFileFromRelativePathWithoutDirectoryAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFileFromRelativePathWithoutDirectoryReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await UploadFileFromRelativePathWithoutDirectoryAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldProvideQueryIdWhenPutSucceedsAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await ProvideQueryIdWhenPutSucceedsAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldProvideQueryIdWhenPutSucceedsReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await ProvideQueryIdWhenPutSucceedsAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFileFromRelativePathWithDirectoryAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await UploadFileFromRelativePathWithDirectoryAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFileFromRelativePathWithDirectoryReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await UploadFileFromRelativePathWithDirectoryAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFilesMatchingRelativePathAsteriskWildcardAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await UploadFilesMatchingRelativePathAsteriskWildcardAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFilesMatchingRelativePathAsteriskWildcardReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await UploadFilesMatchingRelativePathAsteriskWildcardAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldSkipSecondUploadWithoutOverwriteFlagAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        Skip.For(SkipCondition.SkipOnCloudGCP, "Test is skipped on GCP.");
        await SkipSecondUploadWithoutOverwriteFlagAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldSkipSecondUploadWithoutOverwriteFlagReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        Skip.For(SkipCondition.SkipOnCloudGCP, "Test is skipped on GCP.");
        await SkipSecondUploadWithoutOverwriteFlagAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldRunSecondUploadWithOverwriteFlagAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await RunSecondUploadWithOverwriteFlagAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldRunSecondUploadWithOverwriteFlagReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await RunSecondUploadWithOverwriteFlagAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFilesFromDirectoriesMatchingAsteriskWildcardAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await UploadFilesFromDirectoriesMatchingAsteriskWildcardAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFilesFromDirectoriesMatchingAsteriskWildcardReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await UploadFilesFromDirectoriesMatchingAsteriskWildcardAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFilesFromDirectoriesMatchingQuestionMarkWildcardAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await UploadFilesFromDirectoriesMatchingQuestionMarkWildcardAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFilesFromDirectoriesMatchingQuestionMarkWildcardReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await UploadFilesFromDirectoriesMatchingQuestionMarkWildcardAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFilesFromDirectoriesMatchingMixedWildcardsAsync()
    {
        Skip.OnNewDriver("BD#put-get-compression-type-casing", NewDriverCompressionTypeCasing);
        Skip.OnNewDriver("BD#put-get-error-details-empty-string", NewDriverErrorDetails);
        await UploadFilesFromDirectoriesMatchingMixedWildcardsAsync(PutResultColumns.OldDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadFilesFromDirectoriesMatchingMixedWildcardsReportingUppercaseCompressionAndEmptyErrorDetailsAsync()
    {
        Skip.OnOldDriver("BD#put-get-compression-type-casing", OldDriverCompressionTypeCasing);
        Skip.OnOldDriver("BD#put-get-error-details-empty-string", OldDriverErrorDetails);
        await UploadFilesFromDirectoriesMatchingMixedWildcardsAsync(PutResultColumns.NewDriver, TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData("GET")]
    [InlineData("PUT")]
    public async Task ShouldThrowWithoutQueryIdWhenPutGetRunsOnClosedConnectionAsync(string command)
    {
        // TODO(SNOW-4382591): SnowflakeDbException (SqlState/ErrorCode/QueryId)
        Skip.FutureMilestone();
        using var conn = Fixture.Factory.Create(Output);
        var names = default(ObjectNames);
        using (var open = Fixture.Factory.Create(Output))
        {
            open.Open();
            names = CreateObjects(open);
        }

        try
        {
            var ex = Assert.ThrowsAny<Exception>(
                () => ProcessFile(
                    command,
                    conn,
                    "unexisting_file.csv",
                    $"@{names.Schema}.{names.Stage}",
                    Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"))));
            AssertIsSnowflakeDbException(ex);
            Assert.Null(ExceptionQueryId(ex));
            Assert.Equal(ClosedConnectionErrorCode, ExceptionErrorCode(ex));
        }
        finally
        {
            using var cleanup = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            DropObjects(cleanup, names);
        }
    }

    [SnowflakeFact]
    public async Task ShouldReturnNoRowsWithoutThrowingWhenGetFileDoesNotExistAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException (SqlState/ErrorCode/QueryId)
        Skip.FutureMilestone();
        var outputDirectory = UniqueTempDir();
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            using var command = conn.CreateCommand();
            command.CommandText = $"GET @{names.Schema}.{names.Stage}/ file://{outputDirectory}";
            using var reader = command.ExecuteReader();
            Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        }
        finally
        {
            DropObjects(conn, names);
            DeleteDirectory(outputDirectory);
        }
    }

    [SnowflakeFact]
    public async Task ShouldThrowWithQueryIdWhenPutFileDoesNotExistAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException (SqlState/ErrorCode/QueryId)
        Skip.FutureMilestone();
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            var ex = Assert.ThrowsAny<Exception>(
                () => PutFile(
                    conn,
                    "non_existent_file.csv",
                    $"@{names.Schema}.{names.Stage}",
                    autoCompress: false,
                    PutResultColumns.OldDriver));
            AssertIsSnowflakeDbException(ex);
            Assert.NotNull(ExceptionQueryId(ex));
            Assert.Equal(IoErrorOnGetPutErrorCode, ExceptionErrorCode(ex));
        }
        finally
        {
            DropObjects(conn, names);
        }
    }

    [SnowflakeFact]
    public async Task ShouldProvideQueryIdWhenPutFailsAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException (SqlState/ErrorCode/QueryId)
        Skip.FutureMilestone();
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            var ex = Assert.ThrowsAny<Exception>(
                () => PutFile(
                    conn,
                    "unexisting_file.csv",
                    $"@{names.Schema}.{names.Stage}",
                    autoCompress: false,
                    PutResultColumns.OldDriver));
            AssertIsSnowflakeDbException(ex);
            var queryId = ExceptionQueryId(ex);

            Assert.NotEmpty(queryId!);
            Guid.Parse(queryId!);
            Assert.Equal(IoErrorOnGetPutErrorCode, ExceptionErrorCode(ex));
        }
        finally
        {
            DropObjects(conn, names);
        }
    }

    [SnowflakeFact]
    public async Task ShouldProvideQueryIdWhenPutHasSyntaxErrorAsync()
    {
        // TODO(SNOW-4382591): SnowflakeDbException (SqlState/ErrorCode/QueryId)
        Skip.FutureMilestone();
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            var ex = Assert.ThrowsAny<Exception>(
                () => PutFile(
                    conn,
                    "unexisting_file.csv SOME CODE FORCING SYNTAX ERROR",
                    $"@{names.Schema}.{names.Stage}",
                    autoCompress: false,
                    PutResultColumns.OldDriver));
            AssertIsSnowflakeDbException(ex);
            var queryId = ExceptionQueryId(ex);

            Assert.NotEmpty(queryId!);
            Guid.Parse(queryId!);
            Assert.Equal(SyntaxErrorCode, ExceptionErrorCode(ex));
            Assert.Null(ex.InnerException);
        }
        finally
        {
            DropObjects(conn, names);
        }
    }

    [SnowflakeTheory]
    [MemberData(nameof(PutGetCommandTestCases))]
    public async Task ShouldUploadCopyAndDownloadFileAsync(
        string sourceFileCompressionType,
        StageType stageType,
        string stagePath,
        bool autoCompress)
    {
        // TODO(SNOW-4382582): GET result reader columns
        Skip.FutureMilestone();
        Skip.When(
            !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("JENKINS_HOME")),
            "Test is skipped on Jenkins.");
        await RunPutGetRoundTripAsync(sourceFileCompressionType, stageType, stagePath, autoCompress, clientEncryption: true, cancellationToken: TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [MemberData(nameof(PutGetCommandForNamedStageWithoutClientSideEncryptionTestCases))]
    public async Task ShouldUploadCopyAndDownloadFileOnNamedStageWithoutClientSideEncryptionAsync(
        string sourceFileCompressionType,
        string stagePath,
        bool autoCompress)
    {
        // TODO(SNOW-4382582): GET result reader columns
        Skip.FutureMilestone();
        await RunPutGetRoundTripAsync(
                sourceFileCompressionType,
                StageType.NAMED,
                stagePath,
                autoCompress,
                clientEncryption: false,
                cancellationToken: TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [MemberData(nameof(PutGetGcsDownscopedCredentialTestCases))]
    public async Task ShouldUploadCopyAndDownloadFileWithGcsDownscopedCredentialAsync(StageType stageType, string stagePath)
    {
        // TODO(SNOW-4382582): GET result reader columns
        Skip.FutureMilestone();
        Skip.For(
            SkipCondition.SkipOnCloudAWS | SkipCondition.SkipOnCloudAzure,
            "Test is skipped on AWS.Test is skipped on Azure.");
        var outputDirectory = UniqueTempDir();
        var tempDir = UniqueTempDir();
        using var conn = Fixture.Factory.Create(
            Output,
            Fixture.Factory.Create(Output).ConnectionString + ";GCS_USE_DOWNSCOPED_CREDENTIAL=true");
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            var prepared = PrepareRoundTrip(
                tempDir,
                outputDirectory,
                names,
                "none",
                stageType,
                stagePath,
                autoCompress: false,
                clientEncryption: true);
            PutPrepared(conn, prepared);
            await CopyIntoTableAsync(conn, names, prepared, TestContext.Current.CancellationToken).ConfigureAwait(false);
            GetFile(conn, prepared);
        }
        finally
        {
            DropObjects(conn, names);
            DeleteDirectory(outputDirectory);
            DeleteDirectory(tempDir);
        }
    }

    [SnowflakeTheory]
    [MemberData(nameof(PutGetFileWithSpaceAndSingleQuoteTestCases))]
    public async Task ShouldUploadCopyAndDownloadFileWithSpaceInQuotedPathAsync(StageType stageType, string stagePath)
    {
        // TODO(SNOW-4382582): GET result reader columns
        Skip.FutureMilestone();
        Skip.When(
            !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("JENKINS_HOME")),
            "Test is skipped on Jenkins.");
        await RunPutGetRoundTripAsync(
                "none",
                stageType,
                stagePath,
                autoCompress: false,
                clientEncryption: true,
                cancellationToken: TestContext.Current.CancellationToken,
                makeFilePathWithSpace: true)
            .ConfigureAwait(false);
    }

    public static IEnumerable<object[]> PutGetCommandTestCases()
    {
        var compressionTypes = new[] { "none", "gzip", "bzip2", "brotli", "deflate", "raw_deflate", "zstd" };
        var stageTypes = (StageType[])Enum.GetValues(typeof(StageType));
        var stagePaths = new[] { "", "/TEST_PATH", "/DEEP/TEST_PATH" };
        foreach (var compressionType in compressionTypes)
            foreach (var stageType in stageTypes)
                foreach (var stagePath in stagePaths)
                    foreach (var autoCompress in new[] { false, true })
                        yield return new object[] { compressionType, stageType, stagePath, autoCompress };
    }

    public static IEnumerable<object[]> PutGetCommandForNamedStageWithoutClientSideEncryptionTestCases()
    {
        var compressionTypes = new[] { "none", "gzip" };
        var stagePaths = new[] { "", "/DEEP/TEST_PATH" };
        foreach (var compressionType in compressionTypes)
            foreach (var stagePath in stagePaths)
                foreach (var autoCompress in new[] { false, true })
                    yield return new object[] { compressionType, stagePath, autoCompress };
    }

    public static IEnumerable<object[]> PutGetFileWithSpaceAndSingleQuoteTestCases()
    {
        foreach (var stageType in (StageType[])Enum.GetValues(typeof(StageType)))
            yield return new object[] { stageType, "/STAGE PATH WITH SPACE" };
    }

    public static IEnumerable<object[]> PutGetGcsDownscopedCredentialTestCases()
    {
        var stagePaths = new[] { "", "/TEST_PATH" };
        foreach (var stageType in (StageType[])Enum.GetValues(typeof(StageType)))
            foreach (var stagePath in stagePaths)
                yield return new object[] { stageType, stagePath };
    }

    private async Task RunPutGetRoundTripAsync(
        string sourceFileCompressionType,
        StageType stageType,
        string stagePath,
        bool autoCompress,
        bool clientEncryption,
        CancellationToken cancellationToken,
        bool makeFilePathWithSpace = false)
    {
        var outputDirectory = UniqueTempDir();
        var tempDir = UniqueTempDir();
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            var prepared = PrepareRoundTrip(
                tempDir,
                outputDirectory,
                names,
                sourceFileCompressionType,
                stageType,
                stagePath,
                autoCompress,
                clientEncryption,
                makeFilePathWithSpace);
            PutPrepared(conn, prepared, encloseInSingleQuotes: makeFilePathWithSpace);
            await CopyIntoTableAsync(conn, names, prepared, cancellationToken, makeFilePathWithSpace).ConfigureAwait(false);
            GetFile(conn, prepared, makeFilePathWithSpace);
        }
        finally
        {
            DropObjects(conn, names);
            DeleteDirectory(outputDirectory);
            DeleteDirectory(tempDir);
        }
    }

    private static PreparedTransfer PrepareRoundTrip(
        string tempDir,
        string outputDirectory,
        ObjectNames names,
        string sourceFileCompressionType,
        StageType stageType,
        string stagePath,
        bool autoCompress,
        bool clientEncryption,
        bool makeFilePathWithSpace = false)
    {
        var fileName = Guid.NewGuid().ToString("N") + ".csv" +
                       (autoCompress ? CompressionExtension(sourceFileCompressionType) : "");
        var sourceFolderWithSpace = $"{Guid.NewGuid():N} source file path with space";
        var inputPathBase = makeFilePathWithSpace
            ? Path.Combine(outputDirectory, sourceFolderWithSpace)
            : tempDir;
        var inputFilePath = Path.Combine(inputPathBase, fileName);

        string destCompressionType;
        string outputFileName;
        if (sourceFileCompressionType == "none" && autoCompress)
        {
            destCompressionType = "gzip";
            outputFileName = $"{fileName}.gz";
        }
        else
        {
            destCompressionType = sourceFileCompressionType;
            outputFileName = fileName;
        }

        var destinationFolderWithSpace = $"{Guid.NewGuid():N} destination file path with space";
        var outputPathBase = makeFilePathWithSpace
            ? Path.Combine(outputDirectory, destinationFolderWithSpace)
            : outputDirectory;
        var outputFilePath = Path.Combine(outputPathBase, outputFileName);
        if (makeFilePathWithSpace)
        {
            Directory.CreateDirectory(inputPathBase);
            Directory.CreateDirectory(outputPathBase);
        }

        PrepareFileData(inputFilePath);

        var internalStagePath = stageType switch
        {
            StageType.USER => $"@~{stagePath}",
            StageType.TABLE => $"@{names.Schema}.%{names.Table}{stagePath}",
            StageType.NAMED => clientEncryption
                ? $"@{names.Schema}.{names.Stage}{stagePath}"
                : $"@{names.Schema}.{names.StageSse}{stagePath}",
            _ => throw new ArgumentOutOfRangeException(nameof(stageType))
        };

        return new PreparedTransfer
        {
            InputFilePath = inputFilePath,
            InternalStagePath = internalStagePath,
            FileName = fileName,
            OutputFilePath = outputFilePath,
            OutputDirectory = outputDirectory,
            StageType = stageType,
            SourceCompressionType = sourceFileCompressionType,
            DestCompressionType = destCompressionType,
            AutoCompress = autoCompress
        };
    }

    private static string CompressionExtension(string sourceFileCompressionType) =>
        sourceFileCompressionType switch
        {
            "none" => "",
            "gzip" => ".gz",
            "bzip2" => ".bz2",
            "brotli" => ".br",
            "deflate" => ".deflate",
            "raw_deflate" => ".raw_deflate",
            "zstd" => ".zst",
            _ => throw new ArgumentOutOfRangeException(nameof(sourceFileCompressionType))
        };

    private static string PutFile(
        ISnowflakeDbConnection conn,
        string inputFilePath,
        string internalStagePath,
        bool autoCompress,
        PutResultColumns columns,
        string additionalAttribute = "",
        string expectedStatus = "UPLOADED",
        string? sourceCompressionType = null,
        string? destCompressionType = null,
        bool encloseInSingleQuotes = false)
    {
        using var command = conn.CreateCommand();
        var putQuery = encloseInSingleQuotes
            ? $"PUT 'file://{inputFilePath.Replace("\\", "/")}' '{internalStagePath}'"
            : $"PUT file://{inputFilePath} {internalStagePath}";
        putQuery += $" AUTO_COMPRESS={(autoCompress ? "TRUE" : "FALSE")}" + $" {additionalAttribute}";
        command.CommandText = putQuery;
        using var reader = command.ExecuteReader();
        try
        {
            Assert.True(reader.Read());
        }
        catch (Exception e) when (e is OldSnowflakeDbException or NewSnowflakeDbException)
        {
            Assert.Equal(ExceptionQueryId(e), CommandQueryId(command));
            throw;
        }

        var queryId = ReaderQueryId(reader, command);
        Assert.Equal(queryId, CommandQueryId(command));
        Assert.Equal(expectedStatus, reader.GetString(ResultStatusOrdinal));
        if (autoCompress)
        {
            Assert.Equal(columns.CompressionType(sourceCompressionType), reader.GetString(SourceCompressionTypeOrdinal));
            Assert.Equal(columns.CompressionType(destCompressionType), reader.GetString(DestinationCompressionTypeOrdinal));
        }
        else
        {
            Assert.Equal(columns.CompressionType("none"), reader.GetString(SourceCompressionTypeOrdinal));
            Assert.Equal(columns.CompressionType("none"), reader.GetString(DestinationCompressionTypeOrdinal));
        }

        Assert.Equal(columns.SuccessErrorDetails, reader.GetString(ErrorDetailsOrdinal));
        return queryId!;
    }

    private static void PutPrepared(
        ISnowflakeDbConnection conn,
        PreparedTransfer prepared,
        string additionalAttribute = "",
        string expectedStatus = "UPLOADED",
        bool encloseInSingleQuotes = false) =>
        PutFile(
            conn,
            prepared.InputFilePath,
            prepared.InternalStagePath,
            prepared.AutoCompress,
            PutResultColumns.OldDriver,
            additionalAttribute,
            expectedStatus,
            prepared.SourceCompressionType,
            prepared.DestCompressionType,
            encloseInSingleQuotes);

    private static async Task CopyIntoTableAsync(
        ISnowflakeDbConnection conn,
        ObjectNames names,
        PreparedTransfer prepared,
        CancellationToken cancellationToken,
        bool encloseInSingleQuotes = false)
    {
        using var command = conn.CreateCommand();
        command.CommandText = prepared.StageType == StageType.TABLE
            ? $"COPY INTO {names.Schema}.{names.Table}"
            : encloseInSingleQuotes
                ? $"COPY INTO {names.Schema}.{names.Table} FROM '{prepared.InternalStagePath}/{prepared.FileName}'"
                : $"COPY INTO {names.Schema}.{names.Table} FROM {prepared.InternalStagePath}/{prepared.FileName}";

        await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);

        command.CommandText = $"SELECT * FROM {names.Schema}.{names.Table}";
        using (var reader = await command.ExecuteReaderAsync(cancellationToken).ConfigureAwait(false))
        {
            while (reader.Read())
            {
                for (var i = 0; i < s_colData.Length; i++)
                    Assert.Equal(s_colData[i], reader.GetString(i));
            }
        }

        command.CommandText = $"SELECT COUNT(*) FROM {names.Schema}.{names.Table}";
        Assert.Equal(
            NumberOfRows,
            (long)(await command.ExecuteScalarAsync(cancellationToken).ConfigureAwait(false))!);
    }

    private static void GetFile(
        ISnowflakeDbConnection conn,
        PreparedTransfer prepared,
        bool encloseInSingleQuotes = false)
    {
        using var command = conn.CreateCommand();
        var getQuery = encloseInSingleQuotes
            ? $"GET '{prepared.InternalStagePath}/{prepared.FileName}' 'file://{Path.GetDirectoryName(prepared.OutputFilePath)!.Replace("\\", "/")}'"
            : $"GET {prepared.InternalStagePath}/{prepared.FileName} file://{prepared.OutputDirectory}";
        command.CommandText = getQuery;
        using var reader = command.ExecuteReader();
        Assert.True(reader.Read());
        Assert.Equal("DOWNLOADED", reader.GetString(ResultStatusOrdinal));

        foreach (var line in ReadOutputFileLines(prepared))
        {
            if (string.IsNullOrEmpty(line))
                continue;
            var values = line.Split(',');
            for (var i = 0; i < s_colData.Length; i++)
                Assert.Equal(s_colData[i], values[i]);
        }
    }

    private static void ProcessFile(
        string command,
        ISnowflakeDbConnection connection,
        string inputFilePath,
        string internalStagePath,
        string outputDirectory)
    {
        switch (command)
        {
            case "GET":
                using (var cmd = connection.CreateCommand())
                {
                    cmd.CommandText = $"GET {internalStagePath}/ file://{outputDirectory}";
                    using var reader = cmd.ExecuteReader();
                    Assert.True(reader.Read());
                }

                break;
            case "PUT":
                PutFile(connection, inputFilePath, internalStagePath, autoCompress: false, PutResultColumns.OldDriver);
                break;
        }
    }

    private static string[] ReadOutputFileLines(PreparedTransfer prepared)
    {
        using var outputStream = File.OpenRead(prepared.OutputFilePath);
        using var processedOutputStream = prepared.CompressedByTheDriver ? Unzip(outputStream) : outputStream;
        return SplitLines(processedOutputStream);
    }

    private static Stream Unzip(Stream stream)
    {
        using var decompressingStream = new GZipStream(stream, CompressionMode.Decompress);
        var decompressedStream = new MemoryStream();
        decompressingStream.CopyTo(decompressedStream);
        return decompressedStream;
    }

    private static string[] SplitLines(Stream stream)
    {
        var bytes = new byte[stream.Length];
        stream.Position = 0;
        var readBytes = stream.Read(bytes, 0, (int)stream.Length);
        Assert.Equal(stream.Length, readBytes);
        return Encoding.UTF8.GetString(bytes).Split('\n');
    }

    private static void PrepareFileData(string file)
    {
        var rawDataRow = string.Join(",", s_colData) + "\n";
        var rawData = string.Concat(Enumerable.Repeat(rawDataRow, (int)NumberOfRows));
        var directory = Path.GetDirectoryName(file);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);
        File.WriteAllText(file, rawData);
    }

    private static void PrepareFileData(List<string> files) => files.ForEach(PrepareFileData);

    private static void VerifyFilesAreUploaded(ISnowflakeDbConnection conn, ICollection<string> files, string stage)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = $"LIST {stage}";
        using var dbDataReader = cmd.ExecuteReader();
        var rowCount = 0;
        while (dbDataReader.Read())
            rowCount++;
        Assert.Equal(files.Count, rowCount);
    }

    private async Task<ISnowflakeDbConnection> OpenConnectionAsync(CancellationToken cancellationToken)
    {
        var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        return conn;
    }

    private async Task UploadFilesMatchingAsteriskWildcardAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var tempDir = UniqueTempDir();
        var absolutePathPrefix = Path.Combine(tempDir, Guid.NewGuid().ToString("N"));
        var files = new List<string>
        {
            $"{absolutePathPrefix}_one.csv",
            $"{absolutePathPrefix}_two.csv",
            $"{absolutePathPrefix}_three.csv"
        };
        PrepareFileData(files);
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PutFile(
                conn,
                $"{absolutePathPrefix}*",
                $"@{names.Schema}.{names.Stage}",
                autoCompress: false,
                columns);
            VerifyFilesAreUploaded(conn, files, $"@{names.Schema}.{names.Stage}");
        }
        finally
        {
            DropObjects(conn, names);
            DeleteDirectory(tempDir);
        }
    }

    private async Task UploadOnlyFilesMatchingAsteriskWildcardWithExtensionAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var tempDir = UniqueTempDir();
        var absolutePathPrefix = Path.Combine(tempDir, Guid.NewGuid().ToString("N"));
        var files = new List<string>
        {
            $"{absolutePathPrefix}_one.csv",
            $"{absolutePathPrefix}_two.csv",
            $"{absolutePathPrefix}_three.csv"
        };
        PrepareFileData(files);
        PrepareFileData($"{absolutePathPrefix}_four.txt");
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PutFile(
                conn,
                $"{absolutePathPrefix}*.csv",
                $"@{names.Schema}.{names.Stage}",
                autoCompress: false,
                columns);
            VerifyFilesAreUploaded(conn, files, $"@{names.Schema}.{names.Stage}");
        }
        finally
        {
            DropObjects(conn, names);
            DeleteDirectory(tempDir);
        }
    }

    private async Task UploadFilesMatchingQuestionMarkWildcardAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var tempDir = UniqueTempDir();
        var absolutePathPrefix = Path.Combine(tempDir, Guid.NewGuid().ToString("N"));
        var files = new List<string>
        {
            $"{absolutePathPrefix}_1.csv",
            $"{absolutePathPrefix}_2.csv",
            $"{absolutePathPrefix}_3.csv"
        };
        PrepareFileData(files);
        PrepareFileData($"{absolutePathPrefix}_four.csv");
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PutFile(
                conn,
                $"{absolutePathPrefix}_?.csv",
                $"@{names.Schema}.{names.Stage}",
                autoCompress: false,
                columns);
            VerifyFilesAreUploaded(conn, files, $"@{names.Schema}.{names.Stage}");
        }
        finally
        {
            DropObjects(conn, names);
            DeleteDirectory(tempDir);
        }
    }

    private async Task UploadFileFromRelativePathWithoutDirectoryAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var inputFilePath = $"{Guid.NewGuid():N}_1.csv";
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PrepareFileData(inputFilePath);
            PutFile(conn, inputFilePath, $"@{names.Schema}.{names.Stage}", autoCompress: false, columns);
            VerifyFilesAreUploaded(conn, [inputFilePath], $"@{names.Schema}.{names.Stage}");
        }
        finally
        {
            DropObjects(conn, names);
            TryDeleteFile(inputFilePath);
        }
    }

    private async Task ProvideQueryIdWhenPutSucceedsAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var inputFilePath = $"{Guid.NewGuid():N}_1.csv";
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PrepareFileData(inputFilePath);
            var queryId = PutFile(conn, inputFilePath, $"@{names.Schema}.{names.Stage}", autoCompress: false, columns);
            Assert.NotNull(queryId);
            Guid.Parse(queryId);
            VerifyFilesAreUploaded(conn, [inputFilePath], $"@{names.Schema}.{names.Stage}");
        }
        finally
        {
            DropObjects(conn, names);
            TryDeleteFile(inputFilePath);
        }
    }

    private async Task UploadFileFromRelativePathWithDirectoryAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var guid = Guid.NewGuid().ToString("N");
        Directory.CreateDirectory(guid);
        var inputFilePath = $"{guid}{Path.DirectorySeparatorChar}{guid}_1.csv";
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PrepareFileData(inputFilePath);
            PutFile(conn, inputFilePath, $"@{names.Schema}.{names.Stage}", autoCompress: false, columns);
            VerifyFilesAreUploaded(conn, [inputFilePath], $"@{names.Schema}.{names.Stage}");
        }
        finally
        {
            DropObjects(conn, names);
            DeleteDirectory(guid);
        }
    }

    private async Task UploadFilesMatchingRelativePathAsteriskWildcardAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var relativePath = Guid.NewGuid().ToString("N");
        var files = new List<string>
        {
            $"{relativePath}_one.csv",
            $"{relativePath}_two.csv",
            $"{relativePath}_three.csv"
        };
        PrepareFileData(files);
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PutFile(conn, $"{relativePath}*", $"@{names.Schema}.{names.Stage}", autoCompress: false, columns);
            VerifyFilesAreUploaded(conn, files, $"@{names.Schema}.{names.Stage}");
        }
        finally
        {
            DropObjects(conn, names);
            foreach (var file in files)
                TryDeleteFile(file);
        }
    }

    private async Task SkipSecondUploadWithoutOverwriteFlagAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var inputFilePath = Path.Combine(Path.GetTempPath(), $"{Guid.NewGuid():N}.csv");
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PrepareFileData(inputFilePath);
            PutFile(
                conn,
                inputFilePath,
                $"@{names.Schema}.{names.Stage}",
                autoCompress: false,
                columns,
                expectedStatus: "UPLOADED");
            VerifyFilesAreUploaded(conn, [inputFilePath], $"@{names.Schema}.{names.Stage}");
            PutFile(
                conn,
                inputFilePath,
                $"@{names.Schema}.{names.Stage}",
                autoCompress: false,
                columns,
                expectedStatus: "SKIPPED");
        }
        finally
        {
            DropObjects(conn, names);
            TryDeleteFile(inputFilePath);
        }
    }

    private async Task RunSecondUploadWithOverwriteFlagAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var inputFilePath = Path.Combine(Path.GetTempPath(), $"{Guid.NewGuid():N}.csv");
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PrepareFileData(inputFilePath);
            PutFile(
                conn,
                inputFilePath,
                $"@{names.Schema}.{names.Stage}",
                autoCompress: false,
                columns,
                additionalAttribute: "OVERWRITE=TRUE",
                expectedStatus: "UPLOADED");
            VerifyFilesAreUploaded(conn, [inputFilePath], $"@{names.Schema}.{names.Stage}");
            PutFile(
                conn,
                inputFilePath,
                $"@{names.Schema}.{names.Stage}",
                autoCompress: false,
                columns,
                additionalAttribute: "OVERWRITE=TRUE",
                expectedStatus: "UPLOADED");
        }
        finally
        {
            DropObjects(conn, names);
            TryDeleteFile(inputFilePath);
        }
    }

    private async Task UploadFilesFromDirectoriesMatchingAsteriskWildcardAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var (path, _, files) = CreateMatchingDirectories();
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PutFile(
                conn,
                $"{path}*{Path.DirectorySeparatorChar}*",
                $"@{names.Schema}.{names.Stage}",
                autoCompress: false,
                columns);
            VerifyFilesAreUploaded(conn, files, $"@{names.Schema}.{names.Stage}");
        }
        finally
        {
            DropObjects(conn, names);
            DeleteMatchingDirectories(path);
        }
    }

    private async Task UploadFilesFromDirectoriesMatchingQuestionMarkWildcardAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var (path, guid, files) = CreateMatchingDirectories();
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PutFile(
                conn,
                $"{path}_?{Path.DirectorySeparatorChar}{guid}_?_file.csv",
                $"@{names.Schema}.{names.Stage}",
                autoCompress: false,
                columns);
            VerifyFilesAreUploaded(conn, files, $"@{names.Schema}.{names.Stage}");
        }
        finally
        {
            DropObjects(conn, names);
            DeleteMatchingDirectories(path);
        }
    }

    private async Task UploadFilesFromDirectoriesMatchingMixedWildcardsAsync(PutResultColumns columns, CancellationToken cancellationToken)
    {
        var (path, guid, files) = CreateMatchingDirectories();
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var names = CreateObjects(conn);
        try
        {
            PutFile(
                conn,
                $"{path}_*{Path.DirectorySeparatorChar}{guid}_?_file.csv",
                $"@{names.Schema}.{names.Stage}",
                autoCompress: false,
                columns);
            VerifyFilesAreUploaded(conn, files, $"@{names.Schema}.{names.Stage}");
        }
        finally
        {
            DropObjects(conn, names);
            DeleteMatchingDirectories(path);
        }
    }

    private static (string Path, string Guid, List<string> Files) CreateMatchingDirectories()
    {
        var guid = Guid.NewGuid().ToString("N");
        var path = Path.Combine(Path.GetTempPath(), guid);
        var files = new List<string>();
        for (var i = 0; i < MatchingDirectoryCount; i++)
        {
            var filePath = $"{path}_{i}";
            Directory.CreateDirectory(filePath);
            var fullPath = $"{filePath}{Path.DirectorySeparatorChar}{guid}_{i}_file.csv";
            PrepareFileData(fullPath);
            files.Add(fullPath);
        }

        return (path, guid, files);
    }

    private static void DeleteMatchingDirectories(string path)
    {
        for (var i = 0; i < MatchingDirectoryCount; i++)
            DeleteDirectory($"{path}_{i}");
    }

    private static ObjectNames CreateObjects(ISnowflakeDbConnection conn)
    {
        var schema = CurrentSchema(conn);
        var suffix = Guid.NewGuid().ToString("N");
        var table = $"TABLE_{suffix}";
        var stage = $"STAGE_{suffix}";
        var stageSse = $"STAGE_{suffix}_SSE";
        var columnNamesWithTypes = string.Join(",", s_colName.Select(col => col + " STRING"));
        using var command = conn.CreateCommand();
        command.CommandText = $"CREATE OR REPLACE TABLE {schema}.{table} ({columnNamesWithTypes})";
        command.ExecuteNonQuery();
        command.CommandText = $"CREATE OR REPLACE STAGE {schema}.{stage}";
        command.ExecuteNonQuery();
        command.CommandText = $"CREATE OR REPLACE STAGE {schema}.{stageSse} ENCRYPTION = (TYPE = 'SNOWFLAKE_SSE')";
        command.ExecuteNonQuery();
        return new ObjectNames { Schema = schema, Table = table, Stage = stage, StageSse = stageSse };
    }

    private static void DropObjects(ISnowflakeDbConnection conn, ObjectNames names)
    {
        using var command = conn.CreateCommand();
        try
        {
            command.CommandText = $"DROP STAGE IF EXISTS {names.Schema}.{names.Stage}";
            command.ExecuteNonQuery();
        }
        catch (Exception)
        {
        }

        try
        {
            command.CommandText = $"DROP STAGE IF EXISTS {names.Schema}.{names.StageSse}";
            command.ExecuteNonQuery();
        }
        catch (Exception)
        {
        }

        try
        {
            command.CommandText = $"DROP TABLE IF EXISTS {names.Schema}.{names.Table}";
            command.ExecuteNonQuery();
        }
        catch (Exception)
        {
        }
    }

    private static string CurrentSchema(ISnowflakeDbConnection conn)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select current_schema()";
        return Convert.ToString(cmd.ExecuteScalar(), CultureInfo.InvariantCulture)!;
    }

    private static string UniqueTempDir()
    {
        var dir = Path.Combine(Path.GetTempPath(), Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);
        return dir;
    }

    private static void DeleteDirectory(string path)
    {
        try
        {
            if (Directory.Exists(path))
                Directory.Delete(path, true);
        }
        catch (Exception)
        {
        }
    }

    private static void TryDeleteFile(string path)
    {
        try
        {
            File.Delete(path);
        }
        catch (Exception)
        {
        }
    }

    private static string? CommandQueryId(IDbCommand command) =>
        command switch
        {
            OldSnowflakeDbCommand oldCmd => oldCmd.GetQueryId(),
            NewSnowflakeDbCommand newCmd => newCmd.GetQueryId(),
            _ => null
        };

    private static string? ReaderQueryId(IDataReader reader, IDbCommand command) =>
        reader is OldSnowflakeDbDataReader oldReader ? oldReader.GetQueryId() : CommandQueryId(command);

    private static void AssertIsSnowflakeDbException(Exception ex)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            Assert.IsType<OldSnowflakeDbException>(ex);
        else
            Assert.IsType<NewSnowflakeDbException>(ex);
    }

    private static string? ExceptionQueryId(Exception ex) =>
        ex switch
        {
            OldSnowflakeDbException old => old.QueryId,
            NewSnowflakeDbException neu => neu.QueryId,
            _ => null
        };

    private static int? ExceptionErrorCode(Exception ex) =>
        ex switch
        {
            OldSnowflakeDbException old => old.ErrorCode,
            NewSnowflakeDbException neu => neu.ErrorCode,
            _ => null
        };

    private sealed class PutResultColumns
    {
        public static readonly PutResultColumns OldDriver = new(uppercaseCompressionTypes: false, successErrorDetails: null);
        public static readonly PutResultColumns NewDriver = new(uppercaseCompressionTypes: true, successErrorDetails: string.Empty);

        private readonly bool _uppercaseCompressionTypes;

        private PutResultColumns(bool uppercaseCompressionTypes, string? successErrorDetails)
        {
            _uppercaseCompressionTypes = uppercaseCompressionTypes;
            SuccessErrorDetails = successErrorDetails;
        }

        public string? SuccessErrorDetails { get; }

        public string? CompressionType(string? name) =>
            _uppercaseCompressionTypes ? name?.ToUpperInvariant() : name;
    }

    private sealed class ObjectNames
    {
        public string Schema { get; set; } = string.Empty;
        public string Table { get; set; } = string.Empty;
        public string Stage { get; set; } = string.Empty;
        public string StageSse { get; set; } = string.Empty;
    }

    private sealed class PreparedTransfer
    {
        public string InputFilePath { get; set; } = string.Empty;
        public string InternalStagePath { get; set; } = string.Empty;
        public string FileName { get; set; } = string.Empty;
        public string OutputFilePath { get; set; } = string.Empty;
        public string OutputDirectory { get; set; } = string.Empty;
        public StageType StageType { get; set; }
        public string SourceCompressionType { get; set; } = string.Empty;
        public string DestCompressionType { get; set; } = string.Empty;
        public bool AutoCompress { get; set; }
        public bool CompressedByTheDriver => SourceCompressionType == "none" && AutoCompress;
    }
}
