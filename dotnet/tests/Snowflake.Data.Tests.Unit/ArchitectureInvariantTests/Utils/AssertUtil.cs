namespace Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;

public static class AssertUtil
{
    internal static void AssertOnViolations(IEnumerable<string> expectedViolations, IEnumerable<string> actualViolations, params string[] toIgnore)
        => AssertOnViolations(expectedViolations, actualViolations, StringComparer.Ordinal, toIgnore);

    internal static void AssertOnViolations(IEnumerable<string> expectedViolations, IEnumerable<string> actualViolations, IEqualityComparer<string> comparer, params string[] toIgnore)
    {
        expectedViolations = expectedViolations.Concat(toIgnore).Distinct(comparer).ToArray();
        actualViolations = actualViolations.Concat(toIgnore).Select(x => x.Replace(Path.DirectorySeparatorChar, '/'));
        var expectedNotReceived = expectedViolations.Except(actualViolations, comparer).ToArray();
        var unexpected = actualViolations.Except(expectedViolations, comparer).ToArray();

        var failedCount = expectedNotReceived.Length + unexpected.Length;
        Assert.True(failedCount == 0, $"Expected, but not received: \n {string.Join(",\n", expectedNotReceived)} \n Observed unexpected: \n{string.Join(",\n", unexpected)}");
    }
}
