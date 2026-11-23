extern alias OldDriver;
using System.Data.Common;
using System.Diagnostics.CodeAnalysis;
using System.Security;
using Snowflake.Data.Interfaces;
using OldQueryStatus = OldDriver::Snowflake.Data.Core.QueryStatus;
using OldSnowflakeDbConnection = OldDriver::Snowflake.Data.Client.SnowflakeDbConnection;

namespace Snowflake.Data.Tests.E2E.Legacy;

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

    public void Open()
    {
        _inner.Open();
        using var command = CreateCommand();
        command.CommandText = $"ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = {Environment.GetEnvironmentVariable("QUERY_RESULT_FORMAT") ?? "ARROW"}";
        command.ExecuteNonQuery();
    }

    public void Dispose() => _inner.Dispose();

    public async Task OpenAsync(CancellationToken cancellationToken = default)
    {
        await _inner.OpenAsync(cancellationToken).ConfigureAwait(false);
        using var command = CreateCommand();
        command.CommandText = $"ALTER SESSION SET DOTNET_QUERY_RESULT_FORMAT = {Environment.GetEnvironmentVariable("QUERY_RESULT_FORMAT") ?? "ARROW"}";
        await command.ExecuteNonQueryAsync(cancellationToken).ConfigureAwait(false);
    }

    public bool IsOpen() => _inner.IsOpen();

    public void PreventPooling() => _inner.PreventPooling();

    public Task CloseAsync(CancellationToken cancellationToken) => _inner.CloseAsync(cancellationToken);

    public bool IsStillRunning(QueryStatus status) => _inner.IsStillRunning((OldQueryStatus)status);

    public bool IsAnError(QueryStatus status) => _inner.IsAnError((OldQueryStatus)status);
}
