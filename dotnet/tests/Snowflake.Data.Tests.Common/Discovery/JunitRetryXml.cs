using System.Collections.Concurrent;
using System.Text;
using System.Xml;

namespace Snowflake.Data.Tests.Discovery;

/// <summary>
/// Writes Jenkins/Surefire JUnit so a fail-then-pass retry is a passing
/// <c>testcase</c> with <c>flakyFailure</c> children, and an exhausted retry
/// keeps <c>failure</c> plus <c>rerunFailure</c>. Set <c>SNOWFLAKE_JUNIT_PATH</c>
/// to enable; local <c>dotnet run</c> without it is a no-op.
/// </summary>
public static class JunitRetryXml
{
    public readonly struct AttemptFailure
    {
        public AttemptFailure(string type, string message, string? stack)
        {
            Type = type;
            Message = message;
            Stack = stack;
        }

        public string Type { get; }
        public string Message { get; }
        public string? Stack { get; }
    }

    public enum CaseStatus
    {
        Passed,
        Failed,
        Skipped,
        Error,
    }

    private static readonly ConcurrentDictionary<string, string> Cases = new(StringComparer.Ordinal);
    private static readonly object FileLock = new();
    private static readonly string? OutputPath = BlankToNull(Environment.GetEnvironmentVariable("SNOWFLAKE_JUNIT_PATH"));

    static JunitRetryXml()
    {
        if (OutputPath is null)
            return;
        AppDomain.CurrentDomain.ProcessExit += (_, _) => Flush();
    }

    public static void Record(
        string classname,
        string name,
        decimal timeSeconds,
        CaseStatus status,
        IReadOnlyList<AttemptFailure> priorFailures,
        AttemptFailure? finalFailure)
    {
        if (OutputPath is null)
            return;

        var key = classname + "::" + name;
        Cases[key] = RenderCase(classname, name, timeSeconds, status, priorFailures, finalFailure);
        Flush();
    }

    public static string RenderDocument(IEnumerable<string> testcases)
    {
        var body = string.Join("\n", testcases);
        return """
            <?xml version="1.0" encoding="utf-8"?>
            <testsuites>
            <testsuite name="Snowflake.Data.Tests">
            """ + "\n" + body + "\n</testsuite>\n</testsuites>\n";
    }

    public static string RenderCase(
        string classname,
        string name,
        decimal timeSeconds,
        CaseStatus status,
        IReadOnlyList<AttemptFailure> priorFailures,
        AttemptFailure? finalFailure)
    {
        var sb = new StringBuilder();
        using var writer = XmlWriter.Create(sb, new XmlWriterSettings
        {
            OmitXmlDeclaration = true,
            ConformanceLevel = ConformanceLevel.Fragment,
            Indent = true,
        });
        writer.WriteStartElement("testcase");
        writer.WriteAttributeString("classname", classname);
        writer.WriteAttributeString("name", name);
        writer.WriteAttributeString("time", timeSeconds.ToString(System.Globalization.CultureInfo.InvariantCulture));

        switch (status)
        {
            case CaseStatus.Skipped:
                writer.WriteStartElement("skipped");
                writer.WriteEndElement();
                break;
            case CaseStatus.Failed:
            case CaseStatus.Error:
                WriteFailure(writer, status == CaseStatus.Error ? "error" : "failure", finalFailure);
                foreach (var prior in priorFailures)
                    WriteFailure(writer, "rerunFailure", prior);
                break;
            default:
                foreach (var prior in priorFailures)
                    WriteFailure(writer, "flakyFailure", prior);
                break;
        }

        writer.WriteEndElement();
        writer.Flush();
        return sb.ToString();
    }

    private static void WriteFailure(XmlWriter writer, string element, AttemptFailure? failure)
    {
        var value = failure ?? new AttemptFailure("Exception", "", null);
        writer.WriteStartElement(element);
        writer.WriteAttributeString("type", value.Type);
        writer.WriteAttributeString("message", Truncate(value.Message));
        if (!string.IsNullOrEmpty(value.Stack))
            writer.WriteString(Truncate(value.Stack));
        writer.WriteEndElement();
    }

    private static void Flush()
    {
        if (OutputPath is null)
            return;

        lock (FileLock)
        {
            var dir = Path.GetDirectoryName(OutputPath);
            if (!string.IsNullOrEmpty(dir))
                Directory.CreateDirectory(dir);
            File.WriteAllText(OutputPath, RenderDocument(Cases.Values.OrderBy(static c => c, StringComparer.Ordinal)));
        }
    }

    private static string Truncate(string? text)
    {
        var value = text ?? string.Empty;
        return value.Length <= 2000 ? value : value.Substring(0, 2000);
    }

    private static string? BlankToNull(string? value) =>
        string.IsNullOrWhiteSpace(value) ? null : value;
}
