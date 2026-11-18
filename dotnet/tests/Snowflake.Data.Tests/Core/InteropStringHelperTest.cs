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
        var bytes = Encoding.UTF8.GetBytes("zażółć\0");

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
}
