#if !NETFRAMEWORK
using Snowflake.Data.Interop;

namespace Snowflake.Data.Tests.Interop;

/// <summary>
/// <see cref="SfCoreLibraryLoader.ResolveLibrary"/> against copies of the stub library named as sf_core.
/// </summary>
/// <remarks>
/// The tests change <c>SF_CORE_LIB_PATH</c> and the working directory, which are process-wide; the Interop collection
/// runs its tests one at a time, and every test restores both.
/// </remarks>
[Collection("Interop")]
[Trait("Category", "Interop")]
public sealed class SfCoreLibraryLoaderTest : IDisposable
{
    private const string SfCoreLibPath = "SF_CORE_LIB_PATH";

    private readonly string? _previousLibPath = Environment.GetEnvironmentVariable(SfCoreLibPath);
    private readonly string _previousDirectory = Directory.GetCurrentDirectory();
    private readonly string _libraryDirectory = Path.Combine(Path.GetTempPath(), $"sf_core_loader_{Guid.NewGuid():N}");
    private readonly string _emptyDirectory = Path.Combine(Path.GetTempPath(), $"sf_core_loader_empty_{Guid.NewGuid():N}");
    private readonly string _workingDirectory = Path.Combine(Path.GetTempPath(), $"sf_core_loader_cwd_{Guid.NewGuid():N}");
    private readonly string _libraryPath;
    private readonly string _workingDirectoryLibraryPath;
    private readonly List<nint> _loaded = [];

    public SfCoreLibraryLoaderTest()
    {
        Directory.CreateDirectory(_libraryDirectory);
        Directory.CreateDirectory(_emptyDirectory);
        Directory.CreateDirectory(_workingDirectory);
        _libraryPath = Path.Combine(_libraryDirectory, SfCoreFileName());
        _workingDirectoryLibraryPath = Path.Combine(_workingDirectory, SfCoreFileName());
        File.Copy(_previousLibPath!, _libraryPath);
        File.Copy(_previousLibPath!, _workingDirectoryLibraryPath);
    }

    public void Dispose()
    {
        Environment.SetEnvironmentVariable(SfCoreLibPath, _previousLibPath);
        Directory.SetCurrentDirectory(_previousDirectory);
        foreach (var handle in _loaded)
            NativeLibrary.Free(handle);
        Directory.Delete(_libraryDirectory, recursive: true);
        Directory.Delete(_emptyDirectory, recursive: true);
        Directory.Delete(_workingDirectory, recursive: true);
    }

    [SnowflakeFact]
    public void ALibraryOtherThanSfCoreIsLeftToTheDefaultSearch()
    {
        Environment.SetEnvironmentVariable(SfCoreLibPath, _libraryPath);

        Assert.Equal(nint.Zero, Resolve("sf_core_stub"));
    }

    [SnowflakeFact]
    public void AnEnvironmentVariableNamingTheLibraryFileLoadsThatFile()
    {
        Environment.SetEnvironmentVariable(SfCoreLibPath, _libraryPath);
        Directory.SetCurrentDirectory(_emptyDirectory);

        AssertLoadedFrom(_libraryPath, Resolve("sf_core"));
    }

    [SnowflakeFact]
    public void AnEnvironmentVariableNamingADirectoryLoadsThePlatformNamedLibraryInIt()
    {
        Environment.SetEnvironmentVariable(SfCoreLibPath, _libraryDirectory);
        Directory.SetCurrentDirectory(_emptyDirectory);

        AssertLoadedFrom(_libraryPath, Resolve("sf_core"));
    }

    [SnowflakeFact]
    public void WithoutTheEnvironmentVariableTheLibraryInTheWorkingDirectoryIsLoaded()
    {
        Environment.SetEnvironmentVariable(SfCoreLibPath, null);
        Directory.SetCurrentDirectory(_libraryDirectory);

        AssertLoadedFrom(_libraryPath, Resolve("sf_core"));
    }

    [SnowflakeFact]
    public void AnEnvironmentVariableNamingAMissingPathFallsBackToTheWorkingDirectory()
    {
        Environment.SetEnvironmentVariable(SfCoreLibPath, Path.Combine(_emptyDirectory, "missing"));
        Directory.SetCurrentDirectory(_libraryDirectory);

        AssertLoadedFrom(_libraryPath, Resolve("sf_core"));
    }

    [SnowflakeFact]
    public void AnEnvironmentVariableNamingADirectoryWithoutTheLibraryFallsBackToTheWorkingDirectory()
    {
        Environment.SetEnvironmentVariable(SfCoreLibPath, _emptyDirectory);
        Directory.SetCurrentDirectory(_libraryDirectory);

        AssertLoadedFrom(_libraryPath, Resolve("sf_core"));
    }

    [SnowflakeFact]
    public void EnvironmentVariablePrecedesWorkingDirectory()
    {
        Environment.SetEnvironmentVariable(SfCoreLibPath, _libraryPath);
        Directory.SetCurrentDirectory(_workingDirectory);

        AssertLoadedFrom(_libraryPath, Resolve("sf_core"));
    }

    [SnowflakeFact]
    public void EnvironmentVariableNamingUnloadableFileFallsBackToWorkingDirectory()
    {
        var unloadable = Path.Combine(_emptyDirectory, SfCoreFileName());
        File.WriteAllText(unloadable, "not a native library");
        Environment.SetEnvironmentVariable(SfCoreLibPath, unloadable);
        Directory.SetCurrentDirectory(_workingDirectory);

        AssertLoadedFrom(_workingDirectoryLibraryPath, Resolve("sf_core"));
    }

    [SnowflakeFact]
    public void ALibraryFoundNowhereIsLeftToTheDefaultSearch()
    {
        Environment.SetEnvironmentVariable(SfCoreLibPath, _emptyDirectory);
        Directory.SetCurrentDirectory(_emptyDirectory);

        Assert.Equal(nint.Zero, Resolve("sf_core"));
    }

    private nint Resolve(string libraryName)
    {
        var handle = SfCoreLibraryLoader.ResolveLibrary(libraryName, typeof(SfCoreLibraryLoader).Assembly, null);
        if (handle != nint.Zero)
            _loaded.Add(handle);
        return handle;
    }

    private static void AssertLoadedFrom(string path, nint handle)
    {
        Assert.NotEqual(nint.Zero, handle);
        Assert.True(NativeLibrary.TryLoad(path, out var expected));
        try
        {
            Assert.Equal(expected, handle);
        }
        finally
        {
            NativeLibrary.Free(expected);
        }
    }

    private static string SfCoreFileName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return "sf_core.dll";

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return "libsf_core.dylib";

        return "libsf_core.so";
    }
}
#endif
