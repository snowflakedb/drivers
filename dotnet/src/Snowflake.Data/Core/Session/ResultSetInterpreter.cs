using Snowflake.Data.Interfaces;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.Session;

// TODO(SNOW-4382583): an array-bound DML statement reports its count as the sum of the result row's columns.
internal sealed class ResultSetInterpreter : IResultSetInterpreter
{
    /// <summary>The shared stateless instance.</summary>
    internal static IResultSetInterpreter Instance { get; } = new ResultSetInterpreter();

    public int CalculateUpdateCount(ResultSetDescriptor resultSetDescriptor) =>
        resultSetDescriptor.HasRowsAffected ? ToInt32OrMinusOne(resultSetDescriptor.RowsAffected) : 0x0;

    public int CalculateRecordsAffected(ResultSetDescriptor resultSetDescriptor) =>
        resultSetDescriptor.HasRowsAffected ? ToInt32OrMinusOne(resultSetDescriptor.RowsAffected) : -0x1;

    public bool TryGetOrdinal(ResultSetDescriptor resultSetDescriptor, string name, out int ordinal)
    {
        var columns = resultSetDescriptor.Columns;
        for (ordinal = 0; ordinal < columns.Count; ordinal++)
        {
            if (string.Equals(columns[ordinal].Name, name, StringComparison.Ordinal))
                return true;
        }

        for (ordinal = 0; ordinal < columns.Count; ordinal++)
        {
            if (string.Equals(columns[ordinal].Name, name, StringComparison.OrdinalIgnoreCase))
                return true;
        }

        ordinal = -1;
        return false;
    }

    private static int ToInt32OrMinusOne(long rowsAffected) =>
        rowsAffected is < 0x0 or > int.MaxValue ? -1 : (int)rowsAffected;
}
