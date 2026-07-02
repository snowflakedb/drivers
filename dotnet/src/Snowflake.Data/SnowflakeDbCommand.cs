using System.Data;
using System.Data.Common;
using System.Diagnostics.CodeAnalysis;

namespace Snowflake.Data;

public sealed class SnowflakeDbCommand : DbCommand
{
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

    public override int ExecuteNonQuery() =>
        throw new NotImplementedException();

    public override object? ExecuteScalar() =>
        throw new NotImplementedException();

    public override void Prepare() =>
        throw new NotImplementedException();

    protected override DbParameter CreateDbParameter() =>
        new SnowflakeDbParameter();

    protected override DbDataReader ExecuteDbDataReader(CommandBehavior behavior) =>
        throw new NotImplementedException();
}
