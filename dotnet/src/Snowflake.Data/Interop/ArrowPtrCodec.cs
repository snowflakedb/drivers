using System.Buffers.Binary;
using Google.Protobuf;

namespace Snowflake.Data.Interop;

/// <summary>The form sf_core reads and writes native addresses in: 8 bytes, little-endian, zero-extended.</summary>
internal static class ArrowPtrCodec
{
    internal static ByteString Encode(nint address)
    {
        Span<byte> bytes = stackalloc byte[sizeof(ulong)];
        BinaryPrimitives.WriteUInt64LittleEndian(bytes, unchecked((ulong)(nuint)address));
        return ByteString.CopyFrom(bytes);
    }

    /// <exception cref="ArgumentOutOfRangeException"><paramref name="bytes"/> is shorter than 8 bytes.</exception>
    internal static nint Decode(ReadOnlySpan<byte> bytes) =>
        unchecked((nint)(nuint)BinaryPrimitives.ReadUInt64LittleEndian(bytes));
}
