using System.Data;
using System.Data.Common;
using System.Diagnostics.CodeAnalysis;
using Snowflake.Data.Proto;

namespace Snowflake.Data;

public sealed class SnowflakeDbConnection : DbConnection
{
    private readonly IDatabaseDriverService _driver;
    private string _connectionString = string.Empty;
    private ConnectionState _state = ConnectionState.Closed;
    private DatabaseHandle? _dbHandle;
    private ConnectionHandle? _connHandle;

    internal SnowflakeDbConnection(ICoreTransport transport)
    {
        _driver = new DatabaseDriverServiceClient(transport);
    }

    internal SnowflakeDbConnection(ICoreTransport transport, string connectionString)
        : this(transport)
    {
        _connectionString = connectionString;
    }

    public SnowflakeDbConnection()
        : this(NativeCoreTransport.Instance)
    {
    }

    public SnowflakeDbConnection(string connectionString)
        : this(NativeCoreTransport.Instance)
    {
        _connectionString = connectionString;
    }

    [AllowNull]
    public override string ConnectionString
    {
        get => _connectionString;
        set => _connectionString = value ?? string.Empty;
    }

    public override string Database => string.Empty;

    public override string DataSource => string.Empty;

    public override string ServerVersion => string.Empty;

    public override ConnectionState State => _state;

    public override void ChangeDatabase(string databaseName) =>
        throw new NotSupportedException();

    // TODO this implementation is just PoC and will undergo heavy refactoring.
    public override void Open()
    {
        if (_state == ConnectionState.Open)
        {
            throw new InvalidOperationException("Connection is already open.");
        }

        _state = ConnectionState.Connecting;

        var parser = new ConnectionStringParser(_connectionString);

        _dbHandle = _driver.DatabaseNew(new DatabaseNewRequest()).DbHandle;
        _driver.DatabaseInit(new DatabaseInitRequest { DbHandle = _dbHandle });

        _connHandle = _driver.ConnectionNew(new ConnectionNewRequest()).ConnHandle;

        var setOptionsReq = new ConnectionSetOptionsRequest
        {
            ConnHandle = _connHandle,
            NoConnectionDetails = parser.IsEmpty,
        };
        setOptionsReq.Options.Add(parser.ToProtoOptions());
        _driver.ConnectionSetOptions(setOptionsReq);

        _driver.ConnectionInit(new ConnectionInitRequest
        {
            ConnHandle = _connHandle,
            DbHandle = _dbHandle,
            WrapperIdentity = BuildWrapperIdentity(),
        });

        _state = ConnectionState.Open;
    }

    public override async Task OpenAsync(CancellationToken cancellationToken)
    {
        if (_state == ConnectionState.Open)
        {
            throw new InvalidOperationException("Connection is already open.");
        }

        _state = ConnectionState.Connecting;

        var parser = new ConnectionStringParser(_connectionString);

        var dbNewResp = await _driver.DatabaseNewAsync(new DatabaseNewRequest(), cancellationToken);
        _dbHandle = dbNewResp.DbHandle;
        await _driver.DatabaseInitAsync(new DatabaseInitRequest { DbHandle = _dbHandle }, cancellationToken);

        var connNewResp = await _driver.ConnectionNewAsync(new ConnectionNewRequest(), cancellationToken);
        _connHandle = connNewResp.ConnHandle;

        var setOptionsReq = new ConnectionSetOptionsRequest
        {
            ConnHandle = _connHandle,
            NoConnectionDetails = parser.IsEmpty,
        };
        setOptionsReq.Options.Add(parser.ToProtoOptions());
        await _driver.ConnectionSetOptionsAsync(setOptionsReq, cancellationToken);

        await _driver.ConnectionInitAsync(new ConnectionInitRequest
        {
            ConnHandle = _connHandle,
            DbHandle = _dbHandle,
            WrapperIdentity = BuildWrapperIdentity(),
        }, cancellationToken);

        _state = ConnectionState.Open;
    }

    public override void Close()
    {
        if (_state == ConnectionState.Closed)
        {
            return;
        }

        if (_connHandle is not null)
        {
            _driver.ConnectionClose(new ConnectionCloseRequest { ConnHandle = _connHandle });
            _driver.ConnectionRelease(new ConnectionReleaseRequest { ConnHandle = _connHandle });
            _connHandle = null;
        }

        if (_dbHandle is not null)
        {
            _driver.DatabaseRelease(new DatabaseReleaseRequest { DbHandle = _dbHandle });
            _dbHandle = null;
        }

        _state = ConnectionState.Closed;
    }

    protected override DbTransaction BeginDbTransaction(IsolationLevel isolationLevel) =>
        throw new NotImplementedException();

    protected override DbCommand CreateDbCommand() =>
        new SnowflakeDbCommand { Connection = this };

    private static WrapperIdentity BuildWrapperIdentity() => new()
    {
        DriverName = "Snowflake .NET Driver",
        DriverVersion = "0.1.0",
        LanguageRuntime = ".NET",
        LanguageVersion = Environment.Version.ToString(),
    };
}
