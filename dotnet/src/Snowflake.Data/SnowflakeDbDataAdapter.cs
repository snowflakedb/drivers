using System.Data;
using System.Data.Common;
// ReSharper disable UnusedMember.Global
// ReSharper disable UnusedParameter.Local

namespace Snowflake.Data;

public class SnowflakeDbDataAdapter : DbDataAdapter, IDbDataAdapter
{
    public SnowflakeDbDataAdapter()
    {
        throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
    }

    public SnowflakeDbDataAdapter(SnowflakeDbCommand selectCommand) : this()
    {
        throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
    }

    public SnowflakeDbDataAdapter(string selectCommandText, SnowflakeDbConnection selectConnection) : this()
    {
        throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
    }

    IDbCommand? IDbDataAdapter.DeleteCommand { get; set; }

    IDbCommand? IDbDataAdapter.InsertCommand { get; set; }

    new public SnowflakeDbCommand SelectCommand
    {
        get =>
            throw new NotImplementedException(
                "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

        set => throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
    }

    IDbCommand? IDbDataAdapter.SelectCommand
    {
        get => throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
        set => throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
    }

    IDbCommand? IDbDataAdapter.UpdateCommand { get; set; }
}
