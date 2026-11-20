using System.Data.Common;
using System.Globalization;
using System.IO.Compression;
using System.Text;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using NewQueryStatus = Snowflake.Data.QueryStatus;
using NewSnowflakeDbCommand = Snowflake.Data.SnowflakeDbCommand;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldQueryStatus = Snowflake.Data.Core.QueryStatus;
using OldSnowflakeDbCommand = Snowflake.Data.Client.SnowflakeDbCommand;
using OldSnowflakeDbDataReader = Snowflake.Data.Client.SnowflakeDbDataReader;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Reference;

public sealed class SFPutFromMemoryTest : ReferenceTestBase
{
    private const int SourceFileSizeOrdinal = 2;
    private const int DestinationFileNameOrdinal = 1;
    private const int DestinationFileSizeOrdinal = 3;
    private const int SourceCompressionTypeOrdinal = 4;
    private const int DestinationCompressionTypeOrdinal = 5;
    private const int ResultStatusOrdinal = 6;
    private const int ErrorDetailsOrdinal = 7;
    private const int ClosedConnectionErrorCode = 270059;
    private const int SyntaxErrorCode = 1003;
    private const string SyntaxErrorSqlState = "42000";
    private static readonly string[] s_colData = ["FIRST", "SECOND", "THIRD"];

    public SFPutFromMemoryTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    public enum StageType
    {
        TABLE,
        NAMED
    }

    [SnowflakeTheory]
    [InlineData(StageType.NAMED, true, 4)]
    [InlineData(StageType.NAMED, false, 4)]
    [InlineData(StageType.TABLE, true, 4)]
    [InlineData(StageType.TABLE, false, 4)]
    [InlineData(StageType.NAMED, true, 200)]
    [InlineData(StageType.NAMED, false, 200)]
    [InlineData(StageType.TABLE, true, 200)]
    [InlineData(StageType.TABLE, false, 200)]
    public async Task ShouldUploadCsvFromMemoryAsync(StageType stageType, bool autoCompress, int rows)
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();
        await PutFromMemoryAndVerifyRoundTripAsync(MakeCsvBytes(rows), stageType, autoCompress, expectCsvRoundTrip: true, 1, TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData(StageType.NAMED)]
    [InlineData(StageType.TABLE)]
    public async Task ShouldUploadEmptyStreamWithoutAutoCompressAsync(StageType stageType)
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();
        await PutFromMemoryAndVerifyRoundTripAsync([], stageType, autoCompress: false, expectCsvRoundTrip: false, 1, TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData(StageType.NAMED)]
    [InlineData(StageType.TABLE)]
    public async Task ShouldUploadEmptyStreamWithAutoCompressAsync(StageType stageType)
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();
        await PutFromMemoryAndVerifyRoundTripAsync([], stageType, autoCompress: true, expectCsvRoundTrip: false, 1, TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData(StageType.NAMED, true)]
    [InlineData(StageType.NAMED, false)]
    [InlineData(StageType.TABLE, true)]
    [InlineData(StageType.TABLE, false)]
    public async Task ShouldUploadBinaryDataFromMemoryAsync(StageType stageType, bool autoCompress)
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();
        var binary = new byte[512];
        new Random(42).NextBytes(binary);
        await PutFromMemoryAndVerifyRoundTripAsync(binary, stageType, autoCompress, expectCsvRoundTrip: false, 1, TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData(StageType.NAMED, true)]
    [InlineData(StageType.TABLE, false)]
    public async Task ShouldUploadLargePayloadFromMemoryAsync(StageType stageType, bool autoCompress)
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();
        var csv = MakeCsvBytes(5_000_000);
        await PutFromMemoryAndVerifyRoundTripAsync(csv, stageType, autoCompress, expectCsvRoundTrip: true, expectedFilesCount: 1, TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldUploadSubsequentStreamsToSameStageAsync()
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();

        var tableName = UniqueName("PUT_MEM");
        var stageName = UniqueName("STAGE_MEM");
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var schemaName = CurrentSchema(conn);
        try
        {
            CreateTableAndStage(conn, schemaName, tableName, stageName);
            var stagePath = StagePath(StageType.NAMED, schemaName, tableName, stageName);

            var csv1 = MakeCsvBytes();
            string destFileName1;
            using (var cmd = conn.CreateCommand())
            {
                cmd.CommandText = $"PUT file://first.csv {stagePath} AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
                using var reader = ExecutePutFromMemory(cmd, new MemoryStream(csv1));
                Assert.True(reader.Read());
                Assert.Equal("UPLOADED", reader.GetString(ResultStatusOrdinal));
                destFileName1 = reader.GetString(DestinationFileNameOrdinal);
            }

            var csv2 = MakeCsvBytes(8);
            string destFileName2;
            using (var cmd = conn.CreateCommand())
            {
                cmd.CommandText = $"PUT file://second.csv {stagePath} AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
                using var reader = ExecutePutFromMemory(cmd, new MemoryStream(csv2));
                Assert.True(reader.Read());
                Assert.Equal("UPLOADED", reader.GetString(ResultStatusOrdinal));
                destFileName2 = reader.GetString(DestinationFileNameOrdinal);
            }

            Assert.NotEqual(destFileName1, destFileName2);
            VerifyFilesAreUploaded(conn, stagePath, 2);

            using (var cmd = conn.CreateCommand())
            {
                cmd.CommandText = $"TRUNCATE TABLE IF EXISTS {schemaName}.{tableName}";
                cmd.ExecuteNonQuery();
                cmd.CommandText = $"COPY INTO {schemaName}.{tableName} FROM {stagePath}/{destFileName1}";
                cmd.ExecuteNonQuery();
                cmd.CommandText = $"COPY INTO {schemaName}.{tableName} FROM {stagePath}/{destFileName2}";
                cmd.ExecuteNonQuery();
                cmd.CommandText = $"SELECT COUNT(*) FROM {schemaName}.{tableName}";
                Assert.Equal(12L, (long)cmd.ExecuteScalar()!);
            }

            CleanupStageFile(conn, stagePath, destFileName1);
            CleanupStageFile(conn, stagePath, destFileName2);
        }
        finally
        {
            DropStageAndTable(conn, schemaName, stageName, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData(StageType.NAMED, true)]
    [InlineData(StageType.NAMED, false)]
    [InlineData(StageType.TABLE, true)]
    [InlineData(StageType.TABLE, false)]
    public async Task ShouldDetectPreCompressedGzipStreamAsync(StageType stageType, bool autoCompress)
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();
        var gzipped = MakeGzipBytes(MakeCsvBytes());
        await PutFromMemoryAndVerifyUploadAsync(gzipped, stageType, autoCompress, expectedSourceCompression: autoCompress ? "gzip" : "none", TestContext.Current.CancellationToken)
            .ConfigureAwait(false);
    }

    [SnowflakeFact]
    public async Task ShouldKeepCallerStreamReadableAfterUploadAsync()
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();

        var tableName = UniqueName("PUT_MEM");
        var stageName = UniqueName("STAGE_MEM");
        var data = MakeCsvBytes();
        var stream = new MemoryStream(data);
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var schemaName = CurrentSchema(conn);
        try
        {
            CreateTableAndStage(conn, schemaName, tableName, stageName);
            var stagePath = StagePath(StageType.NAMED, schemaName, tableName, stageName);
            var fileName = $"{Guid.NewGuid():N}.csv";

            string destFileName;
            using (var cmd = conn.CreateCommand())
            {
                cmd.CommandText = $"PUT file://{fileName} {stagePath} AUTO_COMPRESS=TRUE OVERWRITE=TRUE";
                using var reader = ExecutePutFromMemory(cmd, stream);
                Assert.True(reader.Read());
                Assert.Equal("UPLOADED", reader.GetString(ResultStatusOrdinal));
                destFileName = reader.GetString(DestinationFileNameOrdinal);
            }

            stream.Position = 0;
            var readBack = new byte[stream.Length];
            var bytesRead = stream.Read(readBack, 0, readBack.Length);
            Assert.Equal(data.Length, bytesRead);
            Assert.Equal(data, readBack);

            CleanupStageFile(conn, stagePath, destFileName);
        }
        finally
        {
            stream.Dispose();
            DropStageAndTable(conn, schemaName, stageName, tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldUploadCompressedFileSmallerThanOriginalAsync()
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();

        var tableName = UniqueName("PUT_MEM");
        var stageName = UniqueName("STAGE_MEM");
        var csv = MakeCsvBytes(500);
        var stream = new MemoryStream(csv);
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var schemaName = CurrentSchema(conn);
        try
        {
            CreateTableAndStage(conn, schemaName, tableName, stageName);
            var stagePath = StagePath(StageType.NAMED, schemaName, tableName, stageName);
            var fileName = $"{Guid.NewGuid():N}.csv";

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"PUT file://{fileName} {stagePath} AUTO_COMPRESS=TRUE OVERWRITE=TRUE";
            using var reader = ExecutePutFromMemory(cmd, stream);
            Assert.True(reader.Read());

            var srcSize = long.Parse(reader.GetString(SourceFileSizeOrdinal), CultureInfo.InvariantCulture);
            var destSize = long.Parse(reader.GetString(DestinationFileSizeOrdinal), CultureInfo.InvariantCulture);
            var destFileName = reader.GetString(DestinationFileNameOrdinal);
            Assert.Equal(csv.Length, srcSize);
            Assert.True(destSize < srcSize, $"Compressed size {destSize} should be smaller than original {srcSize}");
            Assert.Equal("gzip", reader.GetString(DestinationCompressionTypeOrdinal));

            CleanupStageFile(conn, stagePath, destFileName);
        }
        finally
        {
            stream.Dispose();
            DropStageAndTable(conn, schemaName, stageName, tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldExposeQueryIdAfterSuccessfulUploadAsync()
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();

        var tableName = UniqueName("PUT_MEM");
        var stageName = UniqueName("STAGE_MEM");
        var data = MakeCsvBytes();
        var stream = new MemoryStream(data);
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        var schemaName = CurrentSchema(conn);
        try
        {
            CreateTableAndStage(conn, schemaName, tableName, stageName);
            var stagePath = StagePath(StageType.NAMED, schemaName, tableName, stageName);
            var fileName = $"{Guid.NewGuid():N}.csv";

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"PUT file://{fileName} {stagePath} AUTO_COMPRESS=FALSE OVERWRITE=TRUE";
            using var reader = ExecutePutFromMemory(cmd, stream);
            Assert.True(reader.Read());

            var queryId = ReaderQueryId(reader, cmd);
            var destFileName = reader.GetString(DestinationFileNameOrdinal);
            Assert.NotNull(queryId);
            Guid.Parse(queryId);
            Assert.Equal(queryId, CommandQueryId(cmd));
            AssertQuerySuccess(cmd, queryId);

            CleanupStageFile(conn, stagePath, destFileName);
        }
        finally
        {
            stream.Dispose();
            DropStageAndTable(conn, schemaName, stageName, tableName);
        }
    }

    [SnowflakeFact]
    public void ShouldThrowWhenStreamIsNull()
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output);
        conn.Open();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "PUT file://null.csv @~ AUTO_COMPRESS=FALSE";
        Assert.Throws<ArgumentNullException>(() => ExecutePutFromMemory(cmd, null));
    }

    [SnowflakeFact]
    public void ShouldThrowWhenStreamIsDisposed()
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();

        var stream = new MemoryStream([1, 2, 3]);
        stream.Dispose();

        using var conn = Fixture.Factory.Create(Output);
        conn.Open();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "PUT file://disposed.csv @~ AUTO_COMPRESS=FALSE";
        Assert.Throws<ObjectDisposedException>(() => ExecutePutFromMemory(cmd, stream));
    }

    [SnowflakeFact]
    public void ShouldThrowWhenStreamIsAtNonZeroPosition()
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();

        using var stream = new MemoryStream([1, 2, 3]);
        stream.Position = 2;

        using var conn = Fixture.Factory.Create(Output);
        conn.Open();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "PUT file://offset.csv @~ AUTO_COMPRESS=FALSE";
        Assert.Throws<InvalidOperationException>(() => ExecutePutFromMemory(cmd, stream));
    }

    [SnowflakeFact]
    public void ShouldThrowWithoutQueryIdWhenConnectionIsClosed()
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();

        using var stream = new MemoryStream([1, 2, 3]);
        using var conn = Fixture.Factory.Create(Output);
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "PUT file://closed.csv @~ AUTO_COMPRESS=FALSE";

        var ex = Assert.ThrowsAny<DbException>(() => ExecutePutFromMemory(cmd, stream));
        AssertClosedConnectionWithoutQueryId(ex);
    }

    [SnowflakeFact]
    public async Task ShouldThrowWithQueryIdWhenSqlSyntaxIsInvalidAsync()
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();

        using var stream = new MemoryStream([1, 2, 3]);
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "PUT SOME CODE FORCING SYNTAX ERROR";

        var ex = Assert.ThrowsAny<DbException>(() => ExecutePutFromMemory(cmd, stream));
        AssertSyntaxErrorWithQueryId(ex);
    }

    [SnowflakeFact]
    public async Task ShouldIgnoreStreamForNonPutCommandAsync()
    {
        // TODO(SNOW-4429445): in-memory PUT
        Skip.FutureMilestone();

        using var stream = new MemoryStream("this should be ignored"u8.ToArray());
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 1 AS result";
        using var reader = ExecutePutFromMemory(cmd, stream);
        Assert.True(reader.Read());
        Assert.Equal("1", reader.GetString(0));
    }

    private async Task PutFromMemoryAndVerifyRoundTripAsync(
        byte[] data,
        StageType stageType,
        bool autoCompress,
        bool expectCsvRoundTrip,
        int expectedFilesCount,
        CancellationToken cancellationToken)
    {
        var tableName = UniqueName("PUT_MEM");
        var stageName = UniqueName("STAGE_MEM");
        var stream = new MemoryStream(data);
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var schemaName = CurrentSchema(conn);
        try
        {
            CreateTableAndStage(conn, schemaName, tableName, stageName);
            var stagePath = StagePath(stageType, schemaName, tableName, stageName);
            var fileName = $"{Guid.NewGuid():N}.csv";
            var autoCompressFlag = autoCompress ? "TRUE" : "FALSE";

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"PUT file://{fileName} {stagePath} AUTO_COMPRESS={autoCompressFlag} OVERWRITE=TRUE";
            using var reader = ExecutePutFromMemory(cmd, stream);
            Assert.True(reader.Read());
            Assert.Equal("UPLOADED", reader.GetString(ResultStatusOrdinal));
            var errorDetails = reader.GetString(ErrorDetailsOrdinal) ?? "";
            Assert.Empty(errorDetails);

            var destFileName = reader.GetString(DestinationFileNameOrdinal);
            var queryId = ReaderQueryId(reader, cmd);
            Assert.NotNull(queryId);
            Guid.Parse(queryId);
            Assert.Equal(queryId, CommandQueryId(cmd));

            switch (autoCompress)
            {
                case true when data.Length > 0:
                    Assert.Equal("none", reader.GetString(SourceCompressionTypeOrdinal));
                    Assert.Equal("gzip", reader.GetString(DestinationCompressionTypeOrdinal));
                    break;
                case false:
                    Assert.Equal("none", reader.GetString(SourceCompressionTypeOrdinal));
                    Assert.Equal("none", reader.GetString(DestinationCompressionTypeOrdinal));
                    break;
            }

            VerifyFilesAreUploaded(conn, stagePath, expectedFilesCount);

            if (expectCsvRoundTrip && data.Length > 0)
            {
                var expectedRows = Encoding.UTF8.GetString(data).Split('\n').Count(l => !string.IsNullOrEmpty(l));
                CopyIntoAndVerify(conn, schemaName, tableName, stageType, stagePath, destFileName, expectedRows);
            }

            CleanupStageFile(conn, stagePath, destFileName);
        }
        finally
        {
            stream.Dispose();
            DropStageAndTable(conn, schemaName, stageName, tableName);
        }
    }

    private async Task PutFromMemoryAndVerifyUploadAsync(
        byte[] data,
        StageType stageType,
        bool autoCompress,
        string expectedSourceCompression,
        CancellationToken cancellationToken)
    {
        var tableName = UniqueName("PUT_MEM");
        var stageName = UniqueName("STAGE_MEM");
        var stream = new MemoryStream(data);
        using var conn = await OpenConnectionAsync(cancellationToken).ConfigureAwait(false);
        var schemaName = CurrentSchema(conn);
        try
        {
            CreateTableAndStage(conn, schemaName, tableName, stageName);
            var stagePath = StagePath(stageType, schemaName, tableName, stageName);
            var fileName = $"{Guid.NewGuid():N}.csv";
            var autoCompressFlag = autoCompress ? "TRUE" : "FALSE";

            string destFileName;
            using (var cmd = conn.CreateCommand())
            {
                cmd.CommandText = $"PUT file://{fileName} {stagePath} AUTO_COMPRESS={autoCompressFlag} OVERWRITE=TRUE";
                using var reader = ExecutePutFromMemory(cmd, stream);
                Assert.True(reader.Read());
                Assert.Equal("UPLOADED", reader.GetString(ResultStatusOrdinal));
                Assert.Empty(reader.GetString(ErrorDetailsOrdinal) ?? "");
                destFileName = reader.GetString(DestinationFileNameOrdinal);
                Assert.Equal(expectedSourceCompression, reader.GetString(SourceCompressionTypeOrdinal));
            }

            CleanupStageFile(conn, stagePath, destFileName);
        }
        finally
        {
            stream.Dispose();
            DropStageAndTable(conn, schemaName, stageName, tableName);
        }
    }

    private static void CopyIntoAndVerify(
        ISnowflakeDbConnection conn,
        string schemaName,
        string tableName,
        StageType stageType,
        string stagePath,
        string destFileName,
        int expectedRows)
    {
        using var command = conn.CreateCommand();
        command.CommandText = stageType == StageType.TABLE
            ? $"COPY INTO {schemaName}.{tableName}"
            : $"COPY INTO {schemaName}.{tableName} FROM {stagePath}/{destFileName}";
        command.ExecuteNonQuery();

        command.CommandText = $"SELECT COUNT(*) FROM {schemaName}.{tableName}";
        var count = (long)command.ExecuteScalar()!;
        Assert.Equal(expectedRows, count);

        command.CommandText = $"SELECT * FROM {schemaName}.{tableName} LIMIT 1";
        using var reader = command.ExecuteReader();
        Assert.True(reader.Read());
        for (var i = 0; i < s_colData.Length; i++)
            Assert.Equal(s_colData[i], reader.GetString(i));
    }

    private static void VerifyFilesAreUploaded(ISnowflakeDbConnection conn, string stage, int expectedFilesCount)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = $"LIST {stage}";
        using var reader = cmd.ExecuteReader();
        var rowCount = 0;
        while (reader.Read())
            rowCount++;
        Assert.Equal(expectedFilesCount, rowCount);
    }

    private static void CleanupStageFile(ISnowflakeDbConnection conn, string stagePath, string destFileName)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = $"REMOVE {stagePath}/{destFileName}";
        try
        {
            cmd.ExecuteNonQuery();
        }
        catch (Exception)
        {
        }
    }

    private async Task<ISnowflakeDbConnection> OpenConnectionAsync(CancellationToken cancellationToken)
    {
        var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        return conn;
    }

    private static string UniqueName(string prefix) => $"{prefix}_{Guid.NewGuid():N}";

    private static string CurrentSchema(ISnowflakeDbConnection conn)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select current_schema()";
        return Convert.ToString(cmd.ExecuteScalar(), CultureInfo.InvariantCulture)!;
    }

    private static void CreateTableAndStage(
        ISnowflakeDbConnection conn,
        string schemaName,
        string tableName,
        string stageName)
    {
        using var command = conn.CreateCommand();
        command.CommandText = $"CREATE OR REPLACE TABLE {schemaName}.{tableName} (C1 STRING, C2 STRING, C3 STRING)";
        command.ExecuteNonQuery();
        command.CommandText = $"CREATE OR REPLACE STAGE {schemaName}.{stageName}";
        command.ExecuteNonQuery();
    }

    private static void DropStageAndTable(
        ISnowflakeDbConnection conn,
        string schemaName,
        string stageName,
        string tableName)
    {
        using var command = conn.CreateCommand();
        try
        {
            command.CommandText = $"DROP STAGE IF EXISTS {schemaName}.{stageName}";
            command.ExecuteNonQuery();
        }
        catch (Exception)
        {
        }

        try
        {
            command.CommandText = $"DROP TABLE IF EXISTS {schemaName}.{tableName}";
            command.ExecuteNonQuery();
        }
        catch (Exception)
        {
        }
    }

    private static string StagePath(StageType stageType, string schemaName, string tableName, string stageName) =>
        stageType switch
        {
            StageType.TABLE => $"@{schemaName}.%{tableName}",
            StageType.NAMED => $"@{schemaName}.{stageName}",
            _ => throw new ArgumentOutOfRangeException(nameof(stageType))
        };

    private static byte[] MakeCsvBytes(int rows = 4)
    {
        var row = $"{string.Join(",", s_colData)}\n";
        return Encoding.UTF8.GetBytes(string.Concat(Enumerable.Repeat(row, rows)));
    }

    private static byte[] MakeGzipBytes(byte[] raw)
    {
        using var output = new MemoryStream();
        using (var gzip = new GZipStream(output, CompressionMode.Compress, leaveOpen: true))
            gzip.Write(raw, 0, raw.Length);
        return output.ToArray();
    }

    private static DbDataReader ExecutePutFromMemory(IDbCommand cmd, MemoryStream? stream)
    {
        if (cmd is OldSnowflakeDbCommand oldCmd)
            return oldCmd.ExecuteDbDataReaderWithMemoryStream(stream!);
        return ((NewSnowflakeDbCommand)cmd).ExecuteDbDataReaderWithMemoryStream(stream!);
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

    private static void AssertQuerySuccess(IDbCommand cmd, string queryId)
    {
        if (cmd is OldSnowflakeDbCommand oldCmd)
        {
            Assert.Equal(OldQueryStatus.Success, oldCmd.GetQueryStatus(queryId));
            return;
        }

        Assert.Equal(NewQueryStatus.Success, ((NewSnowflakeDbCommand)cmd).GetQueryStatus(queryId));
    }

    private static void AssertClosedConnectionWithoutQueryId(Exception ex)
    {
        switch (ex)
        {
            case OldSnowflakeDbException old:
                Assert.Null(old.QueryId);
                Assert.Equal(ClosedConnectionErrorCode, old.ErrorCode);
                Assert.Null(old.SqlState);
                break;
            case NewSnowflakeDbException neu:
                Assert.Null(neu.QueryId);
                Assert.Equal(ClosedConnectionErrorCode, neu.ErrorCode);
                Assert.Null(neu.SqlState);
                break;
            default:
                Assert.Fail($"Unexpected exception type {ex.GetType()}: {ex.Message}");
                break;
        }
    }

    private static void AssertSyntaxErrorWithQueryId(Exception ex)
    {
        switch (ex)
        {
            case OldSnowflakeDbException old:
                Assert.NotNull(old.QueryId);
                Guid.Parse(old.QueryId);
                Assert.Equal(SyntaxErrorCode, old.ErrorCode);
                Assert.Equal(SyntaxErrorSqlState, old.SqlState);
                break;
            case NewSnowflakeDbException neu:
                Assert.NotNull(neu.QueryId);
                Guid.Parse(neu.QueryId);
                Assert.Equal(SyntaxErrorCode, neu.ErrorCode);
                Assert.Equal(SyntaxErrorSqlState, neu.SqlState);
                break;
            default:
                Assert.Fail($"Unexpected exception type {ex.GetType()}: {ex.Message}");
                break;
        }
    }
}
