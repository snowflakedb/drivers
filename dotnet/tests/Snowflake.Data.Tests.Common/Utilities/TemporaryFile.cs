namespace Snowflake.Data.Tests.Utilities;

/// <summary>
/// A file with a unique name in the system temp directory, deleted on dispose.
/// Declare it with <c>using var</c> so a test needs no try/finally for cleanup.
/// </summary>
public sealed class TemporaryFile : IDisposable
{
    public TemporaryFile(string contents, string extension = ".txt")
    {
        Path = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"ud_test_{Guid.NewGuid():N}{extension}");
        File.WriteAllText(Path, contents);
    }

    public string Path { get; }

    public string Name => System.IO.Path.GetFileName(Path);

    public void Dispose() => File.Delete(Path);
}

/// <summary>
/// An empty directory with a unique name in the system temp directory, deleted
/// with its contents on dispose.
/// </summary>
public sealed class TemporaryDirectory : IDisposable
{
    public TemporaryDirectory()
    {
        Path = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"ud_test_{Guid.NewGuid():N}");
        Directory.CreateDirectory(Path);
    }

    public string Path { get; }

    public void Dispose()
    {
        if (Directory.Exists(Path))
        {
            Directory.Delete(Path, recursive: true);
        }
    }
}
