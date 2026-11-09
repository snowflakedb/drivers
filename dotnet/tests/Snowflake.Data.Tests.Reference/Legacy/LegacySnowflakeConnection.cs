using System.Data;
using System.Data.Common;
using System.Diagnostics.CodeAnalysis;
using System.Security;
using OldQueryStatus = Snowflake.Data.Core.QueryStatus;
using OldSnowflakeDbConnection = Snowflake.Data.Client.SnowflakeDbConnection;

namespace Snowflake.Data.Tests.Reference.Legacy;

sealed class LegacySnowflakeConnection : ISnowflakeDbConnection
{
    private readonly OldSnowflakeDbConnection _inner;

    public LegacySnowflakeConnection(string connectionString)
    {
        _inner = new OldSnowflakeDbConnection { ConnectionString = connectionString };
    }

    [AllowNull]
    public string ConnectionString
    {
        get => _inner.ConnectionString;
        set => _inner.ConnectionString = value ?? string.Empty;
    }

    public int ConnectionTimeout => _inner.ConnectionTimeout;

    public string Database => _inner.Database;

    public ConnectionState State => _inner.State;

    public SecureString Password
    {
        get => _inner.Password;
        set => _inner.Password = value;
    }

    public SecureString Passcode
    {
        get => _inner.Passcode;
        set => _inner.Passcode = value;
    }

    public SecureString OAuthClientSecret
    {
        get => _inner.OAuthClientSecret;
        set => _inner.OAuthClientSecret = value;
    }

    public SecureString Token
    {
        get => _inner.Token;
        set => _inner.Token = value;
    }

    public IDbTransaction BeginTransaction() => _inner.BeginTransaction();

    public IDbTransaction BeginTransaction(IsolationLevel isolationLevel) => _inner.BeginTransaction(isolationLevel);

    public void ChangeDatabase(string databaseName) => _inner.ChangeDatabase(databaseName);

    public void Close() => _inner.Close();

    public DbCommand CreateCommand() => _inner.CreateCommand();

    IDbCommand IDbConnection.CreateCommand() => _inner.CreateCommand();

    public void Open() => _inner.Open();

    public void Dispose() => _inner.Dispose();

    public Task OpenAsync(CancellationToken cancellationToken = default) => _inner.OpenAsync(cancellationToken);

    public bool IsOpen() => _inner.IsOpen();

    public void PreventPooling() => _inner.PreventPooling();

    public Task CloseAsync(CancellationToken cancellationToken) => _inner.CloseAsync(cancellationToken);

    public bool IsStillRunning(QueryStatus status) => _inner.IsStillRunning((OldQueryStatus)status);

    public bool IsAnError(QueryStatus status) => _inner.IsAnError((OldQueryStatus)status);
}
