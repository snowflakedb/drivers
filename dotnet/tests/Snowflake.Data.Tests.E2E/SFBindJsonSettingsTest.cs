using System.Data.Common;
using System.Globalization;
using Newtonsoft.Json;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.E2E.Fixtures;

namespace Snowflake.Data.Tests.E2E;

[CollectionDefinition(nameof(IsolatedCollection), DisableParallelization = true)]
public sealed class IsolatedCollection
{
}

[Collection(nameof(IsolatedCollection))]
[Trait("Category", "Conversion")]
public sealed class SFBindJsonSettingsTest : LiveE2ETestBase
{
    private const string TablePrefix = "DOTNET_TEST_SFBIND_JSON";

    public SFBindJsonSettingsTest(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
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
            using var conn = await OpenConnectionAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
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

    private async Task<ISnowflakeDbConnection> OpenConnectionAsync(CancellationToken cancellationToken)
    {
        var conn = Fixture.Factory.Create(Output);
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

    private static DbParameter AddParameter(DbCommand command, string name, DbType dbType, object? value)
    {
        var parameter = command.CreateParameter();
        parameter.ParameterName = name;
        parameter.DbType = dbType;
        parameter.Value = value;
        command.Parameters.Add(parameter);
        return parameter;
    }

    private sealed class JsonDefaultSettingsRestorer : IDisposable
    {
        private readonly object? _original;

        public JsonDefaultSettingsRestorer(object? original) => _original = original;

        public void Dispose() => JsonConvert.DefaultSettings = (Func<JsonSerializerSettings>?)_original;
    }
}
