namespace Snowflake.Data.Tests.Interop.Compatibility;

internal static class StubLibraryNames
{
    public const string Stub = "sf_core_stub";

    // On .NET Framework the stub is loaded twice, as sf_core.dll and sf_core_stub.dll,
    // and the callback from sf_core_init_dotnet is stored only in the sf_core.dll copy.
#if NETFRAMEWORK
    public const string LogCallbackHolder = "sf_core";
#else
    public const string LogCallbackHolder = Stub;
#endif
}
