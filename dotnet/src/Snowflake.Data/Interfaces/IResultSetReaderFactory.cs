using System.Data.Common;
using Snowflake.Data.Core.Handles;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Interfaces;

/// <summary>
/// Creates the <see cref="DbDataReader"/> that the statement layer returns over an sf_core result.
/// </summary>
internal interface IResultSetReaderFactory
{
    /// <summary>
    /// Creates a reader over the rows of an executed query.
    /// </summary>
    /// <param name="resultSetHandle">The sf_core result set; the reader takes ownership and releases it when it is closed.</param>
    /// <param name="stream">The Arrow C stream sf_core returned for the result set.</param>
    /// <param name="descriptor">The column metadata and rows-affected count of the result.</param>
    /// <param name="recordsAffected">The value the reader reports as <see cref="DbDataReader.RecordsAffected"/>.</param>
    DbDataReader Create(NativeResultSetHandle resultSetHandle, ArrowArrayStreamPtr stream,
        ResultSetDescriptor descriptor, int recordsAffected);

    /// <summary>
    /// Creates a reader that reports the schema of a described query and no rows. The reader owns no sf_core
    /// result set, because describing a query does not create one.
    /// </summary>
    /// <param name="stream">The Arrow C stream sf_core returned for the described query.</param>
    /// <param name="descriptor">The column metadata of the described query.</param>
    /// <param name="recordsAffected">The value the reader reports as <see cref="DbDataReader.RecordsAffected"/>.</param>
    DbDataReader CreateForSchemaOnly(ArrowArrayStreamPtr stream, ResultSetDescriptor descriptor, int recordsAffected);
}
