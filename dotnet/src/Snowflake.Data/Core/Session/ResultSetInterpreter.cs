using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.Session;

internal sealed class ResultSetInterpreter : IResultSetInterpreter
{
    /// <summary>The shared stateless instance.</summary>
    internal static IResultSetInterpreter Instance { get; } = new ResultSetInterpreter();

    public int CalculateUpdateCount(ResultSetDescriptor resultSetDescriptor) =>
        resultSetDescriptor.HasRowsAffected ? ToInt32OrMinusOne(resultSetDescriptor.RowsAffected) : 0x0;

    public int CalculateRecordsAffected(ResultSetDescriptor resultSetDescriptor) =>
        resultSetDescriptor.HasRowsAffected ? ToInt32OrMinusOne(resultSetDescriptor.RowsAffected) : -0x1;

    private static int ToInt32OrMinusOne(long rowsAffected) =>
        rowsAffected is < 0x0 or > int.MaxValue ? -1 : (int)rowsAffected;
}
