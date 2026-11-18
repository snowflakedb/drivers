#if NETFRAMEWORK
using System.Data;
using System.Data.Common;

namespace Snowflake.Data.Tests.Compatibility;
// TODO will be used in the future
public static class FrameworkShims
{
    public static Task<DbTransaction> BeginTransactionAsync(this DbConnection connection)
        => Task.FromResult(connection.BeginTransaction());

    public static Task<DbTransaction> BeginTransactionAsync(this DbConnection connection, IsolationLevel isolationLevel)
        => Task.FromResult(connection.BeginTransaction(isolationLevel));

    public static Task CloseAsync(this DbConnection connection)
    {
        connection.Close();
        return Task.CompletedTask;
    }

    public static Task CloseAsync(this DbDataReader reader)
    {
        reader.Close();
        return Task.CompletedTask;
    }

    public static Task ChangeDatabaseAsync(this DbConnection connection, string databaseName)
    {
        connection.ChangeDatabase(databaseName);
        return Task.CompletedTask;
    }

    public static Task CommitAsync(this DbTransaction transaction)
    {
        transaction.Commit();
        return Task.CompletedTask;
    }

    public static Task RollbackAsync(this DbTransaction transaction)
    {
        transaction.Rollback();
        return Task.CompletedTask;
    }

    public static Task<DbTransaction> BeginTransactionAsync(this DbConnection connection, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        return connection.BeginTransactionAsync();
    }

    public static Task<DbTransaction> BeginTransactionAsync(
        this DbConnection connection,
        IsolationLevel isolationLevel,
        CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        return connection.BeginTransactionAsync(isolationLevel);
    }

    public static Task CloseAsync(this DbDataReader reader, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        return reader.CloseAsync();
    }

    public static Task DisposeAsync(this DbDataReader reader)
    {
        reader.Dispose();
        return Task.CompletedTask;
    }

    public static Task CommitAsync(this DbTransaction transaction, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        return transaction.CommitAsync();
    }

    public static Task RollbackAsync(this DbTransaction transaction, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        return transaction.RollbackAsync();
    }

    public static bool TryDequeue<T>(this Queue<T> queue, out T element)
    {
        element = default!;
        if (queue.Count == 0)
            return false;

        element = queue.Dequeue();
        return true;
    }

    public static Task CancelAsync(this CancellationTokenSource source)
    {
        source.Cancel();
        return Task.CompletedTask;
    }
}
#endif
