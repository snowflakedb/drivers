using System.Data;
using Snowflake.Data.Client;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Interfaces;

/// <summary>
/// Reads the rows of one result for a <see cref="SnowflakeDbDataReader"/>: the cursor, the batches and the column
/// values. Not thread-safe. Failures are thrown as <see cref="SnowflakeException"/>, which the reader converts at
/// its public boundary.
/// </summary>
/// <remarks>
/// Column metadata (<see cref="FieldCount"/>, <see cref="GetName"/>, <see cref="GetDataTypeName"/>,
/// <see cref="GetFieldType"/>, <see cref="GetOrdinal"/>) stays available after <see cref="Close"/>; the cursor and
/// row data do not. The value reads (<see cref="IsDBNull"/>, <see cref="GetValue"/>, <see cref="GetFieldValue{T}"/>,
/// <see cref="GetBytes"/>, <see cref="GetChars"/>) use the current row and throw
/// <see cref="SFError.DATA_READER_NO_CURRENT_ROW"/> when there is none. A column of a Snowflake type the driver does
/// not read yet throws <see cref="NotImplementedException"/> from <see cref="GetFieldType"/> and from every value
/// read except <see cref="IsDBNull"/>.
/// </remarks>
internal interface ISnowflakeResultSet : IDisposable
{
    /// <summary>The Snowflake query ID of the result, or an empty string when sf_core reported none.</summary>
    string QueryId { get; }

    /// <summary>The rows-affected count reported for the statement that produced the result.</summary>
    int RecordsAffected { get; }

    /// <summary>The number of columns.</summary>
    int FieldCount { get; }

    /// <summary>Whether <see cref="Close"/> has run.</summary>
    bool IsClosed { get; }

    /// <summary>
    /// Whether the result has at least one row; false once closed. Without a row count from sf_core, the first
    /// call fetches up to the first batch that has rows, which <see cref="Read"/> then returns.
    /// </summary>
    bool GetHasRows();

    /// <summary>Moves to the next row, fetching the next batch on the calling thread when the current one is used up.</summary>
    /// <returns>False at the end of the rows.</returns>
    bool Read();

    /// <summary>Moves to the next row of the current batch, without I/O.</summary>
    /// <returns>False when the current batch has no next row, or no batch has been fetched yet.</returns>
    bool TryAdvanceWithinBatch();

    /// <summary>
    /// Fetches the next batch that has rows and moves to its first row. Call it once
    /// <see cref="TryAdvanceWithinBatch"/> has returned false; rows left in the current batch are skipped.
    /// </summary>
    /// <returns>False at the end of the rows.</returns>
    Task<bool> ReadNextBatchAsync(CancellationToken cancellationToken);

    /// <summary>The ordinal of the first column named exactly <paramref name="name"/>, else of the first matching it ignoring case.</summary>
    int GetOrdinal(string name);

    /// <summary>The name of the column.</summary>
    string GetName(int ordinal);

    /// <summary>The Snowflake type of the column in upper case.</summary>
    string GetDataTypeName(int ordinal);

    /// <summary>The CLR type <see cref="GetValue"/> returns for the column.</summary>
    Type GetFieldType(int ordinal);

    /// <summary>
    /// The description of the columns, built on the first call and returned again afterwards. Available after
    /// <see cref="Close"/>.
    /// </summary>
    DataTable GetSchemaTable();

    /// <summary>Whether the column of the current row is NULL.</summary>
    bool IsDBNull(int ordinal);

    /// <summary>The value of the column of the current row, or <see cref="DBNull.Value"/>.</summary>
    object GetValue(int ordinal);

    /// <summary>
    /// The value of the column of the current row as a <typeparamref name="T"/>, read by the column's converter.
    /// Read as <see cref="object"/> it is the same as <see cref="GetValue"/>. Otherwise a <typeparamref name="T"/>
    /// the column cannot be read as, a value that cannot be represented, and a NULL value throw
    /// <see cref="SFError.INVALID_DATA_CONVERSION"/>; a NULL TEXT value read as a string is null.
    /// </summary>
    T? GetFieldValue<T>(int ordinal);

    /// <summary>Copies bytes of the column of the current row, following <see cref="System.Data.Common.DbDataReader.GetBytes"/>.</summary>
    long GetBytes(int ordinal, long dataOffset, byte[]? buffer, int bufferOffset, int length);

    /// <summary>Copies characters of the column of the current row, following <see cref="System.Data.Common.DbDataReader.GetChars"/>.</summary>
    long GetChars(int ordinal, long dataOffset, char[]? buffer, int bufferOffset, int length);

    /// <summary>Releases the batches and stops the download of those not reached. Safe to call more than once.</summary>
    void Close();
}
