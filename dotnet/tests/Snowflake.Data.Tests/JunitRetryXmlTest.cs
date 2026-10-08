using Snowflake.Data.Tests.Discovery;

namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
public sealed class JunitRetryXmlTest
{
    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void RescuedRetryEmitsFlakyFailureOnPassingCase()
    {
        var prior = new JunitRetryXml.AttemptFailure[]
        {
            new("Xunit.Sdk.TrueException", "boom-1", "at Foo.cs:1"),
            new("Xunit.Sdk.TrueException", "boom-2", "at Foo.cs:2"),
        };

        var xml = JunitRetryXml.RenderCase(
            "Snowflake.Data.Tests.FooTest",
            "Bar",
            1.5m,
            JunitRetryXml.CaseStatus.Passed,
            prior,
            finalFailure: null);

        Assert.Contains("classname=\"Snowflake.Data.Tests.FooTest\"", xml);
        Assert.Contains("name=\"Bar\"", xml);
        Assert.DoesNotContain("<failure", xml);
        Assert.Equal(2, Count(xml, "<flakyFailure"));
        Assert.Contains("message=\"boom-1\"", xml);
        Assert.Contains("at Foo.cs:1", xml);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void ExhaustedRetryKeepsFailureAndRerunFailure()
    {
        var prior = new JunitRetryXml.AttemptFailure[]
        {
            new("Exception", "first", null),
        };
        var last = new JunitRetryXml.AttemptFailure("Exception", "last", "stack-last");

        var xml = JunitRetryXml.RenderCase(
            "Cls",
            "t",
            3m,
            JunitRetryXml.CaseStatus.Failed,
            prior,
            last);

        Assert.Contains("<failure", xml);
        Assert.Contains("message=\"last\"", xml);
        Assert.Contains("<rerunFailure", xml);
        Assert.Contains("message=\"first\"", xml);
        Assert.DoesNotContain("<flakyFailure", xml);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void FirstPassHasNoRetryChildren()
    {
        var xml = JunitRetryXml.RenderCase(
            "Cls",
            "ok",
            0.1m,
            JunitRetryXml.CaseStatus.Passed,
            [],
            finalFailure: null);

        Assert.DoesNotContain("<flakyFailure", xml);
        Assert.DoesNotContain("<failure", xml);
        Assert.Contains("<testcase", xml);
    }

    private static int Count(string haystack, string needle)
    {
        var n = 0;
        for (var i = 0; (i = haystack.IndexOf(needle, i, StringComparison.Ordinal)) >= 0; i += needle.Length)
            n++;
        return n;
    }
}
