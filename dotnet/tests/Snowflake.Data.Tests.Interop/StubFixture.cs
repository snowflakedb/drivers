using Snowflake.Data.Interop;
using Snowflake.Data.Interop.TfmDependent;

#if NETFRAMEWORK
using Snowflake.Data.Interop.TfmDependent.Framework;
#endif

namespace Snowflake.Data.Tests.Interop;

/// <summary>
/// Assembly-scoped fixture.  Sets <c>SF_CORE_LIB_PATH</c> to the stub library
/// produced by <c>cargo build -p sf_core_stub</c> and initialises the native
/// interop layer exactly once for the process.
/// </summary>
/// <remarks>
/// Resolution order mirrors <see cref="SfCoreLibraryLoader"/>: the env-var path
/// is checked first, so pointing it at the stub library short-circuits the
/// default sf_core resolution before any P/Invoke fires.
/// </remarks>
public sealed class StubFixture : IDisposable
{
    private const string SfCoreLibPath = "SF_CORE_LIB_PATH";
    private const string StubLibName = "sf_core_stub";

    public StubFixture()
    {
        var stubPath = LocateStubLibrary();
        Environment.SetEnvironmentVariable(SfCoreLibPath, stubPath);

#if NETFRAMEWORK
        // On .NET Framework, NativeLibrary.SetDllImportResolver is unavailable.
        // DllImport("sf_core") uses AssemblyDirectory search — CI copies the stub
        // as sf_core.dll into the output directory before running tests.
#else
        // Register a resolver for this assembly so that StubNativeMethods'
        // [DllImport("sf_core_stub")] declarations resolve to the same stub
        // library.  SfCoreNativeMethods uses its own resolver (registered by
        // SfCoreLibraryLoader) that maps "sf_core" to the same file via
        // SF_CORE_LIB_PATH; this resolver covers the test-only P/Invokes.
        NativeLibrary.SetDllImportResolver(typeof(StubFixture).Assembly,
            (libraryName, _, _) => libraryName == StubLibName ? NativeLibrary.Load(stubPath) : IntPtr.Zero);
#endif

        // Initialize once per process.  SfCoreTransport's Lazy will pick up the
        // stub via SF_CORE_LIB_PATH on the first EnsureInitialized() call.
        SfCoreNativeMethods.Instance.Initialize();
    }

    public void Dispose() { }

    /// <summary>
    /// Walks up the directory tree from the test output directory looking for an
    /// ancestor that contains an <c>sf_core_stub/</c> subdirectory with its own
    /// <c>Cargo.toml</c>, then returns the path to the compiled library inside
    /// <c>sf_core_stub/target/debug/</c>.
    /// </summary>
    private static string LocateStubLibrary()
    {
        var current = new DirectoryInfo(Directory.GetCurrentDirectory());

        for (var depth = 0; depth < 15; depth++)
        {
            if (current is null)
                break;

            var stubCrateDir = Path.Combine(current.FullName, "sf_core_stub");
            if (File.Exists(Path.Combine(stubCrateDir, "Cargo.toml")))
            {
                var libPath = Path.Combine(stubCrateDir, "target", "debug", PlatformLibName());
                if (File.Exists(libPath))
                    return libPath;

                throw new InvalidOperationException(
                    $"Found sf_core_stub crate at '{stubCrateDir}' but the library has not been built. " +
                    "Run 'cargo build' inside 'sf_core_stub/' first.");
            }

            current = current.Parent;
        }

        throw new InvalidOperationException(
            "Could not locate the sf_core_stub crate directory. " +
            $"Set the '{SfCoreLibPath}' environment variable to the stub library path.");
    }

    private static string PlatformLibName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return $"{StubLibName}.dll";

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return $"lib{StubLibName}.dylib";

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            return $"lib{StubLibName}.so";

        throw new PlatformNotSupportedException("Unknown platform.");
    }
}
