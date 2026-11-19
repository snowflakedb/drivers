using System.Runtime.InteropServices;
using System.Text;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.TfmDependent;

namespace Snowflake.Data.Tests.Core;

[Trait("Category", "Unit")]
public sealed unsafe class InteropStringHelperTest
{
    private readonly IInteropStringHelper _helper = SfCoreInteropProvider.StringHelper;

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestNullPointerGivesEmptyString()
    {
        Assert.Equal(string.Empty, _helper.PtrToStringUtf8(null));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestTerminatedUtf8StringIsDecoded()
    {
        var bytes = "zażółć\0"u8.ToArray();

        fixed (byte* ptr = bytes)
            Assert.Equal("zażółć", _helper.PtrToStringUtf8(ptr));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestStringWithoutTerminatorIsCutOffAtMaximumLength()
    {
        const int size = InteropStringHelper.MaxBytes + 16;
        var memory = (byte*)Marshal.AllocHGlobal(size);
        try
        {
            new Span<byte>(memory, size).Fill((byte)'a');
            memory[size - 1] = 0;

            var text = _helper.PtrToStringUtf8(memory);

            Assert.Equal(InteropStringHelper.MaxBytes, text.Length);
        }
        finally
        {
            Marshal.FreeHGlobal((nint)memory);
        }
    }

    [SnowflakeTheory(RetriesCount = RetriesCount.NoRetries)]
    [InlineData("")]
    [InlineData("a")]
    [InlineData("outbound HTTP call")]
    [InlineData("café 日本語")]
    public void ANulTerminatedUtf8StringReadsBackUnchanged(string text)
    {
        Assert.Equal(text, Read([.. Encoding.UTF8.GetBytes(text), 0]));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ReadingStopsAtTheFirstNul()
    {
        Assert.Equal("head", Read([.. "head"u8, 0, .. "tail"u8, 0]));
    }

    private static string Read(byte[]? nulTerminated)
    {
        fixed (byte* text = nulTerminated)
        {
            return SfCoreInteropProvider.StringHelper.PtrToStringUtf8(text);
        }
    }
}
