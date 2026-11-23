extern alias OldDriver;
using System.Data.Common;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;
using OldSnowflakeDbException = OldDriver::Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.E2E;

public sealed class SFStatementTypeTest : LiveE2ETestBase
{
    public SFStatementTypeTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldCallStoredProcedureWithBoundParametersAsync()
    {
        // TODO(SNOW-4382583): parameter bindings
        Skip.FutureMilestone();

        var ct = TestContext.Current.CancellationToken;
        var procedureName = $"TEST_SP_CALL_STMT_ENABLED_{Guid.NewGuid():N}";
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(ct).ConfigureAwait(false);
        using var command = conn.CreateCommand();
        try
        {
            try
            {
                command.CommandText = "ALTER SESSION SET USE_STATEMENT_TYPE_CALL_FOR_STORED_PROC_CALLS=true";
                await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
            }
            catch (DbException ex) when (IsSnowflakeDbException(ex))
            {
                Assert.Equal("22023", GetSqlState(ex));
            }

            command.CommandText = "create or replace procedure\n"
                                  + $"{procedureName}(in1 float, in2 variant)\n"
                                  + "returns string language javascript as $$\n"
                                  + "let res = snowflake.execute({sqlText: 'select ? c1, ? c2', binds:[IN1, JSON.stringify(IN2)]});\n"
                                  + "res.next();\n"
                                  + "return res.getColumnValueAsString(1) + ' ' + res.getColumnValueAsString(2) + ' ' + IN2;\n"
                                  + "$$;";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);

            command.CommandText = $"call {procedureName}(?, to_variant(?))";
            AddParameter(command, "1", DbType.Double, 1);
            AddParameter(command, "2", DbType.String, "[2,3]");

            using var reader = await command.ExecuteReaderAsync(ct).ConfigureAwait(false);
            while (reader.Read())
            {
                Assert.Equal("1 \"[2,3]\" [2,3]", reader.GetString(0));
            }
        }
        finally
        {
            command.Parameters.Clear();
            command.CommandText = $"drop procedure if exists {procedureName}(float, variant)";
            await command.ExecuteNonQueryAsync(ct).ConfigureAwait(false);
        }
    }

    private static bool IsSnowflakeDbException(Exception ex) =>
        ex is OldSnowflakeDbException || ex is NewSnowflakeDbException;

    private static string? GetSqlState(Exception ex) =>
        ex switch
        {
            OldSnowflakeDbException oldException => oldException.SqlState,
            NewSnowflakeDbException newException => newException.SqlState,
            _ => null,
        };

    private static void AddParameter(DbCommand command, string name, DbType dbType, object value)
    {
        var parameter = command.CreateParameter();
        parameter.ParameterName = name;
        parameter.DbType = dbType;
        parameter.Value = value;
        command.Parameters.Add(parameter);
    }
}
