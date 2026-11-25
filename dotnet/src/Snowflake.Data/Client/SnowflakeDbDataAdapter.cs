using System.Data;
using System.Data.Common;

namespace Snowflake.Data.Client;

public class SnowflakeDbDataAdapter : DbDataAdapter, IDbDataAdapter
{
    private SnowflakeDbCommand? _selectCommand;

    public SnowflakeDbDataAdapter()
    {
    }

    public SnowflakeDbDataAdapter(SnowflakeDbCommand selectCommand) : this()
    {
        _selectCommand = selectCommand;
    }

    public SnowflakeDbDataAdapter(string selectCommandText, SnowflakeDbConnection selectConnection) : this()
    {
        _selectCommand = new SnowflakeDbCommand(selectConnection, selectCommandText);
    }

    IDbCommand? IDbDataAdapter.DeleteCommand { get; set; }

    IDbCommand? IDbDataAdapter.InsertCommand { get; set; }

    public new SnowflakeDbCommand? SelectCommand
    {
        get => _selectCommand;
        set => _selectCommand = value;
    }

    IDbCommand? IDbDataAdapter.SelectCommand
    {
        get => _selectCommand;
        set => _selectCommand = (SnowflakeDbCommand)value!;
    }

    IDbCommand? IDbDataAdapter.UpdateCommand { get; set; }
}
