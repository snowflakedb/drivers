using System.Buffers.Binary;
using Snowflake.Data.Interop;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed class ArrowPtrCodecTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestAddressIsEncodedAsEightLittleEndianBytes()
    {
        var encoded = ArrowPtrCodec.Encode(0x0102_0304);

        Assert.Equal([0x04, 0x03, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00], encoded.ToByteArray());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestZeroAddressIsEncodedAsEightZeroBytes()
    {
        var encoded = ArrowPtrCodec.Encode(0);

        Assert.Equal(new byte[sizeof(ulong)], encoded.ToByteArray());
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestAddressAboveTwoGibEncodesAsUnsignedValueOnEveryPointerSize()
    {
        var address = unchecked((nint)(nuint)0x8000_0010u);

        var encoded = ArrowPtrCodec.Encode(address);

        Assert.Equal(0x0000_0000_8000_0010UL, BinaryPrimitives.ReadUInt64LittleEndian(encoded.Span));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestAddressWithTopBitSetIsZeroExtendedToSixtyFourBits()
    {
        nint address = -16;
        var expected = IntPtr.Size == sizeof(uint) ? 0x0000_0000_FFFF_FFF0UL : 0xFFFF_FFFF_FFFF_FFF0UL;

        var encoded = ArrowPtrCodec.Encode(address);

        Assert.Equal(expected, BinaryPrimitives.ReadUInt64LittleEndian(encoded.Span));
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData(0)]
    [InlineData(0x0102_0304)]
    [InlineData(-16)]
    public void TestDecodeReturnsAddressEncodeWrote(int address)
    {
        var encoded = ArrowPtrCodec.Encode(address);

        Assert.Equal((nint)address, ArrowPtrCodec.Decode(encoded.Span));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestDecodeRejectsFewerThanEightBytes()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => _ = ArrowPtrCodec.Decode(new byte[sizeof(ulong) - 1]));
    }
}
