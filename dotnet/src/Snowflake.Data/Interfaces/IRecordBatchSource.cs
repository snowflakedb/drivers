using Apache.Arrow;
using Snowflake.Data.Core.Errors;

namespace Snowflake.Data.Interfaces;

/// <summary>
/// The Arrow record batches of one result, in order. One call at a time.
/// </summary>
/// <remarks>
/// Each returned batch belongs to the caller, which disposes it. Failures are thrown as
/// <see cref="SnowflakeException"/>. A call made while another is in progress throws
/// <see cref="SFError.DATA_READER_FETCH_IN_PROGRESS"/>, and a call after <see cref="IDisposable.Dispose"/>
/// throws <see cref="SFError.DATA_READER_ALREADY_CLOSED"/>. A <see cref="NextAsync"/> whose token is already
/// cancelled throws <see cref="SnowflakeCancellationException"/> with <see cref="SFError.QUERY_CANCELLED"/>
/// and loses no batch. A call that throws <see cref="SFError.QUERY_RESULT_FETCH_FAILED"/> is terminal: every later
/// call, until <see cref="IDisposable.Dispose"/>, throws a new <see cref="SFError.QUERY_RESULT_FETCH_FAILED"/>
/// carrier with the same query ID and arguments rather than returning null. <see cref="IDisposable.Dispose"/>
/// releases what the source holds, may be called more than once, and may be called while a call is in progress.
/// </remarks>
internal interface IRecordBatchSource : IDisposable
{
    /// <summary>The next batch, or null when the result has no more batches.</summary>
    RecordBatch? Next();

    /// <summary>The next batch, or null when the result has no more batches.</summary>
    Task<RecordBatch?> NextAsync(CancellationToken cancellationToken);
}
