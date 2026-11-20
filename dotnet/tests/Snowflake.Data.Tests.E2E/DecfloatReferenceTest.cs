extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using System.Reflection;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using OldSnowflakeDbConnection = OldDriver::Snowflake.Data.Client.SnowflakeDbConnection;

namespace Snowflake.Data.Tests.E2E;

[CollectionDefinition(nameof(DecfloatDriverVersionCollection), DisableParallelization = true)]
public sealed class DecfloatDriverVersionCollection
{
}

[Collection(nameof(DecfloatDriverVersionCollection))]
public sealed class DecfloatReferenceTest : LiveE2ETestBase, IDisposable
{
    private readonly object? _originalDriverVersion;

    public DecfloatReferenceTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
        _originalDriverVersion = TestDriver.Current == TestDriver.Kind.Old ? GetDriverVersionProperty().GetValue(null) : null;
    }

    public void Dispose()
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            GetDriverVersionProperty().SetValue(null, _originalDriverVersion);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectDecfloatLiteralAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): DECFLOAT GetDataTypeName/GetValue conversion
        Skip.FutureMilestone();

        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 123.456::DECFLOAT AS decfloat_value";

        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        ValidateResultFormat(reader, resultFormat);

        Assert.Equal("DECFLOAT", reader.GetDataTypeName(0));
        Assert.Equal(typeof(string), reader.GetFieldType(0));

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        var value = reader.GetValue(0);
        Assert.IsType<string>(value);
        Assert.Equal(123.456m, ParseDecfloatValue((string)value));

        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadHighPrecisionDecfloatAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): DECFLOAT GetValue conversion
        Skip.FutureMilestone();

        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 1234567890.123456789::DECFLOAT AS high_precision";

        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        ValidateResultFormat(reader, resultFormat);

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        var value = reader.GetValue(0);
        Assert.IsType<string>(value);
        Assert.Equal(1234567890.123456789m, ParseDecfloatValue((string)value));

        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadNullDecfloatAsync(string resultFormat)
    {
        Skip.FutureMilestone("TODO SNOW-4483010 next milestone");
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT NULL::DECFLOAT AS null_decfloat";

        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        ValidateResultFormat(reader, resultFormat);

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        Assert.True(reader.IsDBNull(0));
        Assert.Equal(DBNull.Value, reader.GetValue(0));

        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadNegativeDecfloatAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): DECFLOAT GetValue conversion
        Skip.FutureMilestone();

        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT -987.654::DECFLOAT AS negative_value";

        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        ValidateResultFormat(reader, resultFormat);

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        var value = reader.GetValue(0);
        Assert.IsType<string>(value);
        Assert.Equal(-987.654m, ParseDecfloatValue((string)value));

        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadZeroDecfloatAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): DECFLOAT GetValue conversion
        Skip.FutureMilestone();

        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 0::DECFLOAT AS zero_value";

        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        ValidateResultFormat(reader, resultFormat);

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        var value = reader.GetValue(0);
        Assert.IsType<string>(value);
        Assert.Equal(0m, ParseDecfloatValue((string)value));

        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadDecfloatWithLargePositiveExponentAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): DECFLOAT GetValue conversion
        Skip.FutureMilestone();

        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 1.23e10::DECFLOAT AS large_value";

        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        ValidateResultFormat(reader, resultFormat);

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        var value = reader.GetValue(0);
        Assert.IsType<string>(value);
        Assert.Equal(12300000000m, ParseDecfloatValue((string)value));

        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadDecfloatWithSmallNegativeExponentAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): DECFLOAT GetValue conversion
        Skip.FutureMilestone();

        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 1.5e-3::DECFLOAT AS small_value";

        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        ValidateResultFormat(reader, resultFormat);

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        var value = reader.GetValue(0);
        Assert.IsType<string>(value);
        Assert.Equal(0.0015m, ParseDecfloatValue((string)value));

        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadDecfloatColumnFromTableAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): DECFLOAT GetDataTypeName/GetValue conversion
        Skip.FutureMilestone();

        var tableName = $"DECFLOAT_{Guid.NewGuid():N}";
        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            SetResultFormat(conn, resultFormat);
            using (var create = conn.CreateCommand())
            {
                create.CommandText = $"CREATE OR REPLACE TABLE {tableName} (col_decfloat DECFLOAT)";
                await create.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            }

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"INSERT INTO {tableName} VALUES (123.456), (-999.999), (NULL)";
            await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            cmd.CommandText = $"SELECT col_decfloat FROM {tableName} ORDER BY col_decfloat NULLS LAST";

            using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            ValidateResultFormat(reader, resultFormat);

            Assert.Equal("DECFLOAT", reader.GetDataTypeName(0));

            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.Equal(-999.999m, ParseDecfloatValue((string)reader.GetValue(0)));

            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.Equal(123.456m, ParseDecfloatValue((string)reader.GetValue(0)));

            Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.True(reader.IsDBNull(0));

            Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        }
        finally
        {
            using var drop = conn.CreateCommand();
            drop.CommandText = $"DROP TABLE IF EXISTS {tableName}";
            await drop.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadMultipleDecfloatColumnsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): DECFLOAT GetDataTypeName/GetValue conversion
        Skip.FutureMilestone();

        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"SELECT
                        1.1::DECFLOAT AS col1,
                        2.2::DECFLOAT AS col2,
                        3.3::DECFLOAT AS col3";

        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        ValidateResultFormat(reader, resultFormat);

        Assert.Equal(3, reader.FieldCount);
        for (var i = 0; i < 3; i++)
        {
            Assert.Equal("DECFLOAT", reader.GetDataTypeName(i));
            Assert.Equal(typeof(string), reader.GetFieldType(i));
        }

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        Assert.Equal(1.1m, ParseDecfloatValue((string)reader.GetValue(0)));
        Assert.Equal(2.2m, ParseDecfloatValue((string)reader.GetValue(1)));
        Assert.Equal(3.3m, ParseDecfloatValue((string)reader.GetValue(2)));

        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadDecfloatWithCurrentDriverVersionAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): DECFLOAT GetValue conversion
        Skip.FutureMilestone();

        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 123.456::DECFLOAT AS decfloat_value";

        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        ValidateResultFormat(reader, resultFormat);

        var dataTypeName = reader.GetDataTypeName(0);
        var fieldType = reader.GetFieldType(0);

        Console.WriteLine($"Result format: {resultFormat}");
        Console.WriteLine($"DataTypeName: {dataTypeName}");
        Console.WriteLine($"FieldType: {fieldType}");

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        var value = reader.GetValue(0);
        Console.WriteLine($"Value type: {value?.GetType()?.Name ?? "null"}");
        Console.WriteLine($"Value: {value}");
        Assert.IsType<string>(value);
        Assert.Equal(123.456m, ParseDecfloatValue((string)value));

        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadDecfloatWithOlderDriverVersionAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4483010): DECFLOAT GetValue conversion
        Skip.FutureMilestone();

        var versionProp = GetDriverVersionProperty();
        versionProp.SetValue(null, "4.0.0");
        Console.WriteLine($"Driver version set to: {versionProp.GetValue(null)}");

        using var conn = await CreateAndOpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "SELECT 123.456::DECFLOAT AS decfloat_value";

        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        ValidateResultFormat(reader, resultFormat);

        var dataTypeName = reader.GetDataTypeName(0);
        var fieldType = reader.GetFieldType(0);

        Console.WriteLine($"Result format: {resultFormat}");
        Console.WriteLine($"DataTypeName: {dataTypeName}");
        Console.WriteLine($"FieldType: {fieldType}");

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        var value = reader.GetValue(0);
        Console.WriteLine($"Value type: {value?.GetType()?.Name ?? "null"}");
        Console.WriteLine($"Value: {value}");
        Assert.IsType<string>(value);
        Assert.Equal(123.456m, ParseDecfloatValue((string)value));

        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
    }

    private async Task<ISnowflakeDbConnection> CreateAndOpenConnectionAsync(CancellationToken ct)
    {
        var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(ct).ConfigureAwait(false);
        conn.PreventPooling();
        return conn;
    }

    private static void SetResultFormat(ISnowflakeDbConnection conn, string resultFormat)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = $"ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = '{resultFormat}'";
        cmd.ExecuteNonQuery();
    }

    private static void ValidateResultFormat(DbDataReader reader, string resultFormat)
    {
        var resultFormatProperty = reader.GetType().GetProperty(
            "ResultFormat",
            BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic);
        if (resultFormatProperty == null && TestDriver.Current == TestDriver.Kind.New)
            return;
        Assert.NotNull(resultFormatProperty);
        Assert.Equal(resultFormat, resultFormatProperty.GetValue(reader)?.ToString());
    }

    private static decimal ParseDecfloatValue(string value) =>
        decimal.Parse(value, NumberStyles.Float, CultureInfo.InvariantCulture);

    private static PropertyInfo GetDriverVersionProperty()
    {
        var envType = typeof(OldSnowflakeDbConnection).Assembly.GetType("Snowflake.Data.Core.SFEnvironment");
        Assert.NotNull(envType);
        var versionProp = envType.GetProperty(
            "DriverVersion",
            BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.Public);
        Assert.NotNull(versionProp);
        return versionProp;
    }
}
