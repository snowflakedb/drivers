extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using System.Text;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using OldSFDataType = OldDriver::Snowflake.Data.Core.SFDataType;
using OldSnowflakeDbParameter = OldDriver::Snowflake.Data.Client.SnowflakeDbParameter;

namespace Snowflake.Data.Tests.E2E;

public sealed class IcebergTableTest : LiveE2ETestBase
{
    private const string NotInCiScope = "Not a scope for CICD: requires external volume 'demo_exvol'";
    private const string SqlCreateIcebergTableColumns = @"nu1 number(10,0),
              nu2 number(19,0),
              nu3 number(18,2),
              nu4 number(38,0),
              f float,
              tx varchar(16777216),
              bt boolean,
              bf boolean,
              dt date,
              tm time,
              ntz timestamp_ntz(6),
              ltz timestamp_ltz(6),
              bi binary(5),
              ar array(number(10,0)),
              ob object(a number(10,0), b varchar),
              ma map(varchar, varchar)";
    private const string SqlCreateHybridTableColumns = @"id number(10,0) not null primary key,
              nu number(10,0),
              tx2 varchar(100)";
    private const string IcebergTableCreateFlags =
        "external_volume = 'demo_exvol' catalog = 'snowflake' base_location = 'x/'";
    private const string SqlColumnsSimpleTypes = "nu1,nu2,nu3,nu4,f,tx,bt,bf,dt,tm,ntz,ltz,bi";
    private const string SqlColumnsHybridTypes = "id,nu,tx2";
    private const string SqlColumnsStructureTypes = "ar,ob,ma";
    private const int I32 = 1;
    private const long I64 = 9223372036854775807;
    private const decimal Dec = (decimal)2.67;
    private const double Dbl = 3.333e8;
    private const float Flt = -1.0e7f;
    private const string Txt = "Sample text";
    private const bool B1 = true;
    private const bool B0 = false;
    private const int Id1 = 1;
    private const int Id2 = 2;
    private const string Txt1 = "sample text for join1";
    private const string Txt2 = "sample text for join2";
    private const string FormatYmd = "yyyy-MM-dd";
    private const string FormatHms = "HH:mm:ss";
    private const string FormatYmdHmsf = "yyyy-MM-dd HH:mm:ss.fffffff";
    private const string FormatYmdHmsfZ = "yyyy-MM-dd HH:mm:ss.fffffff zzz";

    private static readonly DateTime s_ts = DateTime.ParseExact(
        "2023/03/15 13:17:29.207",
        "yyyy/MM/dd HH:mm:ss.fff",
        CultureInfo.InvariantCulture);
    private readonly DateTime _dt = s_ts.Date;
    private readonly DateTime _tm = s_ts;
    private readonly DateTime _ntz = s_ts;
    private readonly DateTimeOffset _ltz = DateTimeOffset.ParseExact(
        "2023/03/15 13:17:29.207 +05:00",
        "yyyy/MM/dd HH:mm:ss.fff zzz",
        CultureInfo.InvariantCulture);
    private readonly byte[] _bi = Encoding.Default.GetBytes("flake");

    public IcebergTableTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public void ShouldInsertPlainText(string resultFormat)
    {
        Skip.When(true, NotInCiScope);
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4382582): GetBoolean/GetFloat on SnowflakeDbDataReader
        Skip.FutureMilestone();

        var icebergTable = UniqueTableName("DOTNET_TEST_DATA_IB");
        using var conn = OpenConnection();
        try
        {
            CreateIcebergTable(conn, icebergTable);
            SetResultFormat(conn, resultFormat);

            ExecuteNonQuery(
                conn,
                $@"insert into {icebergTable} ({SqlColumnsSimpleTypes})
                                                    values ({I32}, {I64}, {Dec.ToString(CultureInfo.InvariantCulture)}, {Dbl.ToString(CultureInfo.InvariantCulture)}, {Flt.ToString(CultureInfo.InvariantCulture)}, '{Txt}', {B1}, {B0},
                                                            '{_dt.ToString(FormatYmd, CultureInfo.InvariantCulture)}',
                                                            '{_tm.ToString(FormatHms, CultureInfo.InvariantCulture)}',
                                                            '{_ntz.ToString(FormatYmdHmsf, CultureInfo.InvariantCulture)}',
                                                            '{_ltz.ToString(FormatYmdHmsfZ, CultureInfo.InvariantCulture)}',
                                                            '{ByteArrayToHexString(_bi)}')");

            using var reader = ExecuteReader(conn, $"select {SqlColumnsSimpleTypes} from {icebergTable}");
            var rowsRead = 0;
            while (reader.Read())
            {
                rowsRead++;
                AssertRowValuesEqual(
                    reader,
                    SqlCreateIcebergTableColumns.Split('\n'),
                    I32,
                    I64,
                    Dec,
                    Dbl,
                    Flt,
                    Txt,
                    B1,
                    B0,
                    _dt,
                    _tm,
                    _ntz,
                    _ltz,
                    _bi);
            }

            Assert.Equal(1, rowsRead);
        }
        finally
        {
            DropTableIfExists(conn, icebergTable);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public void ShouldInsertWithValueBinding(string resultFormat)
    {
        Skip.When(true, NotInCiScope);
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var icebergTable = UniqueTableName("DOTNET_TEST_DATA_IB");
        using var conn = OpenConnection();
        try
        {
            CreateIcebergTable(conn, icebergTable);
            SetResultFormat(conn, resultFormat);

            InsertSingleRow(conn, icebergTable, I32, I64, Dec, Dbl, Flt, Txt, B1, B0, _dt, _tm, _ntz, _ltz, _bi);

            using var reader = ExecuteReader(conn, $"select {SqlColumnsSimpleTypes} from {icebergTable}");
            var rowsRead = 0;
            while (reader.Read())
            {
                rowsRead++;
                AssertRowValuesEqual(
                    reader,
                    SqlCreateIcebergTableColumns.Split('\n'),
                    I32,
                    I64,
                    Dec,
                    Dbl,
                    Flt,
                    Txt,
                    B1,
                    B0,
                    _dt,
                    _tm,
                    _ntz,
                    _ltz,
                    _bi);
            }

            Assert.Equal(1, rowsRead);
        }
        finally
        {
            DropTableIfExists(conn, icebergTable);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public void ShouldUpdateWithValueBinding(string resultFormat)
    {
        Skip.When(true, NotInCiScope);
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var i32 = I32 * 2;
        var i64 = I32;
        var dec = Dec + (decimal)0.1;
        var dbl = Dbl / 16;
        var flt = Flt * 2.5;
        var txt = Txt + " updated";
        var b1 = !B1;
        var b0 = !B0;
        var dt = _dt.Add(TimeSpan.FromDays(3));
        var tm = _tm.AddMinutes(7);
        var ntz = _ntz.Add(TimeSpan.FromDays(10));
        var ltz = _ltz.Subtract(TimeSpan.FromSeconds(37));
        var bi = Encoding.Default.GetBytes("Snow");

        var icebergTable = UniqueTableName("DOTNET_TEST_DATA_IB");
        using var conn = OpenConnection();
        try
        {
            CreateIcebergTable(conn, icebergTable);
            SetResultFormat(conn, resultFormat);
            InsertSingleRow(conn, icebergTable, I32, I64, Dec, Dbl, Flt, Txt, B1, B0, _dt, _tm, _ntz, _ltz, _bi);

            using (var cmd = CreateCommand(
                       conn,
                       $"update {icebergTable} set nu1=?,nu2=?,nu3=?,nu4=?,f=?,tx=?,bt=?,bf=?,dt=?,tm=?,ntz=?,ltz=?,bi=? where nu1=? and (bt=? or dt=?)"))
            {
                AddParameter(cmd, "1", DbType.Int32, i32);
                AddParameter(cmd, "2", DbType.Int64, i64);
                AddParameter(cmd, "3", DbType.Decimal, dec);
                AddParameter(cmd, "4", DbType.Double, dbl);
                AddParameter(cmd, "5", DbType.Double, flt);
                AddParameter(cmd, "6", DbType.String, txt);
                AddParameter(cmd, "7", DbType.Boolean, b1);
                AddParameter(cmd, "8", DbType.Boolean, b0);
                AddParameter(cmd, "9", DbType.Date, dt);
                AddParameter(cmd, "10", DbType.Time, tm);
                AddParameter(cmd, "11", DbType.DateTime, ntz);
                SetTimestampLtz(AddParameter(cmd, "12", DbType.DateTime, ltz));
                AddParameter(cmd, "13", DbType.Binary, bi);
                AddParameter(cmd, "14", DbType.Int32, I32);
                AddParameter(cmd, "15", DbType.Boolean, B1);
                AddParameter(cmd, "16", DbType.Date, _dt);
                Assert.Equal(1, cmd.ExecuteNonQuery());
            }

            using var reader = ExecuteReader(conn, $"select {SqlColumnsSimpleTypes} from {icebergTable}");
            var rowsRead = 0;
            while (reader.Read())
            {
                rowsRead++;
                AssertRowValuesEqual(
                    reader,
                    SqlCreateIcebergTableColumns.Split('\n'),
                    i32,
                    i64,
                    dec,
                    dbl,
                    flt,
                    txt,
                    b1,
                    b0,
                    dt,
                    tm,
                    ntz,
                    ltz,
                    bi);
            }

            Assert.Equal(1, rowsRead);
        }
        finally
        {
            DropTableIfExists(conn, icebergTable);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public void ShouldJoinIcebergAndHybridTables(string resultFormat)
    {
        Skip.When(true, NotInCiScope);
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var icebergTable = UniqueTableName("DOTNET_TEST_DATA_IB");
        var hybridTable = UniqueTableName("DOTNET_TEST_DATA_HY");
        using var conn = OpenConnection();
        try
        {
            CreateIcebergTable(conn, icebergTable);
            CreateHybridTable(conn, hybridTable);
            InsertManyRows(conn, icebergTable, 10, I32, I64, Dec, Dbl, Flt, Txt, B1, B0, _dt, _tm, _ntz, _ltz, _bi);
            InsertHybridTableData(conn, hybridTable);
            SetResultFormat(conn, resultFormat);

            var sql = $@"select i.nu1,i.nu2,i.nu3,i.nu4,i.f,i.tx,i.bt,i.bf,i.dt,i.tm,i.ntz,i.ltz,i.bi, h.id,h.nu,h.tx2
                             from {icebergTable} i
                             join {hybridTable} h
                               on i.nu1 = h.nu order by i.nu1";

            var resultSetColumns = @"nu1 number(10,0),
                                    nu2 number(19,0),
                                    nu3 number(18,2),
                                    nu4 number(38,0),
                                    f float,
                                    tx varchar(16777216),
                                    bt boolean,
                                    bf boolean,
                                    dt date,
                                    tm time,
                                    ntz timestamp_ntz(6),
                                    ltz timestamp_ltz(6),
                                    bi binary(5),
                                    id number(10,0),
                                    nu number(10,0),
                                    tx2 varchar(100)".Split('\n');
            using var reader = ExecuteReader(conn, sql);
            Assert.True(reader.Read());
            AssertRowValuesEqual(
                reader,
                resultSetColumns,
                I32,
                I64,
                Dec,
                Dbl,
                Flt,
                Txt,
                B1,
                B0,
                _dt,
                _tm,
                _ntz,
                _ltz,
                _bi,
                Id1,
                I32,
                Txt1);
            Assert.True(reader.Read());
            AssertRowValuesEqual(
                reader,
                resultSetColumns,
                I32,
                I64,
                Dec,
                Dbl,
                Flt,
                Txt,
                B1,
                B0,
                _dt,
                _tm,
                _ntz,
                _ltz,
                _bi,
                Id2,
                I32,
                Txt2);
            Assert.False(reader.Read());
        }
        finally
        {
            DropTableIfExists(conn, icebergTable);
            DropTableIfExists(conn, hybridTable);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public void ShouldDeleteRowsMatchingBinding(string resultFormat)
    {
        Skip.When(true, NotInCiScope);
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var icebergTable = UniqueTableName("DOTNET_TEST_DATA_IB");
        using var conn = OpenConnection();
        try
        {
            CreateIcebergTable(conn, icebergTable);
            InsertManyRows(conn, icebergTable, 100, I32, I64, Dec, Dbl, Flt, Txt, B1, B0, _dt, _tm, _ntz, _ltz, _bi);
            SetResultFormat(conn, resultFormat);

            using var cmd = CreateCommand(conn, $"delete from {icebergTable} where nu1 = ?");
            AddParameter(cmd, "1", DbType.Int32, I32);
            using var removed = cmd.ExecuteReader();

            Assert.Equal(1, removed.RecordsAffected);
            using var left = ExecuteReader(conn, $"select count(*) from {icebergTable} where nu1 <> {I32}");
            Assert.True(left.Read());
            Assert.Equal(99, left.GetInt32(0));
        }
        finally
        {
            DropTableIfExists(conn, icebergTable);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public void ShouldDeleteAllRows(string resultFormat)
    {
        Skip.When(true, NotInCiScope);
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var icebergTable = UniqueTableName("DOTNET_TEST_DATA_IB");
        using var conn = OpenConnection();
        try
        {
            CreateIcebergTable(conn, icebergTable);
            InsertManyRows(conn, icebergTable, 100, I32, I64, Dec, Dbl, Flt, Txt, B1, B0, _dt, _tm, _ntz, _ltz, _bi);
            SetResultFormat(conn, resultFormat);

            using var cmd = CreateCommand(conn, $"delete from {icebergTable}");
            using var removed = cmd.ExecuteReader();

            Assert.Equal(100, removed.RecordsAffected);
            using var left = ExecuteReader(conn, $"select count(*) from {icebergTable}");
            Assert.True(left.Read());
            Assert.Equal(0, left.GetInt32(0));
        }
        finally
        {
            DropTableIfExists(conn, icebergTable);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public void ShouldReadIcebergTableInMultiStatement(string resultFormat)
    {
        Skip.When(true, NotInCiScope);
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var icebergTable = UniqueTableName("DOTNET_TEST_DATA_IB");
        using var conn = OpenConnection();
        try
        {
            CreateIcebergTable(conn, icebergTable);
            InsertSingleRow(conn, icebergTable, I32, I64, Dec, Dbl, Flt, Txt, B1, B0, _dt, _tm, _ntz, _ltz, _bi);
            SetResultFormat(conn, resultFormat);

            using var cmd = CreateCommand(
                conn,
                $"select * from {icebergTable};select 1;select current_timestamp;select * from {icebergTable}");
            AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int32, 4);
            using var reader = cmd.ExecuteReader();

            var rowsRead = 0;
            while (reader.Read())
            {
                rowsRead++;
                AssertRowValuesEqual(
                    reader,
                    SqlCreateIcebergTableColumns.Split('\n'),
                    I32,
                    I64,
                    Dec,
                    Dbl,
                    Flt,
                    Txt,
                    B1,
                    B0,
                    _dt,
                    _tm,
                    _ntz,
                    _ltz,
                    _bi);
            }

            Assert.Equal(1, rowsRead);
        }
        finally
        {
            DropTableIfExists(conn, icebergTable);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public void ShouldBatchInsertLargeDataWithNulls(string resultFormat)
    {
        Skip.When(true, NotInCiScope);
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var icebergTable = UniqueTableName("DOTNET_TEST_DATA_IB");
        using var conn = OpenConnection();
        try
        {
            CreateIcebergTable(conn, icebergTable);
            SetResultFormat(conn, resultFormat);
            InsertManyRowsWithNulls(
                conn,
                icebergTable,
                20_000,
                I32,
                I64,
                Dec,
                Dbl,
                Flt,
                Txt,
                B1,
                B0,
                _dt,
                _tm,
                _ntz,
                _ltz,
                _bi);

            using var reader = ExecuteReader(
                conn,
                $"select {SqlColumnsSimpleTypes} from {icebergTable} order by nu1");

            var resultSetColumns = SqlCreateIcebergTableColumns.Split('\n');
            var expected = new object?[] { I32, I64, Dec, Dbl, Flt, Txt, B1, B0, _dt, _tm, _ntz, _ltz, _bi };
            var rowsRead = 0;
            while (reader.Read())
            {
                ++rowsRead;
                expected[0] = rowsRead;
                var expectedRow = NullEachNthValueBesidesFirst(expected, rowsRead - 1);
                AssertRowValuesEqual(reader, resultSetColumns, expectedRow);
            }

            Assert.Equal(20_000, rowsRead);
        }
        finally
        {
            DropTableIfExists(conn, icebergTable);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public void ShouldReadStructuredTypesAsJsonString(string resultFormat)
    {
        Skip.When(true, NotInCiScope);
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        var icebergTable = UniqueTableName("DOTNET_TEST_DATA_IB");
        using var conn = OpenConnection();
        try
        {
            SetResultFormat(conn, resultFormat);
            CreateIcebergTable(conn, icebergTable);
            var sql = $@"insert into {icebergTable} ({SqlColumnsStructureTypes})
                            select
                                [1,2,3]::ARRAY(number),
                                {{'a' : 1, 'b': 'two'}}::OBJECT(a number, b varchar),
                                {{'4':'one', '5': 'two', '6': 'three'}}::MAP(varchar, varchar)
                            ";
            ExecuteNonQuery(conn, sql);

            using var dbDataReader = ExecuteReader(conn, $"select {SqlColumnsStructureTypes} from {icebergTable}");
            var rowsRead = 0;
            while (dbDataReader.Read())
            {
                rowsRead++;
                Assert.Equal("[1,2,3]", RemoveBlanks(dbDataReader.GetString(0)));
                Assert.Equal("{\"a\":1,\"b\":\"two\"}", RemoveBlanks(dbDataReader.GetString(1)));
                Assert.Equal("{\"4\":\"one\",\"5\":\"two\",\"6\":\"three\"}", RemoveBlanks(dbDataReader.GetString(2)));
            }

            Assert.Equal(1, rowsRead);
        }
        finally
        {
            DropTableIfExists(conn, icebergTable);
        }
    }

    private ISnowflakeDbConnection OpenConnection()
    {
        var conn = Fixture.Factory.Create(Output);
        conn.Open();
        return conn;
    }

    private static string UniqueTableName(string prefix) => $"{prefix}_{Guid.NewGuid():N}";

    private static void CreateIcebergTable(ISnowflakeDbConnection conn, string tableName) =>
        ExecuteNonQuery(
            conn,
            $"create or replace iceberg table {tableName} ({SqlCreateIcebergTableColumns}) {IcebergTableCreateFlags}");

    private static void CreateHybridTable(ISnowflakeDbConnection conn, string tableName) =>
        ExecuteNonQuery(conn, $"create or replace hybrid table {tableName} ({SqlCreateHybridTableColumns})");

    private static void SetResultFormat(ISnowflakeDbConnection conn, string resultFormat) =>
        ExecuteNonQuery(conn, $"alter session set DOTNET_QUERY_RESULT_FORMAT={resultFormat}");

    private static void DropTableIfExists(ISnowflakeDbConnection conn, string tableName) =>
        ExecuteNonQuery(conn, $"drop table if exists {tableName}");

    private static void InsertSingleRow(ISnowflakeDbConnection conn, string tableName, params object[] bindings)
    {
        Assert.Equal(13, bindings.Length);
        var sqlInsert = $"insert into {tableName} ({SqlColumnsSimpleTypes}) values (?,?,?,?,?,?,?,?,?,?,?,?,?)";
        using var cmd = CreateCommand(conn, sqlInsert);
        AddParameter(cmd, "1", DbType.Int32, bindings[0]);
        AddParameter(cmd, "2", DbType.Int64, bindings[1]);
        AddParameter(cmd, "3", DbType.Decimal, bindings[2]);
        AddParameter(cmd, "4", DbType.Double, bindings[3]);
        AddParameter(cmd, "5", DbType.Double, bindings[4]);
        AddParameter(cmd, "6", DbType.String, bindings[5]);
        AddParameter(cmd, "7", DbType.Boolean, bindings[6]);
        AddParameter(cmd, "8", DbType.Boolean, bindings[7]);
        AddParameter(cmd, "9", DbType.DateTime, bindings[8]);
        AddParameter(cmd, "10", DbType.DateTime, bindings[9]);
        AddParameter(cmd, "11", DbType.DateTime, bindings[10]);
        SetTimestampLtz(AddParameter(cmd, "12", DbType.DateTimeOffset, bindings[11]));
        AddParameter(cmd, "13", DbType.Binary, bindings[12]);
        Assert.Equal(1, cmd.ExecuteNonQuery());
    }

    private static void InsertManyRows(ISnowflakeDbConnection conn, string tableName, int times, params object[] bindings)
    {
        Assert.Equal(13, bindings.Length);
        var sqlInsert = $"insert into {tableName} ({SqlColumnsSimpleTypes}) values (?,?,?,?,?,?,?,?,?,?,?,?,?)";
        using var cmd = CreateCommand(conn, sqlInsert);
        AddParameter(cmd, "1", DbType.Int32, Enumerable.Range((int)bindings[0], times).ToArray());
        AddParameter(cmd, "2", DbType.Int64, Enumerable.Repeat((long)bindings[1], times).ToArray());
        AddParameter(cmd, "3", DbType.Decimal, Enumerable.Repeat((decimal)bindings[2], times).ToArray());
        AddParameter(cmd, "4", DbType.Double, Enumerable.Repeat((double)bindings[3], times).ToArray());
        AddParameter(cmd, "5", DbType.Double, Enumerable.Repeat((float)bindings[4], times).ToArray());
        AddParameter(cmd, "6", DbType.String, Enumerable.Repeat((string)bindings[5], times).ToArray());
        AddParameter(cmd, "7", DbType.Boolean, Enumerable.Repeat((bool)bindings[6], times).ToArray());
        AddParameter(cmd, "8", DbType.Boolean, Enumerable.Repeat((bool)bindings[7], times).ToArray());
        AddParameter(cmd, "9", DbType.DateTime, Enumerable.Repeat((DateTime)bindings[8], times).ToArray());
        AddParameter(cmd, "10", DbType.DateTime, Enumerable.Repeat((DateTime)bindings[9], times).ToArray());
        AddParameter(cmd, "11", DbType.DateTime, Enumerable.Repeat((DateTime)bindings[10], times).ToArray());
        SetTimestampLtz(AddParameter(cmd, "12", DbType.DateTimeOffset, Enumerable.Repeat((DateTimeOffset)bindings[11], times).ToArray()));
        AddParameter(cmd, "13", DbType.Binary, Enumerable.Repeat((byte[])bindings[12], times).ToArray());
        Assert.Equal(times, cmd.ExecuteNonQuery());
    }

    private static void InsertManyRowsWithNulls(
        ISnowflakeDbConnection conn,
        string tableName,
        int times,
        params object[] bindings)
    {
        Assert.Equal(13, bindings.Length);
        var sqlInsert = $"insert into {tableName} ({SqlColumnsSimpleTypes}) values (?,?,?,?,?,?,?,?,?,?,?,?,?)";
        using var cmd = CreateCommand(conn, sqlInsert);
        AddParameter(cmd, "1", DbType.Int32, Enumerable.Range((int)bindings[0], times).ToArray());

        var longArray = Enumerable.Repeat((long?)bindings[1], times).ToArray();
        AddParameter(cmd, "2", DbType.Int64, NullEachNthValue(longArray, 2));

        var decArray = Enumerable.Repeat((decimal?)bindings[2], times).ToArray();
        AddParameter(cmd, "3", DbType.Decimal, NullEachNthValue(decArray, 3));

        var dblArray = Enumerable.Repeat((double?)bindings[3], times).ToArray();
        AddParameter(cmd, "4", DbType.Double, NullEachNthValue(dblArray, 4));

        var fltArray = Enumerable.Repeat((float?)bindings[4], times).ToArray();
        AddParameter(cmd, "5", DbType.Double, NullEachNthValue(fltArray, 5));

        var strArray = Enumerable.Repeat((string)bindings[5], times).ToArray();
        AddParameter(cmd, "6", DbType.String, NullEachNthValue(strArray, 6));

        var bltArray = Enumerable.Repeat((bool?)bindings[6], times).ToArray();
        AddParameter(cmd, "7", DbType.Boolean, NullEachNthValue(bltArray, 7));

        var blfArray = Enumerable.Repeat((bool?)bindings[7], times).ToArray();
        AddParameter(cmd, "8", DbType.Boolean, NullEachNthValue(blfArray, 8));

        var dtArray = Enumerable.Repeat((DateTime?)bindings[8], times).ToArray();
        AddParameter(cmd, "9", DbType.Date, NullEachNthValue(dtArray, 9));

        var tmArray = Enumerable.Repeat((DateTime?)bindings[9], times).ToArray();
        AddParameter(cmd, "10", DbType.Time, NullEachNthValue(tmArray, 10));

        var ntzArray = Enumerable.Repeat((DateTime?)bindings[10], times).ToArray();
        AddParameter(cmd, "11", DbType.DateTime, NullEachNthValue(ntzArray, 11));

        var ltzArray = Enumerable.Repeat((DateTimeOffset?)bindings[11], times).ToArray();
        SetTimestampLtz(AddParameter(cmd, "12", DbType.DateTimeOffset, NullEachNthValue(ltzArray, 12)));

        var binArray = Enumerable.Repeat((byte[])bindings[12], times).ToArray();
        AddParameter(cmd, "13", DbType.Binary, NullEachNthValue(binArray, 13));

        Assert.Equal(times, cmd.ExecuteNonQuery());
    }

    private static void InsertHybridTableData(ISnowflakeDbConnection conn, string tableName)
    {
        using var cmd = CreateCommand(conn, $"insert into {tableName} ({SqlColumnsHybridTypes}) values (?,?,?)");
        AddParameter(cmd, "1", DbType.Int32, new[] { Id1, Id2 });
        AddParameter(cmd, "2", DbType.Int32, new[] { I32, I32 });
        AddParameter(cmd, "3", DbType.String, new[] { Txt1, Txt2 });
        cmd.ExecuteNonQuery();
    }

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

    private static DbParameter AddParameter(IDbCommand command, string name, DbType dbType, object? value)
    {
        var parameter = (DbParameter)command.CreateParameter();
        parameter.ParameterName = name;
        parameter.DbType = dbType;
        parameter.Value = value;
        command.Parameters.Add(parameter);
        return parameter;
    }

    private static void SetTimestampLtz(DbParameter parameter)
    {
        if (parameter is OldSnowflakeDbParameter oldParameter)
            oldParameter.SFDataType = OldSFDataType.TIMESTAMP_LTZ;
    }

    private static string ByteArrayToHexString(byte[] ba)
    {
        var hex = new StringBuilder(ba.Length * 2);
        foreach (var b in ba)
            hex.AppendFormat(CultureInfo.InvariantCulture, "{0:x2}", b);
        return hex.ToString();
    }

    private static T?[] NullEachNthValue<T>(T?[] sourceColumn, int nullEachNthItem)
        where T : struct
    {
        var destination = new T?[sourceColumn.Length];
        foreach (var rowIndex in Enumerable.Range(0, sourceColumn.Length))
            destination[rowIndex] = rowIndex % nullEachNthItem == 0 ? null : sourceColumn[rowIndex];
        return destination;
    }

    private static T?[] NullEachNthValue<T>(T?[] sourceColumn, int nullEachNthItem)
        where T : class
    {
        var destination = new T?[sourceColumn.Length];
        foreach (var rowIndex in Enumerable.Range(0, sourceColumn.Length))
            destination[rowIndex] = rowIndex % nullEachNthItem == 0 ? null : sourceColumn[rowIndex];
        return destination;
    }

    private static object?[] NullEachNthValueBesidesFirst(object?[] sourceRow, int nullEachNthItem)
    {
        var ret = new object?[sourceRow.Length];
        foreach (var column in Enumerable.Range(0, sourceRow.Length))
            ret[column] = column > 0 && nullEachNthItem % (column + 1) == 0 ? null : sourceRow[column];
        return ret;
    }

    private static string RemoveBlanks(string text) => text.Replace("\n", "").Replace(" ", "");

    private static bool IsTimeColumn(string columnDefinition)
    {
        var parts = columnDefinition.Split(new[] { ' ' }, 2);
        return parts.Length == 2 && parts[1].TrimEnd(',') == "TIME";
    }

    private static void AssertRowValuesEqual(DbDataReader actualRow, string[] columns, params object?[] expectedRow)
    {
        foreach (var idx in Enumerable.Range(0, expectedRow.Length))
        {
            var expected = expectedRow[idx];
            if (expected is DBNull || expected is null)
            {
                Assert.True(actualRow.IsDBNull(idx));
                continue;
            }

            var column = columns[idx].ToUpperInvariant().Trim();
            switch (expected)
            {
                case int i32:
                    Assert.Equal(i32, actualRow.GetInt32(idx));
                    break;
                case long i64:
                    Assert.Equal(i64, actualRow.GetInt64(idx));
                    break;
                case decimal dec:
                    Assert.Equal(dec, actualRow.GetDecimal(idx));
                    break;
                case float flt:
                    Assert.Equal(flt, actualRow.GetFloat(idx));
                    break;
                case string str:
                    Assert.Equal(str, actualRow.GetString(idx));
                    break;
                case bool bl:
                    Assert.Equal(bl, actualRow.GetBoolean(idx));
                    break;
                case DateTime dt:
                    var frmt = IsTimeColumn(column) ? FormatHms : FormatYmdHmsf;
                    Assert.Equal(
                        dt.ToString(frmt, CultureInfo.InvariantCulture),
                        actualRow.GetDateTime(idx).ToString(frmt, CultureInfo.InvariantCulture));
                    break;
                case DateTimeOffset dto:
                    Assert.Equal(
                        dto.ToUniversalTime().ToString(FormatYmdHmsfZ, CultureInfo.InvariantCulture),
                        actualRow.GetFieldValue<DateTimeOffset>(idx)
                            .ToUniversalTime()
                            .ToString(FormatYmdHmsfZ, CultureInfo.InvariantCulture));
                    break;
                case byte[] bt:
                    Assert.Equal(bt, actualRow.GetFieldValue<byte[]>(idx));
                    break;
            }
        }
    }
}
