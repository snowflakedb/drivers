using Snowflake.Data.Proto;

namespace Snowflake.Data.Interop.Extensions;

internal static class PrepareResultExtensions
{
    extension(PrepareResult prepareResult)
    {
        /// <summary>
        /// Releases the Arrow C stream in the prepare result, which the driver does not read, and clears
        /// <see cref="PrepareResult.Stream"/>.
        /// </summary>
        /// <remarks>
        /// This calls the stream's <c>release</c> callback, without calling <c>get_schema</c>, and leaves the struct
        /// sf_core allocated in place. A missing or zero address does nothing, so a second call is a no-op.
        /// </remarks>
        /// <exception cref="ArgumentException">The stream at the address is already released.</exception>
        internal void Release()
        {
            var stream = prepareResult.Stream;
            if (stream is null || stream.Value.IsEmpty)
                return;

            var address = ArrowPtrCodec.Decode(stream.Value.Span);
            if (address == IntPtr.Zero)
                return;

            PrepareResult.Release(address);
            prepareResult.Stream = null;
        }

        private static void Release(nint address)
        {
            using var _ = new NativeArrowStream(address);
        }
    }
}
