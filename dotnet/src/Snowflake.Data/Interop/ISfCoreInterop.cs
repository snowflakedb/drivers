namespace Snowflake.Data.Interop;

/// <summary>
/// Abstraction over the raw sf_core native library calls.
/// </summary>
internal unsafe interface ISfCoreInterop
{
    void Initialize();

    nuint CallProto(string api, string method, byte* request, nuint requestLen, byte** response, nuint* responseLen);

    ulong CallProtoAsync(string api, string method, byte* request, nuint requestLen, void* userData);

    void CallProtoCancel(ulong asyncHandle);

    void CallFreeBuffer(byte* buffer, UIntPtr responseLen);
}
