using System.Data.Common;
using System.Globalization;
using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;
using NewSnowflakeDbException = Snowflake.Data.SnowflakeDbException;
using OldSnowflakeDbException = Snowflake.Data.Client.SnowflakeDbException;

namespace Snowflake.Data.Tests.Reference;

public sealed class SFDbTransactionTest : ReferenceTestBase
{
    public SFDbTransactionTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldExposeOwningConnectionOnTransactionAsync()
    {
        // TODO(SNOW-4382589): BeginTransaction on SnowflakeDbConnection
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var t1 = conn.BeginTransaction();

        Assert.Same(UnderlyingConnection(conn), t1.Connection);
    }

    [SnowflakeFact]
    public async Task ShouldUseReadCommittedIsolationLevelByDefaultAsync()
    {
        // TODO(SNOW-4382589): BeginTransaction on SnowflakeDbConnection
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        using var t1 = conn.BeginTransaction();

        Assert.Equal(IsolationLevel.ReadCommitted, t1.IsolationLevel);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.Thrice)]
    public async Task ShouldRollBackTransactionWhenDisposedAsync()
    {
        // TODO(SNOW-4382589): BeginTransaction on SnowflakeDbConnection
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            ExecuteNonQuery(conn, $"CREATE OR REPLACE TABLE {tableName}(c INT)");

            using (var t1 = await BeginTransactionAsync(conn, TestContext.Current.CancellationToken).ConfigureAwait(false))
            {
                using var t1c1 = conn.CreateCommand();
                t1c1.Transaction = t1;
                t1c1.CommandText = $"insert into {tableName} values (1)";
                t1c1.ExecuteNonQuery();
            }

            using var c2 = conn.CreateCommand();
            c2.CommandText = $"SELECT * FROM {tableName}";
            using var reader2 = c2.ExecuteReader();
            Assert.False(reader2.Read());
        }
        finally
        {
            ExecuteNonQuery(conn, $"DROP TABLE IF EXISTS {tableName}");
        }
    }

    [SnowflakeFact]
    public async Task ShouldNotRollBackAfterTransactionIsCommittedAsync()
    {
        // TODO(SNOW-4382589): BeginTransaction on SnowflakeDbConnection
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            ExecuteNonQuery(conn, $"CREATE OR REPLACE TABLE {tableName}(x TIMESTAMP_NTZ, a INTEGER)");

            using (var transaction = await BeginTransactionAsync(conn, TestContext.Current.CancellationToken).ConfigureAwait(false))
            {
                using var t1c1 = conn.CreateCommand();
                t1c1.Transaction = transaction;
                t1c1.CommandText = $"insert into {tableName} values (current_timestamp(), 1), (current_timestamp(), 2), (current_timestamp(), 3)";
                t1c1.ExecuteNonQuery();
                t1c1.Transaction.Commit();

                using var t1c2 = conn.CreateCommand();
                t1c2.Transaction = transaction;
                t1c2.CommandText = "BEGIN";
                t1c2.ExecuteNonQuery();

                using var t1c3 = conn.CreateCommand();
                t1c3.Transaction = transaction;
                t1c3.CommandText = $"insert into {tableName} values (current_timestamp(), 4)";
                t1c3.ExecuteNonQuery();
                t1c3.Transaction.Rollback();
            }

            Assert.Equal(4, CountRows(conn, tableName));
        }
        finally
        {
            ExecuteNonQuery(conn, $"DROP TABLE IF EXISTS {tableName}");
        }
    }

    [SnowflakeFact(RetriesCount = RetriesCount.Once)]
    public async Task ShouldRollBackOnlySecondOfTwoTransactionsAsync()
    {
        // TODO(SNOW-4382589): BeginTransaction on SnowflakeDbConnection
        Skip.FutureMilestone();

        var tableName = UniqueTableName();
        using var conn = Fixture.Factory.Create(Output);
        await conn.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);
        try
        {
            ExecuteNonQuery(conn, $"CREATE OR REPLACE TABLE {tableName}(x TIMESTAMP_NTZ, a INTEGER)");

            using (var transaction = conn.BeginTransaction())
            {
                using var t1c1 = conn.CreateCommand();
                t1c1.Transaction = (DbTransaction)transaction;
                t1c1.CommandText = $"insert into {tableName} values (current_timestamp(), 1), (current_timestamp(), 2), (current_timestamp(), 3)";
                t1c1.ExecuteNonQuery();
                t1c1.Transaction.Commit();
            }

            using (var transaction2 = await BeginTransactionAsync(conn, TestContext.Current.CancellationToken).ConfigureAwait(false))
            {
                using var t2c2 = conn.CreateCommand();
                t2c2.Transaction = transaction2;
                t2c2.CommandText = $"insert into {tableName} values (current_timestamp(), 4)";
                t2c2.ExecuteNonQuery();
                t2c2.Transaction.Rollback();
            }

            Assert.Equal(3, CountRows(conn, tableName));
        }
        finally
        {
            ExecuteNonQuery(conn, $"DROP TABLE IF EXISTS {tableName}");
        }
    }

    [SnowflakeFact]
    public void ShouldThrowWhenBeginningTransactionOnClosedConnection()
    {
        // TODO(SNOW-4382589): BeginTransaction on SnowflakeDbConnection
        Skip.FutureMilestone();

        using var conn = Fixture.Factory.Create(Output);

        AssertThrowsSnowflakeDbException(() => conn.BeginTransaction());
    }

    private static string UniqueTableName() => $"DOTNET_TEST_TRANSACTION_{Guid.NewGuid():N}";

    private static DbConnection UnderlyingConnection(ISnowflakeDbConnection conn)
    {
        using var cmd = conn.CreateCommand();
        return cmd.Connection!;
    }

    private static async Task<DbTransaction> BeginTransactionAsync(
        ISnowflakeDbConnection conn,
        CancellationToken cancellationToken) =>
        await UnderlyingConnection(conn).BeginTransactionAsync(cancellationToken).ConfigureAwait(false);

    private int CountRows(ISnowflakeDbConnection conn, string tableName)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = $"Select * from {tableName}";
        using var reader = cmd.ExecuteReader();

        var row = 0;
        while (reader.Read())
        {
            var dataDate = reader.GetDateTime(0);
            var dataInt = reader.GetInt32(1);
            Output.WriteLine(string.Format(CultureInfo.InvariantCulture, "Row {0}: {1}, {2}", row, dataDate, dataInt));
            row++;
        }

        return row;
    }

    private static void ExecuteNonQuery(ISnowflakeDbConnection conn, string commandText)
    {
        using var cmd = conn.CreateCommand();
        cmd.CommandText = commandText;
        cmd.ExecuteNonQuery();
    }

    private static void AssertThrowsSnowflakeDbException(Action action)
    {
        var exception = Assert.ThrowsAny<Exception>(action);
        var expectedType = TestDriver.Current == TestDriver.Kind.Old
            ? typeof(OldSnowflakeDbException)
            : typeof(NewSnowflakeDbException);
        Assert.Equal(expectedType, exception.GetType());
    }
}
