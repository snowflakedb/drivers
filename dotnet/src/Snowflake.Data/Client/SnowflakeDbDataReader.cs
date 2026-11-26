using System.Collections;
using System.Data;
using System.Data.Common;
using System.Runtime.CompilerServices;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Compatibility;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Telemetry;

namespace Snowflake.Data.Client;

/// <summary>
/// Reads the rows of a Snowflake query result, one row at a time.
/// </summary>
/// <remarks>
/// <para>
/// A reader is not thread-safe. Rows arrive in batches that are downloaded as <see cref="Read"/> and
/// <see cref="ReadAsync"/> reach them. The command timeout does not apply to reading rows.
/// </para>
/// <para>
/// Closing the reader stops the download of batches it has not reached. Column metadata
/// (<see cref="FieldCount"/>, <see cref="GetName"/>, <see cref="GetDataTypeName"/>,
/// <see cref="GetFieldType"/>) stays available after the reader is closed; row data does not.
/// </para>
/// </remarks>
public sealed class SnowflakeDbDataReader : DbDataReader, ISnowflakeDbDataReader
{
    private static readonly Task<bool> RowAvailable = Task.FromResult(true);

    private readonly ISnowflakeResultSet _resultSet;
    private readonly ILogger<SnowflakeDbDataReader> _logger;

    /// <param name="resultSet">The result to read; the reader takes ownership and closes it on <see cref="Close"/>.</param>
    /// <param name="logger">Receives the reader's diagnostics.</param>
    internal SnowflakeDbDataReader(ISnowflakeResultSet resultSet, ILogger<SnowflakeDbDataReader> logger)
    {
        _resultSet = resultSet;
        _logger = logger;
    }

    public override int FieldCount => _resultSet.FieldCount;

    public override int RecordsAffected => _resultSet.RecordsAffected;

    public override bool IsClosed => _resultSet.IsClosed;

    /// <summary>
    /// Whether the result has at least one row. The answer does not change as rows are read, and is false
    /// once the reader is closed. The first call may start downloading of the first batch, if it
    /// hasn't started yet.
    /// </summary>
    public override bool HasRows => Run(static resultSet => resultSet.GetHasRows());

    public override int Depth => 0;

    public override object this[int ordinal] => GetValue(ordinal);

    public override object this[string name] => GetValue(GetOrdinal(name));

    /// <summary>The Snowflake query ID of the result, or an empty string when sf_core reported none.</summary>
    public string GetQueryId() => _resultSet.QueryId;

    public override bool Read() => Run(static resultSet => resultSet.Read());

    public override Task<bool> ReadAsync(CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
            return Task.FromCanceled<bool>(cancellationToken);

        return Run(static resultSet => resultSet.TryAdvanceWithinBatch()) ? RowAvailable : ReadNextBatchAsync(cancellationToken);
    }

    // TODO(SNOW-4429197): move to the next statement's result of a multi-statement query.
    public override bool NextResult() => false;

    public override bool IsDBNull(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.IsDBNull(column));

    public override object GetValue(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetValue(column));

    public override int GetValues(object[] values)
    {
        ArgumentNullExceptionHelper.ThrowIfNull(values, nameof(values));

        var count = Math.Min(values.Length, FieldCount);
        for (var i = 0; i < count; i++)
            values[i] = GetValue(i);

        return count;
    }

    public override bool GetBoolean(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<bool>(column));

    public override byte GetByte(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<byte>(column));

    public override long GetBytes(int ordinal, long dataOffset, byte[]? buffer, int bufferOffset, int length)
    {
        ThrowIfInvalidCopyArguments(dataOffset, buffer?.Length, bufferOffset, length);
        return Run(resultSet => resultSet.GetBytes(ordinal, dataOffset, buffer, bufferOffset, length));
    }

    public override char GetChar(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<char>(column));

    public override long GetChars(int ordinal, long dataOffset, char[]? buffer, int bufferOffset, int length)
    {
        ThrowIfInvalidCopyArguments(dataOffset, buffer?.Length, bufferOffset, length);
        return Run(resultSet => resultSet.GetChars(ordinal, dataOffset, buffer, bufferOffset, length));
    }

    public override DateTime GetDateTime(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<DateTime>(column));

    public override decimal GetDecimal(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<decimal>(column));

    public override double GetDouble(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<double>(column));

    public override float GetFloat(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<float>(column));

    public override Guid GetGuid(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<Guid>(column));

    public override short GetInt16(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<short>(column));

    public override int GetInt32(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<int>(column));

    public override long GetInt64(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<long>(column));

    /// <summary>
    /// The value of the column as a string. Returns null for a NULL value of a TEXT column; a NULL value of
    /// any other column is a conversion error.
    /// </summary>
    public override string GetString(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<string>(column))!;

    // TODO(SNOW-4382582): override GetFieldValue<T> through Run once the converters from PR stack #3380 land; the
    // base implementation casts GetValue, which skips the converter's IValueReader<T> and the carrier mapping.
    /// <summary>The value of a TIME column as the time of day.</summary>
    public TimeSpan GetTimeSpan(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<TimeSpan>(column))!;

    /// <summary>The value of a structured OBJECT column mapped onto <typeparamref name="T"/>.</summary>
    public T GetObject<T>(int ordinal)
        where T : class, new() =>
        Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<T>(column))!;

    /// <summary>The value of a structured ARRAY column.</summary>
    public T[] GetArray<T>(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<T[]>(column))!;

    /// <summary>The value of a structured MAP column.</summary>
    // TODO future milestone: read structured types.
    public Dictionary<TKey, TValue> GetMap<TKey, TValue>(int ordinal)
        where TKey : notnull =>
        Run(ordinal, static (resultSet, column) => resultSet.GetFieldValue<Dictionary<TKey, TValue>>(column))!;

    /// <summary>The Snowflake type of the column in upper case, for example <c>FIXED</c> or <c>TEXT</c>.</summary>
    public override string GetDataTypeName(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetDataTypeName(column));

    public override Type GetFieldType(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetFieldType(column));

    public override string GetName(int ordinal) => Run(ordinal, static (resultSet, column) => resultSet.GetName(column));

    /// <summary>
    /// The ordinal of the first column with the given name.
    /// </summary>
    /// <exception cref="SnowflakeDbException">No column has that name (<see cref="SFError.COLUMN_INDEX_OUT_OF_BOUND"/>).</exception>
    public override int GetOrdinal(string name) => name is null
        ? throw new ArgumentNullException(nameof(name))
        : Run(resultSet => resultSet.GetOrdinal(name));

    /// <summary>
    /// Describes the columns, one row per column. Available after the reader is closed, and returns the same
    /// <see cref="DataTable"/> on every call.
    /// </summary>
    /// <exception cref="SnowflakeDbException">A column has a Snowflake type the driver does not know (<see cref="SFError.INTERNAL_ERROR"/>).</exception>
    public override DataTable GetSchemaTable() => Run(static resultSet => resultSet.GetSchemaTable());

    public override IEnumerator GetEnumerator() => new DbEnumerator(this, closeReader: false);

    public override void Close()
    {
        _logger.LogDebug("Closing the reader of query {QueryId}", _resultSet.QueryId);
        if (_resultSet.IsClosed)
            return;

        using var activity = LoggerAndTelemetryStubs.StartActivity(TelemetryActivities.DataReaderClose);
        _resultSet.Close();
        activity?.SetSuccess();
    }

    protected override void Dispose(bool disposing)
    {
        _logger.LogDebug("Disposing the reader of query {QueryId}", _resultSet.QueryId);
        base.Dispose(disposing);
    }

    private static void ThrowIfInvalidCopyArguments(long dataOffset, int? bufferLength, int bufferOffset, int length)
    {
        if (dataOffset < 0)
            throw new ArgumentOutOfRangeException(nameof(dataOffset), dataOffset, "The offset must not be negative.");

        if (!bufferLength.HasValue)
            return;

        if (bufferOffset < 0 || bufferOffset > bufferLength)
            throw new ArgumentOutOfRangeException(nameof(bufferOffset), bufferOffset, "The offset must lie within the buffer.");

        if (length < 0 || length > bufferLength - bufferOffset)
            throw new ArgumentOutOfRangeException(nameof(length), length, "The length must fit in the buffer after the offset.");
    }

    // The result set logs a failed fetch at error level and leaves a failed conversion to the caller, so the
    // boundary logs the carrier at debug level only.
    private Exception ToPublicException(SnowflakeException carrier, string method)
    {
        _logger.LogDebug(carrier, "The reader {Method} failed for query {QueryId}.", method, _resultSet.QueryId);
        return carrier.ToPublicException();
    }

    private async Task<bool> ReadNextBatchAsync(CancellationToken cancellationToken)
    {
        try
        {
            return await _resultSet.ReadNextBatchAsync(cancellationToken).ConfigureAwait(false);
        }
        catch (SnowflakeException exception)
        {
            _logger.LogDebug(exception, "{Operation} failed", nameof(ReadNextBatchAsync));
            throw ToPublicException(exception, nameof(ReadAsync));
        }
    }

    private T Run<T>(Func<ISnowflakeResultSet, T> operation, [CallerMemberName] string method = "")
    {
        try
        {
            return operation(_resultSet);
        }
        catch (SnowflakeException exception)
        {
            _logger.LogDebug(exception, "{Operation} failed when running {Method}", nameof(Run), method);
            throw ToPublicException(exception, method);
        }
    }

    private T Run<T>(int ordinal, Func<ISnowflakeResultSet, int, T> operation, [CallerMemberName] string method = "")
    {
        try
        {
            return operation(_resultSet, ordinal);
        }
        catch (SnowflakeException exception)
        {
            _logger.LogDebug(exception, "{Operation} failed when running {Method}", nameof(Run), method);
            throw ToPublicException(exception, method);
        }
    }
}
