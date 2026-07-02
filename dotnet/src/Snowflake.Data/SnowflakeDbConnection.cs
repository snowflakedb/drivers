using System.Data;
using System.Data.Common;
using System.Diagnostics.CodeAnalysis;

namespace Snowflake.Data;

public sealed class SnowflakeDbConnection : DbConnection
{
    private string _connectionString = string.Empty;
    private ConnectionState _state = ConnectionState.Closed;

    public SnowflakeDbConnection()
    {
    }

    public SnowflakeDbConnection(string connectionString)
    {
        _connectionString = connectionString;
    }

    [AllowNull]
    public override string ConnectionString
    {
        get => _connectionString;
        set => _connectionString = value ?? string.Empty;
    }

    public override string Database => throw new NotImplementedException();

    public override string DataSource => throw new NotImplementedException();

    public override string ServerVersion => throw new NotImplementedException();

    public override ConnectionState State => _state;

    public override void ChangeDatabase(string databaseName) =>
        throw new NotImplementedException();

    public override void Close()
    {
        _state = ConnectionState.Closed;
    }

    public override void Open() =>
        throw new NotImplementedException();

    public override Task OpenAsync(CancellationToken cancellationToken) =>
        throw new NotImplementedException();

    protected override DbTransaction BeginDbTransaction(IsolationLevel isolationLevel) =>
        throw new NotImplementedException();

    protected override DbCommand CreateDbCommand() =>
        new SnowflakeDbCommand { Connection = this };
}
