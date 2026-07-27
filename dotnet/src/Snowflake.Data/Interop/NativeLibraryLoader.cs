#if !NETFRAMEWORK
using System.Reflection;
using System.Runtime.InteropServices;

namespace Snowflake.Data.Interop;

/// <summary>
/// Registers a custom DLL import resolver that locates the sf_core native library.
/// Resolution order:
///   1. SF_CORE_LIB_PATH environment variable (explicit path to directory or file)
///   2. Assembly output directory
///   3. Default OS search (LD_LIBRARY_PATH, DYLD_LIBRARY_PATH, PATH, etc.)
/// </summary>
///  TODO this is PoC, will be subject to refactoring in the future
internal static class NativeLibraryLoader
{
    private const string SFCore = "sf_core";
    private static int _registered;

    public static void Register()
    {
        if (Interlocked.Exchange(ref _registered, 1) == 1)
            return;

        NativeLibrary.SetDllImportResolver(typeof(NativeLibraryLoader).Assembly, ResolveLibrary);
    }

    private static nint ResolveLibrary(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (libraryName != SFCore)
            return nint.Zero;

        if (TryResolveFromEnvVar(out var resolveLibrary))
            return resolveLibrary;

        if (TryResolveFromOutputDir(out resolveLibrary))
            return resolveLibrary;

        return nint.Zero;
    }

    private static bool TryResolveFromOutputDir(out IntPtr resolveLibrary)
    {
        resolveLibrary = nint.Zero;
        // TODO GetCurrentDirectory() returns the process working directory, not the directory where the assembly (.dll) was deployed. If tests or the host app are launched from a different CWD, this fallback silently searches the wrong location. Use the assembly's own directory
        var assemblyDir = Directory.GetCurrentDirectory();
        return TryResolveFromPath(assemblyDir, out resolveLibrary);
    }

    private static bool TryResolveFromEnvVar(out IntPtr resolveLibrary)
    {
        resolveLibrary = nint.Zero;
        var envPath = Environment.GetEnvironmentVariable("SF_CORE_LIB_PATH");

        return !string.IsNullOrEmpty(envPath) && TryResolveFromPath(envPath, out resolveLibrary);
    }

    private static bool TryResolveFromPath(string path, out nint resolved)
    {
        resolved = nint.Zero;
        string fullPath;

        if (File.Exists(path))
        {
            fullPath = path;
        }
        else if (Directory.Exists(path))
        {
            fullPath = Path.Combine(path, PlatformLibraryName());
        }
        else
            return false;

        return NativeLibrary.TryLoad(fullPath, out resolved);
    }

    private static string PlatformLibraryName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return $"{SFCore}.dll";

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return $"lib{SFCore}.dylib";

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            return $"lib{SFCore}.so";

        throw new PlatformNotSupportedException("Unknown platform!");
    }
}
#endif
