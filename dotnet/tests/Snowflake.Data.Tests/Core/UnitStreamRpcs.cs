using Apache.Arrow.C;
using Apache.Arrow.Ipc;
using NSubstitute;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.Handles;
using Snowflake.Data.Proto;
using Snowflake.Data.Tests.Utilities;

namespace Snowflake.Data.Tests.Core;

/// <summary>
/// Makes a substituted <see cref="IDatabaseDriverService"/> serve the Arrow stream RPCs the way sf_core does.
/// </summary>
internal static class UnitStreamRpcs
{
    /// <summary>ResultSetGetStream fills the caller-allocated stream it is given with <paramref name="stream"/>.</summary>
    internal static void ServeGetStream(IDatabaseDriverService driver, IArrowArrayStream stream) =>
        driver.ResultSetExportStreamAsync(Arg.Any<ResultSetExportStreamRequest>(), Arg.Any<CancellationToken>())
            .Returns(call =>
            {
                ArrowStreamExporter.ExportStreamInto(stream, call.Arg<ResultSetExportStreamRequest>().OutStream.Value.Span);
                return new ResultSetExportStreamResponse();
            });

    /// <summary>ArrowStreamGetNext calls the stream's <c>get_next</c> on a thread of its own and reports the outcome.</summary>
    internal static void ServeGetNext(IDatabaseDriverService driver) =>
        driver.ArrowStreamGetNextBatchAsync(Arg.Any<ArrowStreamGetNextBatchRequest>(), Arg.Any<CancellationToken>())
            .Returns(call =>
            {
                var request = call.Arg<ArrowStreamGetNextBatchRequest>();
                return Task.Run(() => GetNext(request));
            });

    /// <summary>Calls the <c>get_next</c> of the stream in <paramref name="request"/> into its out array on the calling thread.</summary>
    internal static unsafe ArrowStreamGetNextBatchResponse GetNext(ArrowStreamGetNextBatchRequest request)
    {
        var stream = (ArrowArrayStreamAbi*)ArrowPtrCodec.Decode(request.Stream.Value.Span);
        var array = (CArrowArray*)ArrowPtrCodec.Decode(request.OutArray.Value.Span);
        var borrowedArray = new ArrowArrayHandle((nint)array);
        try
        {
            var response = new ArrowStreamGetNextBatchResponse { Status = ArrowStreamCalls.GetNext(stream, borrowedArray) };
            if (response.Status != 0 && ArrowStreamCalls.GetLastError(stream) is { } message)
                response.ErrorMessage = message;

            return response;
        }
        finally
        {
            // The caller owns the array; an invalid handle never frees it.
            borrowedArray.SetHandleAsInvalid();
        }
    }
}
