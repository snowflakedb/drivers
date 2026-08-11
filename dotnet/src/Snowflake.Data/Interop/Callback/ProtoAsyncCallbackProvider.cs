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
                tcsWrapper.TaskCompletionSource.SetException(new Exception("TODO"));
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
