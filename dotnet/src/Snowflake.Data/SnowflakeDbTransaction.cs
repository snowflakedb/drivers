using System.Data;
using System.Data.Common;

namespace Snowflake.Data;

public sealed class SnowflakeDbTransaction : DbTransaction
{
    private readonly DbConnection _connection;

    internal SnowflakeDbTransaction(DbConnection connection, IsolationLevel isolationLevel)
    {
        _connection = connection;
        IsolationLevel = isolationLevel;
    }

    public override IsolationLevel IsolationLevel { get; }

    protected override DbConnection? DbConnection => _connection;

    public override void Commit() => throw new NotImplementedException();

    public override void Rollback() => throw new NotImplementedException();
}
