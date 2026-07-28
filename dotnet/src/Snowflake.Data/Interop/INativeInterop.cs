namespace Snowflake.Data.Interop;

/// <summary>
/// Abstraction over the raw sf_core native library calls.
/// </summary>
internal unsafe interface INativeInterop
{
    void Initialize();

    nuint CallProto(string api, string method, byte* request, nuint requestLen, byte** response, nuint* responseLen);

    void FreeBuffer(byte* buffer, nuint len);

    string PtrToStringUtf8(byte* ptr);
}
