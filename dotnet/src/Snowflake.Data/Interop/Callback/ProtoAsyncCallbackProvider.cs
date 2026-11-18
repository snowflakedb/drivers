using System.Buffers;

namespace Snowflake.Data.Interop.Callback;

internal sealed unsafe class ProtoAsyncCallbackProvider
{
    internal static void ResponseCallback(void* userDataPointer, nuint status, nuint ptr, nuint len)
    {
        SfCoreAsyncCallData tcsWrapper = null!;
        byte[]? responseBytes = null;
        try
        {
            tcsWrapper = SfCoreAsyncCallData.FromPtr(userDataPointer);

            if (ptr == 0 || len > int.MaxValue)
            {
                Exception failure = ptr == 0
                    ? new InvalidOperationException("sf_core passed a null response pointer.")
                    : new NotSupportedException($"sf_core passed a response of {len} bytes, which exceeds {int.MaxValue}.");
                tcsWrapper.TaskCompletionSource.TrySetException(failure);
                tcsWrapper.Dispose();
                return;
            }

            var responseLen = (int)len;
            responseBytes = ArrayPool<byte>.Shared.Rent(responseLen);
            var response = new ArraySegment<byte>(responseBytes, 0, responseLen);

            new ReadOnlySpan<byte>((void*)ptr, responseLen).CopyTo(responseBytes);
            tcsWrapper.TaskCompletionSource.SetResult(new((int)status, response, responseBytes));
        }
        catch (Exception e)
        {
            tcsWrapper?.TaskCompletionSource.TrySetException(e);
            if (responseBytes != null)
                ArrayPool<byte>.Shared.Return(responseBytes);
        }
        finally
        {
            try
            {
                tcsWrapper?.Dispose();
            }
            catch
            {
                // TODO leave any trace here
                // no exceptions across ffi boundary
            }
        }
    }
}
