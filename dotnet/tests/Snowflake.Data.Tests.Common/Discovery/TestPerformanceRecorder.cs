using System.Runtime.InteropServices;

namespace Snowflake.Data.Tests.Discovery;

public sealed class TestPerformanceRecorder : IDisposable
{
    private static readonly bool Enabled =
        Environment.GetEnvironmentVariable("SNOWFLAKE_TEST_PERF_RECORD") == "1";

    private readonly Queue<LogEntry> _entries = new();
    private readonly object _lock = new();
    private static readonly string? FilePath;

    static TestPerformanceRecorder()
    {
        if (!Enabled)
            return;

        var dotnetVersion = Environment.GetEnvironmentVariable("net_version");
        var cloudEnv = Environment.GetEnvironmentVariable("snowflake_cloud_env");
        var separator = Path.DirectorySeparatorChar;

        // TODO future milestones
        FilePath = $"..{separator}..{separator}..{separator}{GetOs()}_{dotnetVersion}_{cloudEnv}_performance.csv";
        File.WriteAllText(FilePath, "test;time_in_ms\n");
    }

    public void Dispose()
    {
        if (!Enabled)
            return;

        LogEntry[] toWrite;
        lock (_lock)
        {
            toWrite = _entries.ToArray();
            _entries.Clear();
        }

        WriteToFile(toWrite);
    }

    public void AddEntry(Xunit.Sdk.ITestResultMessage testResult)
    {
        if (!Enabled)
            return;

        LogEntry[] toWrite;
        lock (_lock)
        {
            _entries.Enqueue(new LogEntry
            {
                TestName = testResult.TestUniqueID,
                TestDuration = testResult.ExecutionTime,
            });

            if (_entries.Count < 100)
                return;

            toWrite = _entries.ToArray();
            _entries.Clear();
        }

        WriteToFile(toWrite);
    }

    private static void WriteToFile(LogEntry[] entries)
    {
        if (FilePath is null || entries.Length == 0)
            return;

        var lines = entries.Select(x => $"{x.TestName};{x.TestDuration}");
        var text = string.Join("\n", lines);

#if NETFRAMEWORK
        var sw = File.AppendText(FilePath);
        sw.Write(text);
        sw.Flush();
        sw.Close();
#else
        File.AppendAllText(FilePath, text);
#endif
    }

    private static string GetOs()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return "windows";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            return "linux";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return "macos";
        return "unknown";
    }

    private struct LogEntry
    {
        public string TestName { get; set; }
        public decimal TestDuration { get; set; }
    }
}
