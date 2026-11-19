using System.Buffers.Binary;
using System.Runtime.InteropServices;
using Google.Protobuf;

namespace Snowflake.Data.Interop;

/// <summary>The forms native addresses take in sf_core's protobuf messages.</summary>
/// <remarks>
/// An address the driver sends to sf_core is 8 bytes, little-endian, zero-extended. The address of an Arrow C stream
/// sf_core allocates and returns is its pointer's native-width, native-endian bytes: 4 bytes in a 32-bit process.
/// </remarks>
internal static class ArrowPtrCodec
{
    /// <summary>Encodes <paramref name="address"/> in the 8-byte form sf_core reads.</summary>
    internal static ByteString Encode64(nint address)
    {
        Span<byte> bytes = stackalloc byte[sizeof(ulong)];
        BinaryPrimitives.WriteUInt64LittleEndian(bytes, unchecked((ulong)(nuint)address));
        return ByteString.CopyFrom(bytes);
    }

    /// <summary>Decodes an address in the 8-byte form <see cref="Encode64"/> writes.</summary>
    /// <exception cref="ArgumentOutOfRangeException">
    /// <paramref name="bytes"/> is shorter than 8 bytes, or the address does not fit in a pointer of this process.
    /// </exception>
    internal static nint Decode64(ReadOnlySpan<byte> bytes)
    {
        var address = BinaryPrimitives.ReadUInt64LittleEndian(bytes);
        if (IntPtr.Size == sizeof(uint) && address > uint.MaxValue)
            throw new ArgumentOutOfRangeException(nameof(bytes), address, "The address does not fit in a 32-bit pointer.");

        return (nint)(nuint)address;
    }

    /// <summary>Decodes the address of an Arrow C stream sf_core allocated and returned.</summary>
    /// <exception cref="ArgumentOutOfRangeException"><paramref name="bytes"/> is not <see cref="IntPtr.Size"/> bytes.</exception>
    internal static nint Decode(ReadOnlySpan<byte> bytes)
    {
        if (bytes.Length != IntPtr.Size)
            throw new ArgumentOutOfRangeException(nameof(bytes), bytes.Length, $"A native address is {IntPtr.Size} bytes in this process.");

        return MemoryMarshal.Read<nint>(bytes);
    }
}
