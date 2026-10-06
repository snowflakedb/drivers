namespace Snowflake.Data.Tests.Compatibility;

public class FileShims
{
    public static Task WriteAllTextAsync(string path, string content)
    {
#if NETFRAMEWORK
        File.WriteAllText(path, content);
        return Task.CompletedTask;
#else
        return File.WriteAllTextAsync(path, content);
#endif
    }

    public static Task<string> ReadAllTextAsync(string path)
    {
#if NETFRAMEWORK
        var result = File.ReadAllText(path);
        return Task.FromResult(result);
#else
        return File.ReadAllTextAsync(path);
#endif
    }

    public static void AppendAllText(string path, string content)
    {
#if NETFRAMEWORK
        var sw = File.AppendText(path);
        sw.Write(content);
        sw.Flush();
        sw.Close();
#else
        File.AppendAllText(path, content);
#endif
    }
}
