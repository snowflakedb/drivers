using Apache.Arrow;
using Snowflake.Data.Core.Errors;
using Snowflake.Data.Interfaces;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.Handles;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Core.ResultSet;

/// <summary>
/// Reads the batches of an Arrow C stream that sf_core filled for a result set, and owns the stream.
/// </summary>
/// <remarks>
/// <see cref="Next"/> calls the stream's <c>get_next</c> on the calling thread. <see cref="NextAsync"/> has sf_core
/// call it on a thread of its own (ArrowStreamGetNext), so the caller's thread is free while a chunk downloads.
/// Neither can be interrupted once started; a token that is already cancelled stops <see cref="NextAsync"/>
/// before it reaches the stream and leaves the source readable. Any other failed read ends the source, whether the
/// stream reports an error (<see cref="SFError.QUERY_RESULT_FETCH_FAILED"/>) or the sf_core call behind
/// <see cref="NextAsync"/> fails (<see cref="SFError.INTERNAL_ERROR"/>): every later <see cref="Next"/> and
/// <see cref="NextAsync"/> rethrows that exception without reading the stream, which may already have consumed a
/// batch or would otherwise report a clean end after one error. The stream is released at its end, at a failed
/// read, and on <see cref="Dispose"/>, which stops the prefetch of chunks not yet downloaded.
/// </remarks>
internal sealed class SfCoreRecordBatchSource : IRecordBatchSource
{
    private readonly IDatabaseDriverService _driverService;
    private readonly NativeArrowStream _stream;
    private readonly Schema _schema;
    private readonly string? _queryId;
    private volatile bool _isDrained;
    private volatile bool _isDisposed;
    private volatile SnowflakeException? _failure;

    private SfCoreRecordBatchSource(IDatabaseDriverService driverService, NativeArrowStream stream, Schema schema, string? queryId)
    {
        _driverService = driverService;
        _stream = stream;
        _schema = schema;
        _queryId = queryId;
    }

    public RecordBatch? Next()
    {
        if (HasEnded())
            return null;

        using var lease = Enter(_stream, _queryId);
        using var slot = new ArrowArrayHandle();
        var status = lease.GetNext(slot);
        return Complete(status, status == 0 ? null : lease.GetLastError(), slot);
    }

    public async Task<RecordBatch?> NextAsync(CancellationToken cancellationToken)
    {
        if (cancellationToken.IsCancellationRequested)
            throw new SnowflakeCancellationException(SFError.QUERY_CANCELLED, _queryId, null);

        if (HasEnded())
            return null;

        using var lease = Enter(_stream, _queryId);
        using var slot = new ArrowArrayHandle();
        var request = new ArrowStreamGetNextBatchRequest { Stream = lease.ToProto(), OutArray = slot.ToProto() };
        ArrowStreamGetNextBatchResponse? response = null;
        SnowflakeException? exceptionToThrow = null;
        try
        {
            response = await _driverService.ArrowStreamGetNextBatchAsync(request, cancellationToken)
                .ConfigureAwait(false);
        }
        catch (OperationCanceledException ex) when (cancellationToken.IsCancellationRequested)
        {
            throw new SnowflakeCancellationException(SFError.QUERY_CANCELLED, _queryId, ex);
        }
        catch (ServiceException ex)
        {
            var sfException = SnowflakeException.FromCore(ex);
            sfException.QueryId ??= _queryId;
            exceptionToThrow = sfException;
        }
        catch (TransportException ex)
        {
            var sfException = SnowflakeException.FromCore(ex);
            sfException.QueryId ??= _queryId;
            exceptionToThrow = sfException;
        }

        var message = exceptionToThrow?.Message;
        message ??= response != null
            ? (response.HasErrorMessage ? response.ErrorMessage : null)
            : null;

        return Complete(response?.Status ?? 1, message, slot, exceptionToThrow);
    }

    public void Dispose()
    {
        _isDisposed = true;
        _stream.Dispose();
    }

    /// <summary>Reads the schema of <paramref name="stream"/>, which sf_core has filled, and takes ownership of the stream.</summary>
    /// <param name="driverService">The sf_core service that reads the stream for <see cref="NextAsync"/>.</param>
    /// <param name="stream">The filled stream; it is disposed when this method throws.</param>
    /// <param name="queryId">The query the rows belong to, attached to failures.</param>
    /// <exception cref="SnowflakeDriverException">
    /// <see cref="SFError.QUERY_RESULT_FETCH_FAILED"/> when the stream reports no schema.
    /// </exception>
    internal static SfCoreRecordBatchSource Open(IDatabaseDriverService driverService, NativeArrowStream stream, string? queryId)
    {
        try
        {
            using var lease = Enter(stream, queryId);
            using var slot = new ArrowSchemaHandle();
            var status = lease.GetSchema(slot);
            return status == 0
                ? new SfCoreRecordBatchSource(driverService, stream, slot.Import(), queryId)
                : throw new SnowflakeDriverException(SFError.QUERY_RESULT_FETCH_FAILED, queryId, lease.GetLastError(), status);
        }
        catch
        {
            stream.Dispose();
            throw;
        }
    }

    private static NativeArrowStream.Lease Enter(NativeArrowStream stream, string? queryId) =>
        stream.TryEnter(out var lease) switch
        {
            NativeArrowStreamEnterResult.Entered => lease!,
            NativeArrowStreamEnterResult.Busy => throw new SnowflakeDriverException(SFError.DATA_READER_FETCH_IN_PROGRESS, queryId),
            _ => throw new SnowflakeDriverException(SFError.DATA_READER_ALREADY_CLOSED, queryId),
        };

    private RecordBatch? Complete(int status, string? message, ArrowArrayHandle handle, SnowflakeException? exceptionToThrow = null)
    {
        if (_stream.IsDisposed)
            throw new SnowflakeDriverException(SFError.DATA_READER_ALREADY_CLOSED, _queryId);

        if (status != 0)
        {
            exceptionToThrow ??= new SnowflakeDriverException(SFError.QUERY_RESULT_FETCH_FAILED, _queryId, message ?? string.Empty, status);
            _failure = exceptionToThrow;
            _stream.Dispose();
            throw exceptionToThrow;
        }

        if (!handle.IsReleased)
            return handle.Import(_schema);

        _isDrained = true;
        _stream.Dispose();
        return null;
    }

    private bool HasEnded()
    {
        if (_isDisposed)
            throw new SnowflakeDriverException(SFError.DATA_READER_ALREADY_CLOSED, _queryId);

        return _failure is not null
            ? throw _failure
            : _isDrained;
    }
}
