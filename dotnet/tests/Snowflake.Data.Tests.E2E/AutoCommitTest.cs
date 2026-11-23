using System.Data.Common;
using System.Globalization;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.E2E;

[Trait("Category", "E2E")]
public class AutoCommitTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public AutoCommitTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should discard uncommitted inserts on rollback
    [SnowflakeFact]
    public async Task ShouldDiscardUncommittedInsertsOnRollbackAsync()
    {
        Skip.FutureMilestone("BeginTransaction, Commit and Rollback are not implemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var writer = Fixture.Factory.Create(Output);
        await writer.OpenAsync(ct).ConfigureAwait(false);

        // And a transient table exists in the test schema
        var tableName = $"AUTO_COMMIT_ROLLBACK_{Guid.NewGuid():N}";
        await ExecuteAsync(writer, $"CREATE TRANSIENT TABLE {tableName} (id INT)", ct).ConfigureAwait(false);
        try
        {
            // When the writer disables autocommit, inserts a row, and rolls back
            using (var transaction = writer.BeginTransaction())
            {
                Assert.Equal(1, await InsertRowAsync(writer, transaction, tableName, ct).ConfigureAwait(false));
                transaction.Rollback();
            }

            // Then a reader session sees zero rows
            Assert.Equal(0L, await CountRowsFromReaderSessionAsync(tableName, ct).ConfigureAwait(false));
        }
        finally
        {
            await ExecuteAsync(writer, $"DROP TABLE IF EXISTS {tableName}", ct).ConfigureAwait(false);
        }
    }

    // Scenario: should publish committed inserts to other sessions
    [SnowflakeFact]
    public async Task ShouldPublishCommittedInsertsToOtherSessionsAsync()
    {
        Skip.FutureMilestone("BeginTransaction, Commit and Rollback are not implemented");
        var ct = TestContext.Current.CancellationToken;

        // Given Snowflake client is logged in
        using var writer = Fixture.Factory.Create(Output);
        await writer.OpenAsync(ct).ConfigureAwait(false);

        // And a transient table exists in the test schema
        var tableName = $"AUTO_COMMIT_COMMIT_{Guid.NewGuid():N}";
        await ExecuteAsync(writer, $"CREATE TRANSIENT TABLE {tableName} (id INT)", ct).ConfigureAwait(false);
        try
        {
            // When the writer disables autocommit, inserts a row, and commits
            using (var transaction = writer.BeginTransaction())
            {
                Assert.Equal(1, await InsertRowAsync(writer, transaction, tableName, ct).ConfigureAwait(false));
                transaction.Commit();
            }

            // Then a reader session sees one row
            Assert.Equal(1L, await CountRowsFromReaderSessionAsync(tableName, ct).ConfigureAwait(false));
        }
        finally
        {
            await ExecuteAsync(writer, $"DROP TABLE IF EXISTS {tableName}", ct).ConfigureAwait(false);
        }
    }

    private static async Task ExecuteAsync(ISnowflakeDbConnection connection, string sql, CancellationToken cancellationToken)
    {
        using var cmd = connection.CreateCommand();
        cmd.CommandText = sql;
        await cmd.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
    }

    private static async Task<int> InsertRowAsync(ISnowflakeDbConnection connection, IDbTransaction transaction, string tableName, CancellationToken cancellationToken)
    {
        using var cmd = connection.CreateCommand();
        cmd.Transaction = (DbTransaction)transaction;
        cmd.CommandText = $"INSERT INTO {tableName} (id) VALUES (1)";
        return await cmd.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
    }

    private async Task<long> CountRowsFromReaderSessionAsync(string tableName, CancellationToken cancellationToken)
    {
        using var reader = Fixture.Factory.Create(Output);
        await reader.OpenAsync(cancellationToken).ConfigureAwait(false);
        using var cmd = reader.CreateCommand();
        cmd.CommandText = $"SELECT COUNT(*) FROM {tableName}";
        return Convert.ToInt64(await cmd.ExecuteScalarAsync(cancellationToken).ConfigureAwait(false), CultureInfo.InvariantCulture);
    }
}
