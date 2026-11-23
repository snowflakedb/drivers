extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using System.Reflection;
using System.Text;
using Newtonsoft.Json;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSFDataType = OldDriver::Snowflake.Data.Core.SFDataType;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbParameter = OldDriver::Snowflake.Data.Client.SnowflakeDbParameter;

namespace Snowflake.Data.Tests.E2E;

[Collection(nameof(IsolatedCollection))]
[Trait("Category", "Conversion")]
public sealed class SFBindTest : LiveE2ETestBase
{
    private const string TablePrefix = "DOTNET_TEST_SFBIND";
    private const int UnsupportedSnowflakeTypeErrorCode = 270054;
    private const string FormatYmd = "yyyy/MM/dd";
    private const string FormatHms = "HH\\:mm\\:ss";
    private const string FormatHmsf = "HH\\:mm\\:ss\\.fff";
    private const string FormatYmdHms = "yyyy/MM/dd HH\\:mm\\:ss";
    private const string FormatYmdHmsZ = "yyyy/MM/dd HH\\:mm\\:ss zzz";

    public SFBindTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldInsertRowsUsingArrayBindingAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola INTEGER", "colb STRING");

            using var cmd = CreateCommand(conn, $"insert into {tableName} values (?, ?)");
            AddParameter(cmd, "1", DbType.Int16, new[] { 1, 2, 3 });
            AddParameter(cmd, "2", DbType.String, new[] { "str1", "str2", "str3" });

            var count = await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(3, count);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeTheory(retriesCount: RetriesCount.Thrice)]
    [InlineData(true)]
    [InlineData(false)]
    public async Task ShouldBindNullValueAsync(bool bindDbNull)
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        object? nullValue = bindDbNull ? DBNull.Value : null;
        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(
                conn,
                tableName,
                "intData NUMBER",
                "fixedNumericData NUMBER(10,1)",
                "floatingNumData DOUBLE",
                "stringData VARCHAR",
                "binaryData BINARY",
                "boolData BOOLEAN",
                "dateData DATE",
                "timeData TIME",
                "dateTimeData DATETIME",
                "dateTimeWithTimeZone TIMESTAMP_TZ");
            foreach (DbType type in Enum.GetValues(typeof(DbType)))
            {
                var isTypeSupported = true;
                string colName;
                using (var command = conn.CreateCommand())
                {
                    var param = command.CreateParameter();
                    param.ParameterName = "p0";
                    param.DbType = type;

                    switch (type)
                    {
                        case DbType.SByte:
                        case DbType.Int16:
                        case DbType.Int32:
                        case DbType.Int64:
                        case DbType.Byte:
                        case DbType.UInt16:
                        case DbType.UInt32:
                        case DbType.UInt64:
                            colName = "intData";
                            break;
                        case DbType.Decimal:
                        case DbType.VarNumeric:
                            colName = "fixedNumericData";
                            break;
                        case DbType.Boolean:
                            colName = "boolData";
                            break;
                        case DbType.Double:
                        case DbType.Single:
                            colName = "floatingNumData";
                            break;
                        case DbType.Guid:
                        case DbType.AnsiString:
                        case DbType.String:
                        case DbType.StringFixedLength:
                            colName = "stringData";
                            break;
                        case DbType.Date:
                            colName = "dateData";
                            break;
                        case DbType.Time:
                            colName = "timeData";
                            break;
                        case DbType.DateTime:
                        case DbType.DateTime2:
                            colName = "dateTimeData";
                            break;
                        case DbType.DateTimeOffset:
                            colName = "dateTimeWithTimeZone";
                            break;
                        case DbType.Binary:
                            colName = "binaryData";
                            break;
                        default:
                            colName = "stringData";
                            isTypeSupported = false;
                            break;
                    }

                    if (isTypeSupported)
                    {
                        command.CommandText = $"insert into {tableName}({colName}) values(:p0)";
                        param.Value = nullValue;
                        command.Parameters.Add(param);
                        var rowsInserted = await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                        Assert.Equal(1, rowsInserted);
                    }
                    else
                    {
                        try
                        {
                            command.CommandText = $"insert into {tableName}(stringData) values(:p0)";
                            param.Value = nullValue;
                            command.Parameters.Add(param);
                            await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                        }
                        catch (DbException e) when (IsSnowflakeDbException(e))
                        {
                            Assert.Equal(270053, e.ErrorCode);
                        }
                    }
                }

                if (isTypeSupported)
                {
                    using var reader = ExecuteReader(conn, $"select {colName} from {tableName};");
                    reader.Read();
                    Assert.True(reader.IsDBNull(0));
                }

                ExecuteNonQuery(conn, $"DELETE FROM {tableName}");
            }
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldBindValueAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(
                conn,
                tableName,
                "intData NUMBER",
                "fixedNumericData NUMBER(10,1)",
                "floatingNumData DOUBLE",
                "stringData VARCHAR",
                "binaryData BINARY",
                "boolData BOOLEAN",
                "dateData DATE",
                "timeData TIME",
                "dateTimeData DATETIME",
                "dateTimeWithTimeZone TIMESTAMP_TZ");

            foreach (DbType type in Enum.GetValues(typeof(DbType)))
            {
                var isTypeSupported = true;
                string colName;
                using (var command = conn.CreateCommand())
                {
                    var param = command.CreateParameter();
                    param.ParameterName = "p0";
                    param.DbType = type;

                    switch (type)
                    {
                        case DbType.SByte:
                        case DbType.Byte:
                            colName = "intData";
                            param.Value = 1;
                            break;
                        case DbType.Int16:
                        case DbType.Int32:
                        case DbType.Int64:
                        case DbType.UInt16:
                        case DbType.UInt32:
                        case DbType.UInt64:
                            colName = "intData";
                            param.Value = 10;
                            break;
                        case DbType.Decimal:
                        case DbType.VarNumeric:
                            colName = "fixedNumericData";
                            param.Value = 10.1;
                            break;
                        case DbType.Boolean:
                            colName = "boolData";
                            param.Value = true;
                            break;
                        case DbType.Double:
                        case DbType.Single:
                            colName = "floatingNumData";
                            param.Value = 2.5;
                            break;
                        case DbType.Guid:
                        case DbType.AnsiString:
                        case DbType.String:
                        case DbType.StringFixedLength:
                            colName = "stringData";
                            param.Value = "thisIsAString";
                            break;
                        case DbType.Date:
                            colName = "dateData";
                            param.Value = DateTime.Now;
                            break;
                        case DbType.Time:
                            colName = "timeData";
                            param.Value = DateTime.Now;
                            break;
                        case DbType.DateTime:
                        case DbType.DateTime2:
                            colName = "dateTimeData";
                            param.Value = DateTime.Now;
                            break;
                        case DbType.DateTimeOffset:
                            colName = "dateTimeWithTimeZone";
                            param.Value = DateTimeOffset.Now;
                            break;
                        case DbType.Binary:
                            colName = "binaryData";
                            param.Value = Encoding.UTF8.GetBytes("BinaryData");
                            break;
                        default:
                            colName = "stringData";
                            isTypeSupported = false;
                            break;
                    }

                    if (isTypeSupported)
                    {
                        command.CommandText = $"insert into {tableName}({colName}) values(:p0)";
                        command.Parameters.Add(param);
                        var rowsInserted = await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                        Assert.Equal(1, rowsInserted);
                    }
                    else
                    {
                        try
                        {
                            command.CommandText = $"insert into {tableName}(stringData) values(:p0)";
                            param.Value = DBNull.Value;
                            command.Parameters.Add(param);
                            await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                        }
                        catch (DbException e) when (IsSnowflakeDbException(e))
                        {
                            Assert.Equal(270053, e.ErrorCode);
                        }
                    }
                }

                if (isTypeSupported)
                {
                    using var command = CreateCommand(conn, $"select {colName} from {tableName};");
                    using var reader = await command.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                    reader.Read();
                    Assert.False(reader.IsDBNull(0));
                }

                ExecuteNonQuery(conn, $"DELETE FROM {tableName}");
            }
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldBindValueWithSFDataTypeAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
        foreach (var type in SFDataTypeNames())
        {
            if (type is "None" or "MAP" or "VECTOR")
                continue;

            var isTypeSupported = true;
            var columns = type != "FIXED"
                ? new[] { $"data {type}", "unsupportedType VARCHAR" }
                : new[] { "data NUMBER", "unsupportedType VARCHAR" };

            var tableName = UniqueTableName();
            try
            {
                CreateOrReplaceTable(conn, tableName, columns);

                using (var command = conn.CreateCommand())
                {
                    var param = command.CreateParameter();
                    param.ParameterName = "p0";
                    SetSFDataType(param, type);
                    switch (type)
                    {
                        case "BINARY":
                            param.Value = Encoding.UTF8.GetBytes("BinaryData");
                            break;
                        case "FIXED":
                            param.Value = 10;
                            break;
                        case "BOOLEAN":
                            param.Value = true;
                            break;
                        case "DATE":
                            param.Value = DateTime.Now;
                            break;
                        case "TEXT":
                            param.Value = "thisIsAString";
                            break;
                        case "TIMESTAMP_LTZ":
                            param.Value = DateTimeOffset.Now;
                            break;
                        case "TIMESTAMP_NTZ":
                            param.Value = DateTime.Now;
                            break;
                        case "TIMESTAMP_TZ":
                            param.Value = DateTimeOffset.Now;
                            break;
                        case "TIME":
                            param.Value = DateTime.Now;
                            break;
                        case "REAL":
                            param.Value = 25.3;
                            break;
                        default:
                            isTypeSupported = false;
                            param.Value = "InvalidSFDataType";
                            break;
                    }

                    if (isTypeSupported)
                    {
                        // DbType.Object is unsupported, so the insert succeeds only when SFDataType takes precedence.
                        param.DbType = DbType.Object;
                        command.CommandText = $"insert into {tableName}(data) values(:p0)";
                        command.Parameters.Add(param);
                        var rowsInserted = await command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                        Assert.Equal(1, rowsInserted);
                    }
                    // The server rejects the query when the parameter type is VARIANT, OBJECT or ARRAY.
                    else if (type is not ("VARIANT" or "OBJECT" or "ARRAY"))
                    {
                        command.CommandText = $"insert into {tableName}(unsupportedType) values(:p0)";
                        command.Parameters.Add(param);
                        var thrown = await Assert.ThrowsAnyAsync<DbException>(
                            () => command.ExecuteNonQueryAsync(TestContext.Current.CancellationToken)).ConfigureAwait(false);
                        AssertSnowflakeDbException(thrown, UnsupportedSnowflakeTypeErrorCode);
                    }
                }

                if (isTypeSupported)
                {
                    using var reader = ExecuteReader(conn, $"select data from {tableName};");
                    reader.Read();
                    Assert.False(reader.IsDBNull(0));
                }
            }
            finally
            {
                DropTableIfExists(conn, tableName);
            }
        }
    }

    [SnowflakeFact]
    public async Task ShouldManageParameterCollectionAsync()
    {
        Skip.OnNewDriver(
            "BD#parameter-collection-copy-to-implemented",
            "the new driver implements CopyTo and throws ArgumentException for an index past the array end");

        using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        AssertParameterCollectionOperations(
            cmd,
            (collection, parameters) => Assert.Throws<NotImplementedException>(() => collection.CopyTo(parameters, 5)));
    }

    [SnowflakeFact]
    public async Task ShouldManageParameterCollectionAndRejectOutOfRangeCopyToAsync()
    {
        Skip.OnOldDriver(
            "BD#parameter-collection-copy-to-implemented",
            "the old driver throws NotImplementedException from CopyTo for any arguments");

        using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
        using var cmd = conn.CreateCommand();
        AssertParameterCollectionOperations(
            cmd,
            (collection, parameters) => Assert.Throws<ArgumentException>(() => collection.CopyTo(parameters, 5)));
    }

    [SnowflakeFact(retriesCount: RetriesCount.Thrice)]
    public async Task ShouldInsertRowsUsingStageArrayBindingAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(
                conn,
                tableName,
                "cola INTEGER",
                "colb STRING",
                "colc DATE",
                "cold TIME",
                "cole TIMESTAMP_NTZ",
                "colf TIMESTAMP_TZ");

            using var cmd = CreateCommand(conn, $"insert into {tableName} values (?, ?, ?, ?, ?, ?)");
            const int total = 250000;

            var arrint = new List<int>();
            for (var i = 0; i < total; i++)
            {
                arrint.Add(i * 10 + 1);
                arrint.Add(i * 10 + 2);
                arrint.Add(i * 10 + 3);
            }

            AddParameter(cmd, "1", DbType.Int16, arrint.ToArray());

            var arrstring = new List<string>();
            for (var i = 0; i < total; i++)
            {
                arrstring.Add("str1");
                arrstring.Add("str2");
                arrstring.Add("str3");
            }

            AddParameter(cmd, "2", DbType.String, arrstring.ToArray());

            var date1 = DateTime.ParseExact("2000-01-01 00:00:00.0000000", "yyyy-MM-dd HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            var date2 = DateTime.ParseExact("2020-05-11 23:59:59.9999999", "yyyy-MM-dd HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            var date3 = DateTime.ParseExact("2021-07-22 23:59:59.9999999", "yyyy-MM-dd HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            var arrDate = new List<DateTime>();
            for (var i = 0; i < total; i++)
            {
                arrDate.Add(date1);
                arrDate.Add(date2);
                arrDate.Add(date3);
            }

            AddParameter(cmd, "3", DbType.Date, arrDate.ToArray());

            var time1 = DateTime.ParseExact("00:00:00.0000000", "HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            var time2 = DateTime.ParseExact("23:59:59.9999999", "HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            var time3 = DateTime.ParseExact("12:35:41.3333333", "HH:mm:ss.fffffff", CultureInfo.InvariantCulture);
            var arrTime = new List<DateTime>();
            for (var i = 0; i < total; i++)
            {
                arrTime.Add(time1);
                arrTime.Add(time2);
                arrTime.Add(time3);
            }

            AddParameter(cmd, "4", DbType.Time, arrTime.ToArray());

            var ntz1 = DateTime.ParseExact("2017-01-01 12:00:00", "yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture);
            var ntz2 = DateTime.ParseExact("2020-12-31 23:59:59", "yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture);
            var ntz3 = DateTime.ParseExact("2022-04-01 00:00:00", "yyyy-MM-dd HH:mm:ss", CultureInfo.InvariantCulture);
            var arrNtz = new List<DateTime>();
            for (var i = 0; i < total; i++)
            {
                arrNtz.Add(ntz1);
                arrNtz.Add(ntz2);
                arrNtz.Add(ntz3);
            }

            AddParameter(cmd, "5", DbType.DateTime2, arrNtz.ToArray());

            var tz1 = DateTimeOffset.Now;
            var tz2 = DateTimeOffset.UtcNow;
            var tz3 = new DateTimeOffset(2007, 1, 1, 12, 0, 0, new TimeSpan(4, 0, 0));
            var arrTz = new List<DateTimeOffset>();
            for (var i = 0; i < total; i++)
            {
                arrTz.Add(tz1);
                arrTz.Add(tz2);
                arrTz.Add(tz3);
            }

            AddParameter(cmd, "6", DbType.DateTimeOffset, arrTz.ToArray());

            var count = await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(total * 3, count);

            cmd.Parameters.Clear();
            cmd.CommandText = $"SELECT * FROM {tableName}";
            using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.True(reader.Read());
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldBindArraysWhenJsonTypeNameHandlingIsAutoAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var restoreJsonDefaultSettings = SetJsonDefaultSettingsWithTypeNameHandlingAuto();

        var tableName = UniqueTableName();
        try
        {
            using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
            try
            {
                CreateOrReplaceTable(conn, tableName, "cola REAL", "colb TEXT", "colc NUMBER(38,0)");

                using var cmd = CreateCommand(conn, $"insert into {tableName} values (?, ?, ?)");
                const int total = 250;

                var arrdouble = new List<double>();
                var arrstring = new List<string>();
                var arrint = new List<int>();
                for (var i = 0; i < total; i++)
                {
                    arrdouble.Add(i * 10 + 1);
                    arrdouble.Add(i * 10 + 2);
                    arrdouble.Add(i * 10 + 3);

                    arrstring.Add("stra" + i.ToString(CultureInfo.InvariantCulture));
                    arrstring.Add("strb" + i.ToString(CultureInfo.InvariantCulture));
                    arrstring.Add("strc" + i.ToString(CultureInfo.InvariantCulture));

                    arrint.Add(i * 10 + 1);
                    arrint.Add(i * 10 + 2);
                    arrint.Add(i * 10 + 3);
                }

                AddParameter(cmd, "1", DbType.Double, arrdouble.ToArray());
                AddParameter(cmd, "2", DbType.String, arrstring.ToArray());
                AddParameter(cmd, "3", DbType.Int32, arrint.ToArray());

                var rowsCount = await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                Assert.Equal(total * 3, rowsCount);

                cmd.CommandText = $"SELECT * FROM {tableName}";
                using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
                Assert.True(reader.Read());
            }
            finally
            {
                DropTableIfExists(conn, tableName);
            }
        }
        finally
        {
            restoreJsonDefaultSettings.Dispose();
        }
    }

    [SnowflakeFact]
    public async Task ShouldInsertIntegersUsingStageArrayBindingAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola INTEGER");

            using var cmd = CreateCommand(conn, $"insert into {tableName} values (?)");
            const int total = 70000;

            var arrint = new List<int>();
            for (var i = 0; i < total; i++)
                arrint.Add(i);

            AddParameter(cmd, "1", DbType.Int16, arrint.ToArray());

            var count = await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(70000, count);

            cmd.CommandText = $"SELECT * FROM {tableName}";
            using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.True(reader.Read());
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldBindSimpleValueWithoutExplicitDbTypeAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola INTEGER");

            using var cmd = CreateCommand(conn, $"insert into {tableName} values (?)");
            var p1 = cmd.CreateParameter();
            p1.ParameterName = "1";
            p1.Value = 1;
            cmd.Parameters.Add(p1);

            var count = await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(1, count);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeFact]
    public async Task ShouldBindArrayValueWithoutExplicitDbTypeAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync(null, TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            CreateOrReplaceTable(conn, tableName, "cola INTEGER");

            using var cmd = CreateCommand(conn, $"insert into {tableName} values (?)");
            var p1 = cmd.CreateParameter();
            p1.ParameterName = "1";
            p1.Value = new[] { 1, 2, 3 };
            cmd.Parameters.Add(p1);

            var count = await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(3, count);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON", "DATE", null, DbType.Date, FormatYmd, null)]
    [InlineData("JSON", "TIME", null, DbType.Time, FormatHms, null)]
    [InlineData("JSON", "TIME", 6, DbType.Time, FormatHmsf, null)]
    [InlineData("JSON", "TIMESTAMP_NTZ", 6, DbType.DateTime, FormatYmdHms, null)]
    [InlineData("JSON", "TIMESTAMP_TZ", 6, DbType.DateTimeOffset, FormatYmdHmsZ, null)]
    [InlineData("JSON", "TIMESTAMP_LTZ", 6, DbType.DateTimeOffset, FormatYmdHmsZ, null)]
    [InlineData("ARROW", "DATE", null, DbType.Date, FormatYmd, null)]
    [InlineData("ARROW", "TIME", null, DbType.Time, FormatHms, null)]
    [InlineData("ARROW", "TIME", 6, DbType.Time, FormatHmsf, null)]
    [InlineData("ARROW", "TIMESTAMP_NTZ", 6, DbType.DateTime, FormatYmdHms, null)]
    [InlineData("ARROW", "TIMESTAMP_TZ", 6, DbType.DateTimeOffset, FormatYmdHmsZ, null)]
    [InlineData("ARROW", "TIMESTAMP_LTZ", 6, DbType.DateTimeOffset, FormatYmdHmsZ, null)]
    [InlineData("JSON", "TIMESTAMP_LTZ", 6, DbType.DateTimeOffset, FormatYmdHmsZ, "Europe/Warsaw")]
    [InlineData("JSON", "TIMESTAMP_LTZ", 6, DbType.DateTimeOffset, FormatYmdHmsZ, "Asia/Tokyo")]
    [InlineData("ARROW", "TIMESTAMP_LTZ", 6, DbType.DateTimeOffset, FormatYmdHmsZ, "Europe/Warsaw")]
    [InlineData("ARROW", "TIMESTAMP_LTZ", 6, DbType.DateTimeOffset, FormatYmdHmsZ, "Asia/Tokyo")]
    public async Task ShouldBindDateTimeAsync(
        string resultFormat,
        string columnType,
        int? columnPrecision,
        DbType bindingType,
        string comparisonFormat,
        string? timeZone)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        string[] timestamps =
        {
            "2023/03/15 13:17:29.207 +05:00",
            "9999/12/30 23:24:25.987 +07:00",
            "0001/01/02 02:06:07.000 -04:00",
        };
        var expected = ExpectedTimestamps.From(timestamps, columnType);
        var columnWithPrecision = columnPrecision != null ? $"{columnType}({columnPrecision})" : columnType;
        const int bindingThreshold = 65280; // when exceeded enforces bindings via file on stage
        const int smallBatchRowCount = 2;
        const int bigBatchRowCount = bindingThreshold / 2;

        var tableName = UniqueTableName();
        using var conn = await OpenConnectionAsync("poolingEnabled=false", TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            ExecuteNonQuery(conn, $"alter session set DOTNET_QUERY_RESULT_FORMAT = {resultFormat}");
            if (!string.IsNullOrEmpty(timeZone))
                ExecuteNonQuery(conn, $"alter session set TIMEZONE = '{timeZone}'");

            var columns = new List<string> { "id number(10,0) not null primary key" };
            var sqlColumns = "id";
            var sqlValues = "?";
            for (var i = 1; i <= timestamps.Length; ++i)
            {
                columns.Add($"ts_{i} {columnWithPrecision}");
                sqlColumns += $",ts_{i}";
                sqlValues += ",?";
            }

            CreateOrReplaceTable(conn, tableName, columns.ToArray());

            var sqlInsert = $"insert into {tableName} ({sqlColumns}) values ({sqlValues})";
            InsertSingleRecord(conn, sqlInsert, bindingType, 1, expected);
            InsertMultipleRecords(conn, sqlInsert, bindingType, 2, expected, smallBatchRowCount, false);
            InsertMultipleRecords(conn, sqlInsert, bindingType, smallBatchRowCount + 2, expected, bigBatchRowCount, true);

            var row = 0;
            using (var reader = ExecuteReader(conn, $"select {sqlColumns} from {tableName} order by id"))
            {
                while (reader.Read())
                {
                    ++row;
                    Assert.Equal(row, reader.GetInt32(0));

                    for (var i = 0; i < timestamps.Length; ++i)
                        expected.AssertEqual(reader.GetValue(i + 1), comparisonFormat, i);
                }
            }

            Assert.Equal(1 + smallBatchRowCount + bigBatchRowCount, row);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
        }
    }

    private static void InsertSingleRecord(
        ISnowflakeDbConnection conn,
        string sqlInsert,
        DbType binding,
        int identifier,
        ExpectedTimestamps ts)
    {
        using var insert = CreateCommand(conn, sqlInsert);
        AddParameter(insert, "1", DbType.Int32, identifier);
        if (ExpectedTimestamps.IsOffsetType(ts.ColumnType))
        {
            var dateTimeOffsets = ts.GetDateTimeOffsets();
            for (var i = 0; i < dateTimeOffsets.Length; ++i)
            {
                var parameter = AddParameter(insert, ParameterName(i + 2), binding, dateTimeOffsets[i]);
                SetSFDataType(parameter, ts.ColumnType);
            }
        }
        else
        {
            var dateTimes = ts.GetDateTimes();
            for (var i = 0; i < dateTimes.Length; ++i)
                AddParameter(insert, ParameterName(i + 2), binding, dateTimes[i]);
        }

        var rowsAffected = insert.ExecuteNonQuery();

        Assert.Equal(1, rowsAffected);
        Assert.Null(GetBindStage(insert));
    }

    private static void InsertMultipleRecords(
        ISnowflakeDbConnection conn,
        string sqlInsert,
        DbType binding,
        int initialIdentifier,
        ExpectedTimestamps ts,
        int rowsCount,
        bool shouldUseBinding)
    {
        using var insert = CreateCommand(conn, sqlInsert);
        AddParameter(insert, "1", DbType.Int32, Enumerable.Range(initialIdentifier, rowsCount).ToArray());
        if (ExpectedTimestamps.IsOffsetType(ts.ColumnType))
        {
            var dateTimeOffsets = ts.GetDateTimeOffsets();
            for (var i = 0; i < dateTimeOffsets.Length; ++i)
            {
                var parameterValue = Enumerable.Repeat(dateTimeOffsets[i], rowsCount).ToArray();
                var parameter = AddParameter(insert, ParameterName(i + 2), binding, parameterValue);
                SetSFDataType(parameter, ts.ColumnType);
            }
        }
        else
        {
            var dateTimes = ts.GetDateTimes();
            for (var i = 0; i < dateTimes.Length; ++i)
            {
                var parameterValue = Enumerable.Repeat(dateTimes[i], rowsCount).ToArray();
                AddParameter(insert, ParameterName(i + 2), binding, parameterValue);
            }
        }

        var rowsAffected = insert.ExecuteNonQuery();

        Assert.Equal(rowsCount, rowsAffected);
        if (shouldUseBinding)
            Assert.NotEmpty(GetBindStage(insert)!);
        else
            Assert.Null(GetBindStage(insert));
    }

    private async Task<ISnowflakeDbConnection> OpenConnectionAsync(
        string? connectionParameters,
        CancellationToken cancellationToken)
    {
        var conn = connectionParameters == null
            ? Fixture.Factory.Create(Output)
            : Fixture.Factory.Create(
                Output,
                $"{Fixture.Factory.Create(Output).ConnectionString.TrimEnd(';')};{connectionParameters}");
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        return conn;
    }

    private static IDisposable SetJsonDefaultSettingsWithTypeNameHandlingAuto()
    {
        var restorer = new JsonDefaultSettingsRestorer(JsonConvert.DefaultSettings);
        JsonConvert.DefaultSettings = CreateTypeNameHandlingAutoSettings;
        return restorer;
    }

    private static JsonSerializerSettings CreateTypeNameHandlingAutoSettings() =>
        new() { TypeNameHandling = TypeNameHandling.Auto };

    private static string UniqueTableName() => $"{TablePrefix}_{Guid.NewGuid():N}";

    private static string ParameterName(int index) => index.ToString(CultureInfo.InvariantCulture);

    private static void CreateOrReplaceTable(ISnowflakeDbConnection conn, string tableName, params string[] columns) =>
        ExecuteNonQuery(conn, $"CREATE OR REPLACE TABLE {tableName}({string.Join(", ", columns)})");

    private static void DropTableIfExists(ISnowflakeDbConnection conn, string tableName) =>
        ExecuteNonQuery(conn, $"DROP TABLE IF EXISTS {tableName}");

    private static DbCommand CreateCommand(ISnowflakeDbConnection connection, string commandText)
    {
        var command = connection.CreateCommand();
        command.CommandText = commandText;
        return command;
    }

    private static int ExecuteNonQuery(ISnowflakeDbConnection connection, string commandText)
    {
        using var command = CreateCommand(connection, commandText);
        return command.ExecuteNonQuery();
    }

    private static DbDataReader ExecuteReader(ISnowflakeDbConnection connection, string commandText)
    {
        var command = CreateCommand(connection, commandText);
        return command.ExecuteReader();
    }

    private static DbParameter AddParameter(DbCommand command, string name, DbType dbType, object? value)
    {
        var parameter = command.CreateParameter();
        parameter.ParameterName = name;
        parameter.DbType = dbType;
        parameter.Value = value;
        command.Parameters.Add(parameter);
        return parameter;
    }

    private static void AssertParameterCollectionOperations(
        DbCommand cmd,
        Action<DbParameterCollection, Array> assertCopyTo)
    {
        var p1 = cmd.CreateParameter();
        p1.ParameterName = "1";
        p1.DbType = DbType.Int16;
        p1.Value = 1;

        var p2 = cmd.CreateParameter();
        p2.ParameterName = "2";
        p1.DbType = DbType.Int16;
        p2.Value = 2;

        var p3 = cmd.CreateParameter();
        p2.ParameterName = "2";
        p1.DbType = DbType.Int16;
        p2.Value = 2;

        var parameters = Array.CreateInstance(typeof(IDbDataParameter), 3);
        parameters.SetValue(p1, 0);
        parameters.SetValue(p2, 1);
        parameters.SetValue(p3, 2);

        cmd.Parameters.AddRange(parameters);
        assertCopyTo(cmd.Parameters, parameters);

        Assert.Equal(3, cmd.Parameters.Count);
        Assert.True(cmd.Parameters.Contains(p2));
        Assert.True(cmd.Parameters.Contains("2"));
        Assert.Equal(1, cmd.Parameters.IndexOf(p2));
        Assert.Equal(1, cmd.Parameters.IndexOf("2"));

        cmd.Parameters.Remove(p2);
        Assert.Equal(2, cmd.Parameters.Count);
        Assert.Same(p1, cmd.Parameters[0]);

        cmd.Parameters.RemoveAt(0);
        Assert.Same(p3, cmd.Parameters[0]);

        cmd.Parameters.Clear();
        Assert.Empty(cmd.Parameters);
    }

    private static string[] SFDataTypeNames() => Enum.GetNames(typeof(OldSFDataType));

    private static void SetSFDataType(DbParameter parameter, string sfDataType)
    {
        if (parameter is OldSnowflakeDbParameter oldParameter)
            oldParameter.SFDataType = (OldSFDataType)Enum.Parse(typeof(OldSFDataType), sfDataType);
    }

    private static string? GetBindStage(DbCommand command)
    {
        var method = command.GetType().GetMethod(
            "GetBindStage",
            BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic);
        Assert.NotNull(method);
        return (string?)method.Invoke(command, null);
    }

    private static bool IsSnowflakeDbException(Exception exception) =>
        exception is OldSnowflakeDbException or NewSnowflakeDbException;

    private static void AssertSnowflakeDbException(DbException thrown, int errorCode)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
        {
            var old = Assert.IsType<OldSnowflakeDbException>(thrown);
            Assert.Equal(errorCode, old.ErrorCode);
        }
        else
        {
            var current = Assert.IsType<NewSnowflakeDbException>(thrown);
            Assert.Equal(errorCode, current.ErrorCode);
        }
    }

    [CollectionDefinition(nameof(IsolatedCollection), DisableParallelization = true)]
    public sealed class IsolatedCollection
    {
    }

    private sealed class JsonDefaultSettingsRestorer : IDisposable
    {
        private readonly object? _original;

        public JsonDefaultSettingsRestorer(object? original) => _original = original;

        public void Dispose() => JsonConvert.DefaultSettings = (Func<JsonSerializerSettings>?)_original;
    }

    private sealed class ExpectedTimestamps
    {
        private const string ParseFormat = "yyyy/MM/dd HH:mm:ss.fff zzz";
        private readonly DateTime[]? _expectedDateTimes;
        private readonly DateTimeOffset[]? _expectedDateTimeOffsets;

        private ExpectedTimestamps(string columnType, DateTime[]? dateTimes, DateTimeOffset[]? dateTimeOffsets)
        {
            ColumnType = columnType;
            _expectedDateTimes = dateTimes;
            _expectedDateTimeOffsets = dateTimeOffsets;
        }

        public string ColumnType { get; }

        public static ExpectedTimestamps From(string[] timestampsWithTimeZone, string columnType)
        {
            if (IsOffsetType(columnType))
            {
                var dateTimeOffsets = timestampsWithTimeZone
                    .Select(ts => DateTimeOffset.ParseExact(ts, ParseFormat, CultureInfo.InvariantCulture))
                    .ToArray();
                return new ExpectedTimestamps(columnType, null, dateTimeOffsets);
            }

            var dateTimes = timestampsWithTimeZone
                .Select(ts => DateTime.ParseExact(ts, ParseFormat, CultureInfo.InvariantCulture))
                .ToArray();
            return new ExpectedTimestamps(columnType, dateTimes, null);
        }

        public static bool IsOffsetType(string columnType) => columnType is "TIMESTAMP_LTZ" or "TIMESTAMP_TZ";

        public void AssertEqual(object actual, string comparisonFormat, int index)
        {
            switch (ColumnType)
            {
                case "TIMESTAMP_TZ":
                    Assert.Equal(
                        GetDateTimeOffsets()[index].ToString(comparisonFormat, CultureInfo.InvariantCulture),
                        ((DateTimeOffset)actual).ToString(comparisonFormat, CultureInfo.InvariantCulture));
                    break;
                case "TIMESTAMP_LTZ":
                    Assert.Equal(
                        GetDateTimeOffsets()[index].ToUniversalTime().ToString(comparisonFormat, CultureInfo.InvariantCulture),
                        ((DateTimeOffset)actual).ToUniversalTime().ToString(comparisonFormat, CultureInfo.InvariantCulture));
                    break;
                default:
                    Assert.Equal(
                        GetDateTimes()[index].ToString(comparisonFormat, CultureInfo.InvariantCulture),
                        ((DateTime)actual).ToString(comparisonFormat, CultureInfo.InvariantCulture));
                    break;
            }
        }

        public DateTime[] GetDateTimes()
        {
            Assert.NotNull(_expectedDateTimes);
            return _expectedDateTimes;
        }

        public DateTimeOffset[] GetDateTimeOffsets()
        {
            Assert.NotNull(_expectedDateTimeOffsets);
            return _expectedDateTimeOffsets;
        }
    }
}
