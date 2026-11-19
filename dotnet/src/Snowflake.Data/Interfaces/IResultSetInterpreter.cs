using Snowflake.Data.Proto;

namespace Snowflake.Data.Interfaces;

/// <summary>
/// Turns the rows-affected count sf_core reports for a result into the values ADO.NET exposes.
/// </summary>
internal interface IResultSetInterpreter
{
    /// <summary>
    /// The value <see cref="System.Data.Common.DbCommand.ExecuteNonQuery"/> returns: sf_core's count,
    /// 0 when sf_core reports none, -1 when it does not fit in an <see cref="int"/>.
    /// </summary>
    int CalculateUpdateCount(ResultSetDescriptor resultSetDescriptor);

    /// <summary>
    /// The value <see cref="System.Data.Common.DbDataReader.RecordsAffected"/> reports: sf_core's count,
    /// -1 when sf_core reports none or it does not fit in an <see cref="int"/>.
    /// </summary>
    int CalculateRecordsAffected(ResultSetDescriptor resultSetDescriptor);

    /// <summary>
    /// Finds the first column named with given name.
    /// </summary>
    /// <param name="resultSetDescriptor">Descriptor for given result set.</param>
    /// <param name="name">The column name to look up.</param>
    /// <param name="ordinal">The column's ordinal when found; otherwise -1.</param>
    /// <returns>False when no column has that name.</returns>
    internal bool TryGetOrdinal(ResultSetDescriptor resultSetDescriptor, string name, out int ordinal);
}
