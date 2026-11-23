extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using OldSnowflakeDbDataReader = OldDriver::Snowflake.Data.Client.SnowflakeDbDataReader;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.E2E;

public sealed class VectorTypesTest : LiveE2ETestBase
{
    private const float LargeNegativeFloat = -1E+38f;
    private const float LargePositiveFloat = 1E+38f;
    private const int InvalidIdentifierErrorCode = 904;
    private const string InvalidIdentifierSqlState = "42000";
    private const int InvalidVectorValueErrorCode = 220000;
    private const string InvalidVectorValueSqlState = "22000";

    public VectorTypesTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectIntVectorFromTableAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382584): VECTOR column conversion (Arrow FixedSizeListArray) and SnowflakeDbDataReader.GetArray<T>
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var tableName = $"VECTOR_TYPES_{Guid.NewGuid():N}";
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        try
        {
            command.CommandText = $"CREATE OR REPLACE TABLE {tableName} (a VECTOR(INT, 3));";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
            command.CommandText = $"INSERT INTO {tableName} SELECT [1,2,3]::VECTOR(INT,3);";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
            command.CommandText = $"INSERT INTO {tableName} SELECT [4,5,6]::VECTOR(INT,3);";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
            command.CommandText = $"INSERT INTO {tableName} SELECT [7,8,9]::VECTOR(INT,3);";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

            command.CommandText = $"SELECT COUNT(*) FROM {tableName};";
            using (var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false))
            {
                Assert.True(reader.Read());
                Assert.Equal(3, reader.GetInt16(0));
            }

            command.CommandText = $"SELECT * FROM {tableName};";
            using (var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false))
            {
                Assert.True(reader.Read());
                Assert.Equal("[1,2,3]", reader.GetString(0));
                var arr = GetArray<int>(reader, 0);
                Assert.Equal(1, arr[0]);
                Assert.Equal(2, arr[1]);
                Assert.Equal(3, arr[2]);

                Assert.True(reader.Read());
                Assert.Equal("[4,5,6]", reader.GetString(0));
                arr = GetArray<int>(reader, 0);
                Assert.Equal(4, arr[0]);
                Assert.Equal(5, arr[1]);
                Assert.Equal(6, arr[2]);

                Assert.True(reader.Read());
                Assert.Equal("[7,8,9]", reader.GetString(0));
                arr = GetArray<int>(reader, 0);
                Assert.Equal(7, arr[0]);
                Assert.Equal(8, arr[1]);
                Assert.Equal(9, arr[2]);
            }
        }
        finally
        {
            command.CommandText = $"DROP TABLE IF EXISTS {tableName};";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectFloatVectorFromTableAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382584): VECTOR column conversion (Arrow FixedSizeListArray) and SnowflakeDbDataReader.GetArray<T>
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var tableName = $"VECTOR_TYPES_{Guid.NewGuid():N}";
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        try
        {
            command.CommandText = $"CREATE OR REPLACE TABLE {tableName} (a VECTOR(FLOAT, 3));";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
            command.CommandText = $"INSERT INTO {tableName} SELECT [1.1,2.2,3.3]::VECTOR(FLOAT,3);";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
            command.CommandText = $"INSERT INTO {tableName} SELECT [4.4,5.5,6.6]::VECTOR(FLOAT,3);";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
            command.CommandText = $"INSERT INTO {tableName} SELECT [7.7,8.8,9.9]::VECTOR(FLOAT,3);";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

            command.CommandText = $"SELECT COUNT(*) FROM {tableName};";
            using (var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false))
            {
                Assert.True(reader.Read());
                Assert.Equal(3, reader.GetInt16(0));
            }

            command.CommandText = $"SELECT * FROM {tableName};";
            using (var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false))
            {
                Assert.True(reader.Read());
                Assert.Equal("[1.100000,2.200000,3.300000]", reader.GetString(0));
                var arr = GetArray<float>(reader, 0);
                Assert.Equal(1.1f, arr[0]);
                Assert.Equal(2.2f, arr[1]);
                Assert.Equal(3.3f, arr[2]);

                Assert.True(reader.Read());
                Assert.Equal("[4.400000,5.500000,6.600000]", reader.GetString(0));
                arr = GetArray<float>(reader, 0);
                Assert.Equal(4.4f, arr[0]);
                Assert.Equal(5.5f, arr[1]);
                Assert.Equal(6.6f, arr[2]);

                Assert.True(reader.Read());
                Assert.Equal("[7.700000,8.800000,9.900000]", reader.GetString(0));
                arr = GetArray<float>(reader, 0);
                Assert.Equal(7.7f, arr[0]);
                Assert.Equal(8.8f, arr[1]);
                Assert.Equal(9.9f, arr[2]);
            }
        }
        finally
        {
            command.CommandText = $"DROP TABLE IF EXISTS {tableName};";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectIntVectorAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382584): VECTOR column conversion (Arrow FixedSizeListArray) and SnowflakeDbDataReader.GetArray<T>
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT [1, 2, 3]::VECTOR(INT, 3) as vec;";
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(reader.Read());
        Assert.Equal("[1,2,3]", reader.GetString(0));

        var arr = GetArray<int>(reader, 0);
        Assert.Equal(1, arr[0]);
        Assert.Equal(2, arr[1]);
        Assert.Equal(3, arr[2]);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectIntVectorWithMinAndMax32BitValuesAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382584): VECTOR column conversion (Arrow FixedSizeListArray) and SnowflakeDbDataReader.GetArray<T>
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = string.Format(
            CultureInfo.InvariantCulture,
            "SELECT [{0}, {1}]::VECTOR(INT, 2) as vec;",
            int.MinValue,
            int.MaxValue);
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(reader.Read());
        Assert.Equal(
            string.Format(CultureInfo.InvariantCulture, "[{0},{1}]", int.MinValue, int.MaxValue),
            reader.GetString(0));

        var arr = GetArray<int>(reader, 0);
        Assert.Equal(int.MinValue, arr[0]);
        Assert.Equal(int.MaxValue, arr[1]);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldThrowExceptionForInvalidValueForIntVectorAsync(string resultFormat)
    {
        Skip.When(IsJenkins(), "Test is skipped on Jenkins.");
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382591): SnowflakeDbException construction for server errors
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT [1.1]::VECTOR(INT, 3) as vec;";

        var thrown = await Assert.ThrowsAnyAsync<DbException>(() => command.ExecuteReaderAsync(ct)).ConfigureAwait(false);
        AssertOldSnowflakeDbException(thrown, InvalidVectorValueErrorCode, InvalidVectorValueSqlState);

        Assert.True(
            thrown.Message.Contains("Array-like value being cast to a vector has incorrect dimension")
            || thrown.Message.Contains(
                "Vector value being cast to a vector is not an array or vector, or has incorrect dimension or element type"),
            $"Unexpected message: {thrown.Message}");
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldThrowExceptionForInvalidIdentifierForIntVectorAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382591): SnowflakeDbException construction for server errors
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT [A, B, C]::VECTOR(INT, 3) as vec;";

        var thrown = await Assert.ThrowsAnyAsync<DbException>(() => command.ExecuteReaderAsync(ct)).ConfigureAwait(false);
        AssertOldSnowflakeDbException(thrown, InvalidIdentifierErrorCode, InvalidIdentifierSqlState);

        Assert.Contains("invalid identifier", thrown.Message);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectFloatVectorAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382584): VECTOR column conversion (Arrow FixedSizeListArray) and SnowflakeDbDataReader.GetArray<T>
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT [1.1,2.22,3.333]::VECTOR(FLOAT, 3) as vec;";
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(reader.Read());
        Assert.Equal("[1.100000,2.220000,3.333000]", reader.GetString(0));

        var arr = GetArray<float>(reader, 0);
        Assert.Equal(1.1f, arr[0]);
        Assert.Equal(2.22f, arr[1]);
        Assert.Equal(3.333f, arr[2]);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectFloatVectorWithLargeFloatValuesAsync(string resultFormat)
    {
        Skip.When(IsJenkins(), "Test is skipped on Jenkins.");
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382584): VECTOR column conversion (Arrow FixedSizeListArray) and SnowflakeDbDataReader.GetArray<T>
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = string.Format(
            CultureInfo.InvariantCulture,
            "SELECT [{0}, {1}]::VECTOR(FLOAT, 2) as vec;",
            LargeNegativeFloat,
            LargePositiveFloat);
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(reader.Read());

        var arr = GetArray<float>(reader, 0);

        if (!float.IsNegativeInfinity(arr[0]))
            Assert.InRange(arr[0], float.MinValue, -9E+37);

        if (!float.IsInfinity(arr[1]))
            Assert.InRange(arr[1], 9E+37, float.MaxValue);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectFloatVectorWithNoDecimalsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382584): VECTOR column conversion (Arrow FixedSizeListArray) and SnowflakeDbDataReader.GetArray<T>
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT [1,2,3]::VECTOR(FLOAT, 3) as vec;";
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(reader.Read());
        Assert.Equal("[1.000000,2.000000,3.000000]", reader.GetString(0));

        var arr = GetArray<float>(reader, 0);
        Assert.Equal(1f, arr[0]);
        Assert.Equal(2f, arr[1]);
        Assert.Equal(3f, arr[2]);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectFloatVectorWithGreaterThanSixDigitPrecisionAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382584): VECTOR column conversion (Arrow FixedSizeListArray) and SnowflakeDbDataReader.GetArray<T>
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT [1.123456789,2.123456789,3.123456789]::VECTOR(FLOAT, 3) as vec;";
        using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);

        Assert.True(reader.Read());
        Assert.Equal("[1.123457,2.123457,3.123457]", reader.GetString(0));

        var arr = GetArray<float>(reader, 0);
        Assert.Equal(1.123457f, arr[0]);
        Assert.Equal(2.123457f, arr[1]);
        Assert.Equal(3.123457f, arr[2]);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldThrowExceptionForInvalidIdentifierForFloatVectorAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382591): SnowflakeDbException construction for server errors
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        using var conn = await OpenConnectionAsync(resultFormat, ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = "SELECT [A, B, C]::VECTOR(FLOAT, 3) as vec;";

        var thrown = await Assert.ThrowsAnyAsync<DbException>(() => command.ExecuteReaderAsync(ct)).ConfigureAwait(false);
        AssertOldSnowflakeDbException(thrown, InvalidIdentifierErrorCode, InvalidIdentifierSqlState);

        Assert.Contains("invalid identifier", thrown.Message);
    }

    private async Task<ISnowflakeDbConnection> OpenConnectionAsync(string resultFormat, CancellationToken ct)
    {
        var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        command.CommandText = $"ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = {resultFormat}";
        await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        return conn;
    }

    private static bool IsJenkins() =>
        !string.IsNullOrEmpty(Environment.GetEnvironmentVariable("JENKINS_HOME"));

    private static T[] GetArray<T>(DbDataReader reader, int ordinal) =>
        reader is OldSnowflakeDbDataReader oldReader
            ? oldReader.GetArray<T>(ordinal)
            : throw new NotSupportedException($"{reader.GetType()} has no GetArray<T>");

    private static void AssertOldSnowflakeDbException(Exception exception, int errorCode, string sqlState)
    {
        var thrown = Assert.IsType<OldSnowflakeDbException>(exception);
        Assert.Equal(errorCode, thrown.ErrorCode);
        Assert.Equal(sqlState, thrown.SqlState);
    }
}
