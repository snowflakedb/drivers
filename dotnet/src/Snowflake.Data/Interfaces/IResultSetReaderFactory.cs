using System.Data.Common;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Interfaces;

/// <summary>
/// Creates the <see cref="DbDataReader"/> that the statement layer returns over an sf_core result.
/// </summary>
// TODO(SNOW-4429197): a multi-statement result needs a reader over several results; it plugs in here.
internal interface IResultSetReaderFactory
{
    /// <summary>
    /// Creates a reader over the rows of an executed query.
    /// </summary>
    /// <param name="batches">The result's batches; the reader takes ownership and disposes it when it is closed.</param>
    /// <param name="descriptor">The column metadata and rows-affected count of the result.</param>
    /// <param name="recordsAffected">The value the reader reports as <see cref="DbDataReader.RecordsAffected"/>.</param>
    DbDataReader Create(IRecordBatchSource batches, ResultSetDescriptor descriptor, int recordsAffected);

    /// <summary>
    /// Creates a reader that reports the schema of a described query and no rows.
    /// </summary>
    /// <param name="descriptor">The column metadata of the described query.</param>
    DbDataReader CreateForSchemaOnly(ResultSetDescriptor descriptor);
}
