extern alias OldDriver;
using System.Data.Common;
using System.Globalization;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbDataReader = OldDriver::Snowflake.Data.Client.SnowflakeDbDataReader;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.E2E;

public sealed class SFMultiStatementsTest : LiveE2ETestBase
{
    private const int MultiStatementCountMismatchErrorCode = 8;
    private const string FeatureNotSupportedSqlState = "0A000";

    public SFMultiStatementsTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectWithoutBindingAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        var testDate = "2020-03-11 12:34:56 +0000";
        var testTime = "12:34:56";
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, 5);
        cmd.CommandText = "select 1; select 2, 3; select 4, 5, 6;select true, false, null;" +
                          $"select '{testDate}'::DATETIME, '{testDate}'::TIMESTAMP_TZ, '{testTime}'::TIME";
        using var reader = cmd.ExecuteReader();

        Assert.True(reader.Read());
        Assert.Equal(1, reader.GetDouble(0));
        Assert.Equal(1, reader.GetFloat(0));
        Assert.Equal(1, reader.GetInt64(0));
        Assert.Equal(1, reader.GetInt32(0));
        Assert.Equal(1, reader.GetInt16(0));
        Assert.Equal(1, reader.GetByte(0));
        Assert.Equal(1L, reader.GetValue(0));
        Assert.False(reader.Read());

        Assert.True(reader.NextResult());
        Assert.True(reader.Read());
        Assert.Equal(2, reader.GetInt32(0));
        Assert.Equal(3, reader.GetInt32(1));
        Assert.False(reader.Read());

        Assert.True(reader.NextResult());
        Assert.True(reader.Read());
        Assert.Equal(4, reader.GetInt32(0));
        Assert.Equal(5, reader.GetInt32(1));
        Assert.Equal(6, reader.GetInt32(2));
        Assert.False(reader.Read());

        Assert.True(reader.NextResult());
        Assert.True(reader.Read());
        Assert.True(reader.GetBoolean(0));
        Assert.False(reader.GetBoolean(1));
        Assert.Equal(DBNull.Value, reader.GetValue(2));
        Assert.False(reader.IsDBNull(0));
        Assert.False(reader.IsDBNull(1));
        Assert.True(reader.IsDBNull(2));
        Assert.False(reader.Read());

        Assert.True(reader.NextResult());
        Assert.True(reader.Read());
        Assert.Equal(
            DateTime.Parse(testDate, CultureInfo.InvariantCulture).ToUniversalTime(),
            reader.GetDateTime(0));
        AssertTimestampTzAndTime(reader, testDate, testTime);
        Assert.False(reader.Read());

        Assert.False(reader.NextResult());
        Assert.False(reader.Read());

        reader.Close();
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory(retriesCount: RetriesCount.Once)]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReadMultipleResultSetsWithAsyncReaderAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, 2);
        cmd.CommandText = "select 1; select 2, 3";
        using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        Assert.Equal(1, reader.GetDouble(0));
        Assert.Equal(1, reader.GetFloat(0));
        Assert.Equal(1, reader.GetInt64(0));
        Assert.Equal(1, reader.GetInt32(0));
        Assert.Equal(1, reader.GetInt16(0));
        Assert.Equal(1, reader.GetByte(0));
        Assert.Equal(1L, reader.GetValue(0));
        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        Assert.True(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        Assert.Equal(2, reader.GetInt32(0));
        Assert.Equal(3, reader.GetInt32(1));
        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        Assert.False(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
        Assert.False(await reader.ReadAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));

        await reader.CloseAsync().ConfigureAwait(false);
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory(retriesCount: RetriesCount.Thrice)]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectWithBindingAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, 3);
        for (var i = 1; i <= 6; i++)
            AddParameter(cmd, i.ToString(CultureInfo.InvariantCulture), DbType.Int16, i);
        cmd.CommandText = "select ?; select ?, ?; select ?, ?, ?";
        using var reader = cmd.ExecuteReader();

        Assert.True(reader.Read());
        Assert.Equal(1, reader.GetInt32(0));
        Assert.False(reader.Read());

        Assert.True(reader.NextResult());
        Assert.True(reader.Read());
        Assert.Equal(2, reader.GetInt32(0));
        Assert.Equal(3, reader.GetInt32(1));
        Assert.False(reader.Read());

        Assert.True(reader.NextResult());
        Assert.True(reader.Read());
        Assert.Equal(4, reader.GetInt32(0));
        Assert.Equal(5, reader.GetInt32(1));
        Assert.Equal(6, reader.GetInt32(2));
        Assert.False(reader.Read());

        Assert.False(reader.NextResult());

        reader.Close();
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldExecuteMixedQueryTypesWithBindingAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        var tableName = UniqueName("DOTNET_MS");
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            SetResultFormat(conn, resultFormat);

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"create or replace table {tableName}(cola integer, colb string);" +
                              $"insert into {tableName} values (?, ?);" +
                              $"insert into {tableName} values (?, ?), (?, ?);" +
                              $"select * from {tableName};" +
                              $"drop table if exists {tableName}";

            AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, 5);
            AddParameter(cmd, "1", DbType.Int16, 1);
            AddParameter(cmd, "2", DbType.String, "str1");
            AddParameter(cmd, "3", DbType.Int16, 2);
            AddParameter(cmd, "4", DbType.String, "str2");
            AddParameter(cmd, "5", DbType.Int16, 3);
            AddParameter(cmd, "6", DbType.String, "str3");

            using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.True(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.True(reader.NextResult());
            Assert.True(reader.HasRows);
            Assert.Equal(1, reader.RecordsAffected);

            Assert.True(reader.NextResult());
            Assert.True(reader.HasRows);
            Assert.Equal(2, reader.RecordsAffected);

            Assert.True(reader.NextResult());
            Assert.True(reader.HasRows);
            Assert.Equal(-1, reader.RecordsAffected);
            Assert.True(reader.Read());
            Assert.Equal(1, reader.GetInt32(0));
            Assert.Equal("str1", reader.GetString(1));
            Assert.True(reader.Read());
            Assert.Equal(2, reader.GetInt32(0));
            Assert.Equal("str2", reader.GetString(1));
            Assert.True(reader.Read());
            Assert.Equal(3, reader.GetInt32(0));
            Assert.Equal("str3", reader.GetString(1));
            Assert.False(reader.Read());

            Assert.True(reader.NextResult());
            Assert.True(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.False(reader.NextResult());
            reader.Close();
        }
        finally
        {
            DropTableIfExists(conn, tableName);
            await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeFact]
    public async Task ShouldExecuteMixedQueryTypesWithBindingWhenMultiStatementCountIsZeroAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        var tableName = UniqueName("DOTNET_MS");
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            var schema = CurrentSchema(conn);
            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"use schema {schema};" +
                              $"create or replace table {tableName}(cola integer, colb string);" +
                              $"insert into {tableName} values (?, ?);" +
                              $"insert into {tableName} values (?, ?), (?, ?);" +
                              $"select * from {tableName};" +
                              $"drop table if exists {tableName}";

            AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, 0);
            AddParameter(cmd, "1", DbType.Int16, 1);
            AddParameter(cmd, "2", DbType.String, "str1");
            AddParameter(cmd, "3", DbType.Int16, 2);
            AddParameter(cmd, "4", DbType.String, "str2");
            AddParameter(cmd, "5", DbType.Int16, 3);
            AddParameter(cmd, "6", DbType.String, "str3");

            using var reader = cmd.ExecuteReader();

            Assert.True(reader.NextResult());

            Assert.True(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.True(reader.NextResult());
            Assert.True(reader.HasRows);
            Assert.Equal(1, reader.RecordsAffected);

            Assert.True(reader.NextResult());
            Assert.True(reader.HasRows);
            Assert.Equal(2, reader.RecordsAffected);

            Assert.True(reader.NextResult());
            Assert.True(reader.HasRows);
            Assert.Equal(-1, reader.RecordsAffected);
            Assert.True(reader.Read());
            Assert.Equal(1, reader.GetInt32(0));
            Assert.Equal("str1", reader.GetString(1));
            Assert.True(reader.Read());
            Assert.Equal(2, reader.GetInt32(0));
            Assert.Equal("str2", reader.GetString(1));
            Assert.True(reader.Read());
            Assert.Equal(3, reader.GetInt32(0));
            Assert.Equal("str3", reader.GetString(1));
            Assert.False(reader.Read());

            Assert.True(reader.NextResult());
            Assert.True(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.False(reader.NextResult());
            reader.Close();
        }
        finally
        {
            DropTableIfExists(conn, tableName);
            conn.Close();
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReturnTotalRowsAffectedFromMultiStatementExecuteNonQueryAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        var tableName = UniqueName("DOTNET_MS");
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            SetResultFormat(conn, resultFormat);

            using var cmd = conn.CreateCommand();
            cmd.CommandText = $"create or replace temporary table {tableName}(cola integer, colb string);" +
                              $"insert into {tableName} values (?, ?);" +
                              $"insert into {tableName} values (?, ?), (?, ?);" +
                              $"select * from {tableName};" +
                              $"drop table if exists {tableName}";

            AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, 5);
            AddParameter(cmd, "1", DbType.Int16, 1);
            AddParameter(cmd, "2", DbType.String, "str1");
            AddParameter(cmd, "3", DbType.Int16, 2);
            AddParameter(cmd, "4", DbType.String, "str2");
            AddParameter(cmd, "5", DbType.Int16, 3);
            AddParameter(cmd, "6", DbType.String, "str3");

            var count = await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
            Assert.Equal(3, count);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
            await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReportRowsAffectedForAllQueryTypesAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        var tableName = UniqueName("DOTNET_MS");
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            SetResultFormat(conn, resultFormat);
            var role = CurrentRole(conn);

            using var cmd = conn.CreateCommand();
            cmd.CommandText = "select 1;" +
                              $"create or replace temporary table {tableName}(c1 varchar);" +
                              $"explain using text select * from {tableName};" +
                              "show parameters;" +
                              $"insert into {tableName} values ('str1');" +
                              $"desc table {tableName};" +
                              $"list @%{tableName};" +
                              $"remove @%{tableName};" +
                              $"create or replace temporary procedure P1_{tableName}() returns varchar language javascript as $$ return ''; $$;" +
                              $"call P1_{tableName}();" +
                              $"use role {role}";

            AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, 11);

            using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.True(reader.HasRows);
            Assert.Equal(-1, reader.RecordsAffected);

            Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.True(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.True(reader.HasRows);
            Assert.True(reader.RecordsAffected == 0 || reader.RecordsAffected == -1);

            Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.True(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.True(reader.HasRows);
            Assert.Equal(1, reader.RecordsAffected);

            Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.True(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.False(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.False(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.True(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.True(reader.HasRows);
            Assert.True(reader.RecordsAffected == 0 || reader.RecordsAffected == -1);

            Assert.True(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            Assert.True(reader.HasRows);
            Assert.Equal(0, reader.RecordsAffected);

            Assert.False(await reader.NextResultAsync(TestContext.Current.CancellationToken).ConfigureAwait(false));
            await reader.CloseAsync().ConfigureAwait(false);
        }
        finally
        {
            DropTableIfExists(conn, tableName);
            await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldHonorMultiStatementCountFromSessionAndParameterAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        cmd.CommandText = "alter session set MULTI_STATEMENT_COUNT=1";
        await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        cmd.CommandText = "select 1; select 2; select 3";
        await AssertMultiStatementCountMismatchAsync(cmd, TestContext.Current.CancellationToken).ConfigureAwait(false);

        cmd.CommandText = "alter session set MULTI_STATEMENT_COUNT=0";
        await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        cmd.CommandText = "select 1; select 2; select 3";
        await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        var stmtCountParam = cmd.CreateParameter();
        stmtCountParam.ParameterName = "MULTI_STATEMENT_COUNT";
        stmtCountParam.DbType = DbType.Int16;
        stmtCountParam.Value = 4;
        cmd.Parameters.Add(stmtCountParam);

        cmd.CommandText = "select 1; select 2; select 3";
        await AssertMultiStatementCountMismatchAsync(cmd, TestContext.Current.CancellationToken).ConfigureAwait(false);

        cmd.Parameters.Clear();
        stmtCountParam = cmd.CreateParameter();
        stmtCountParam.ParameterName = "MULTI_STATEMENT_COUNT";
        stmtCountParam.DbType = DbType.Int16;
        stmtCountParam.Value = 3;
        cmd.Parameters.Add(stmtCountParam);

        cmd.CommandText = "select 1; select 2; select 3";
        await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        cmd.Parameters.Clear();
        cmd.CommandText = "alter session set MULTI_STATEMENT_COUNT=1";
        await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        stmtCountParam = cmd.CreateParameter();
        stmtCountParam.ParameterName = "MULTI_STATEMENT_COUNT";
        stmtCountParam.DbType = DbType.Int16;
        stmtCountParam.Value = 3;
        cmd.Parameters.Add(stmtCountParam);

        cmd.CommandText = "select 1; select 2; select 3";
        await cmd.ExecuteNonQueryAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldReturnResultSetForAllQueryTypesAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        var tableName = UniqueName("DOTNET_MS");
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            SetResultFormat(conn, resultFormat);
            var role = CurrentRole(conn);

            using var cmd = conn.CreateCommand();
            cmd.CommandText = "set query_tag = (select 'dummy_tag');" +
                              "alter session set query_tag='dummy_tag';" +
                              "select 1;" +
                              $"create or replace temporary table {tableName}(c1 varchar);" +
                              $"explain using text select * from {tableName};" +
                              "show parameters;" +
                              $"insert into {tableName} values ('str1');" +
                              $"update {tableName} set c1 = 'str2';" +
                              $"select * from {tableName};" +
                              $"desc table {tableName};" +
                              $"copy into @%{tableName} from {tableName};" +
                              $"list @%{tableName};" +
                              $"remove @%{tableName};" +
                              $"create or replace temporary procedure P1_{tableName}() returns varchar language javascript as $$ return ''; $$;" +
                              $"call P1_{tableName}();" +
                              $"use role {role}";

            const int stmtCount = 16;
            AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, stmtCount);

            using var reader = await cmd.ExecuteReaderAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

            Assert.True(reader.HasRows);
            Assert.True(reader.Read());

            for (var i = 1; i < stmtCount; i++)
            {
                Assert.True(reader.NextResult());
                Assert.True(reader.HasRows);
                Assert.True(reader.Read());
            }

            Assert.False(reader.NextResult());
            reader.Close();
        }
        finally
        {
            DropTableIfExists(conn, tableName);
            await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        }
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldExecuteMultipleUseStatementsAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);
        var schema = CurrentSchema(conn);

        using var cmd = conn.CreateCommand();
        AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, 3);
        cmd.CommandText = $"use schema {schema}; use schema {schema}; select 1";
        using var reader = cmd.ExecuteReader();

        Assert.True(reader.HasRows);
        Assert.Equal(0, reader.RecordsAffected);

        Assert.True(reader.NextResult());
        Assert.True(reader.HasRows);
        Assert.Equal(0, reader.RecordsAffected);

        Assert.True(reader.NextResult());
        Assert.True(reader.Read());
        Assert.Equal(1, reader.GetInt32(0));
        Assert.False(reader.Read());

        Assert.False(reader.NextResult());

        reader.Close();
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    [SnowflakeTheory]
    [InlineData("JSON")]
    [InlineData("ARROW")]
    public async Task ShouldSelectWithMultiStatementCountZeroAsync(string resultFormat)
    {
        if (resultFormat == "JSON")
        {
            // TODO(SNOW-4382580): the new driver ignores DOTNET_QUERY_RESULT_FORMAT, so JSON results cannot be selected
            Skip.FutureMilestone();
        }

        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();
        using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        SetResultFormat(conn, resultFormat);

        using var cmd = conn.CreateCommand();
        AddParameter(cmd, "MULTI_STATEMENT_COUNT", DbType.Int16, 0);
        cmd.CommandText = "select 1; select 2; select 3";
        using var reader = cmd.ExecuteReader();

        Assert.True(reader.Read());
        Assert.Equal(1, reader.GetInt32(0));
        Assert.False(reader.Read());

        Assert.True(reader.NextResult());
        Assert.True(reader.Read());
        Assert.Equal(2, reader.GetInt32(0));
        Assert.False(reader.Read());

        Assert.True(reader.NextResult());
        Assert.True(reader.Read());
        Assert.Equal(3, reader.GetInt32(0));
        Assert.False(reader.Read());

        Assert.False(reader.NextResult());

        reader.Close();
        await conn.CloseAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
    }

    private static async Task AssertMultiStatementCountMismatchAsync(DbCommand cmd, CancellationToken cancellationToken)
    {
        var thrown = await Assert.ThrowsAnyAsync<DbException>(
            () => cmd.ExecuteNonQueryAsync(cancellationToken)).ConfigureAwait(false);
        AssertOldMultiStatementCountMismatch(thrown);
    }

    private static void AssertOldMultiStatementCountMismatch(DbException thrown)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
        {
            var old = Assert.IsType<OldSnowflakeDbException>(thrown);
            Assert.Equal(MultiStatementCountMismatchErrorCode, old.ErrorCode);
            Assert.Equal(FeatureNotSupportedSqlState, old.SqlState);
            return;
        }

        var neu = Assert.IsType<NewSnowflakeDbException>(thrown);
        Assert.Equal(MultiStatementCountMismatchErrorCode, neu.ErrorCode);
        Assert.Equal(FeatureNotSupportedSqlState, neu.SqlState);
    }

    private async Task<ISnowflakeDbConnection> OpenConnectionAsync(CancellationToken cancellationToken)
    {
        var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(cancellationToken).ConfigureAwait(false);
        conn.PreventPooling();
        return conn;
    }

    private static string UniqueName(string prefix) => $"{prefix}_{Guid.NewGuid():N}";

    private static void SetResultFormat(ISnowflakeDbConnection conn, string resultFormat) =>
        ExecuteNonQuery(conn, $"alter session set DOTNET_QUERY_RESULT_FORMAT = {(resultFormat == "ARROW" ? "ARROW_FORCE" : resultFormat)}");

    private static void DropTableIfExists(ISnowflakeDbConnection conn, string tableName)
    {
        try
        {
            ExecuteNonQuery(conn, $"drop table if exists {tableName}");
        }
        catch (Exception)
        {
        }
    }

    private static void ExecuteNonQuery(ISnowflakeDbConnection connection, string commandText)
    {
        using var command = connection.CreateCommand();
        command.CommandText = commandText;
        command.ExecuteNonQuery();
    }

    private static string CurrentSchema(ISnowflakeDbConnection conn)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select current_schema()";
        return Convert.ToString(cmd.ExecuteScalar(), CultureInfo.InvariantCulture)!;
    }

    private static string CurrentRole(ISnowflakeDbConnection conn)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = "select current_role()";
        return Convert.ToString(cmd.ExecuteScalar(), CultureInfo.InvariantCulture)!;
    }

    private static DbParameter AddParameter(IDbCommand command, string name, DbType dbType, object value)
    {
        var parameter = (DbParameter)command.CreateParameter();
        parameter.ParameterName = name;
        parameter.DbType = dbType;
        parameter.Value = value;
        command.Parameters.Add(parameter);
        return parameter;
    }

    private static void AssertTimestampTzAndTime(DbDataReader reader, string testDate, string testTime)
    {
        var expectedOffset = DateTimeOffset.Parse(testDate, CultureInfo.InvariantCulture).ToUniversalTime();
        var expectedTime = TimeSpan.Parse(testTime, CultureInfo.InvariantCulture);
        if (reader is OldSnowflakeDbDataReader oldReader)
        {
            Assert.Equal(expectedOffset, oldReader.GetValue(1));
            Assert.Equal(expectedTime, oldReader.GetTimeSpan(2));
            return;
        }

        Assert.Equal(expectedOffset, reader.GetValue(1));
        Assert.Equal(expectedTime, reader.GetFieldValue<TimeSpan>(2));
    }
}
