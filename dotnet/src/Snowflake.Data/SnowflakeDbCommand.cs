using System.Data;
using System.Data.Common;
using System.Diagnostics.CodeAnalysis;
using Snowflake.Data.Proto;

namespace Snowflake.Data;

public sealed class SnowflakeDbCommand : DbCommand
{
    private StatementHandle? _stmtHandle;
    private SnowflakeDbDataReader? _openReader;
    private bool _disposed;

    [AllowNull]
    public override string CommandText { get; set; } = string.Empty;

    public override int CommandTimeout { get; set; } = 30;

    public override CommandType CommandType { get; set; } = CommandType.Text;

    public override bool DesignTimeVisible { get; set; }

    public override UpdateRowSource UpdatedRowSource { get; set; }

    protected override DbConnection? DbConnection { get; set; }

    protected override DbParameterCollection DbParameterCollection { get; } =
        new SnowflakeDbParameterCollection();

    protected override DbTransaction? DbTransaction { get; set; }

    public override void Cancel() =>
        throw new NotImplementedException();

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override int ExecuteNonQuery()
    {
        var (driver, connHandle) = GetDriverAndConnection();
        var stmtHandle = AllocateStatement(driver, connHandle);

        try
        {
            var response = driver.StatementExecuteQuery(
                new StatementExecuteQueryRequest { StmtHandle = stmtHandle });

            var resultSet = response.Single
                ?? throw new InvalidOperationException("Expected single-statement result.");

            var descriptor = resultSet.ResultDescriptor;

            // Release the result set handle directly — no need to allocate an Arrow stream for non-query commands.
            driver.ResultSetRelease(new ResultSetReleaseRequest
            {
                ResultSetHandle = resultSet.ResultSetHandle,
            });

            return descriptor.HasRowsAffected ? (int)descriptor.RowsAffected : 0;
        }
        finally
        {
            ReleaseStatement(driver, stmtHandle);
        }
    }

    public override object? ExecuteScalar() =>
        throw new NotImplementedException();

    public override void Prepare() =>
        throw new NotImplementedException();

    protected override DbParameter CreateDbParameter() =>
        new SnowflakeDbParameter();

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    protected override DbDataReader ExecuteDbDataReader(CommandBehavior behavior)
    {
        var (driver, connHandle) = GetDriverAndConnection();
        _stmtHandle = AllocateStatement(driver, connHandle);

        ExecuteQueryResponse response;
        try
        {
            response = driver.StatementExecuteQuery(
                new StatementExecuteQueryRequest { StmtHandle = _stmtHandle });
        }
        catch
        {
            ReleaseStatement(driver, _stmtHandle);
            _stmtHandle = null;
            throw;
        }

        if (response.ResultCase != ExecuteQueryResponse.ResultOneofCase.Single)
        {
            ReleaseStatement(driver, _stmtHandle);
            _stmtHandle = null;
            throw new NotSupportedException("Multi-statement results are not supported.");
        }

        var resultSet = response.Single;
        var descriptor = resultSet.ResultDescriptor;
        var rsHandle = resultSet.ResultSetHandle;

        // Fetch the Arrow stream pointer.
        ResultSetGetStreamResponse streamResponse;
        try
        {
            streamResponse = driver.ResultSetGetStream(
                new ResultSetGetStreamRequest { ResultSetHandle = rsHandle });
        }
        catch
        {
            driver.ResultSetRelease(new ResultSetReleaseRequest { ResultSetHandle = rsHandle });
            ReleaseStatement(driver, _stmtHandle);
            _stmtHandle = null;
            throw;
        }

        var reader = new SnowflakeDbDataReader(driver, rsHandle, streamResponse.Stream, descriptor);
        _openReader = reader;
        return reader;
    }

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    protected override void Dispose(bool disposing)
    {
        if (_disposed)
        {
            base.Dispose(disposing);
            return;
        }

        if (disposing)
        {
            // Force-close open reader before releasing the statement.
            if (_openReader is { IsClosed: false })
            {
                _openReader.Close();
            }
            _openReader = null;

            if (_stmtHandle is not null && DbConnection is SnowflakeDbConnection conn)
            {
                var driver = conn.Driver;
                ReleaseStatement(driver, _stmtHandle);
                _stmtHandle = null;
            }
        }

        _disposed = true;
        base.Dispose(disposing);
    }

    private StatementHandle AllocateStatement(IDatabaseDriverService driver, ConnectionHandle connHandle)
    {
        var stmtHandle = driver.StatementNew(
            new StatementNewRequest { ConnHandle = connHandle }).StmtHandle;

        driver.StatementSetSqlQuery(
            new StatementSetSqlQueryRequest { StmtHandle = stmtHandle, Query = CommandText });

        return stmtHandle;
    }

    private static void ReleaseStatement(IDatabaseDriverService driver, StatementHandle stmtHandle)
    {
        try
        {
            driver.StatementRelease(new StatementReleaseRequest { StmtHandle = stmtHandle });
        }
        catch
        {
            // Best-effort release; swallow exceptions during cleanup.
        }
    }

    private (IDatabaseDriverService Driver, ConnectionHandle ConnHandle) GetDriverAndConnection()
    {
        if (DbConnection is not SnowflakeDbConnection conn)
        {
            throw new InvalidOperationException("Command is not associated with a SnowflakeDbConnection.");
        }

        if (conn.State != ConnectionState.Open)
        {
            throw new InvalidOperationException("Connection is not open.");
        }

        return (conn.Driver, conn.ConnHandle);
    }
}
