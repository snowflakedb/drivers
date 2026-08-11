namespace Snowflake.Data.Interop;

internal unsafe interface IInteropStringHelper
{
    string PtrToStringUtf8(byte* ptr);
}
