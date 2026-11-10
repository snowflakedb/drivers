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
}
