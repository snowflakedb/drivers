using Xunit.Internal;
using Xunit.Sdk;

namespace Snowflake.Data.Tests.Discovery;

public sealed class SnowflakeTestCaseDiscoverer : IXunitTestCaseDiscoverer
{
    public ValueTask<IReadOnlyCollection<IXunitTestCase>> Discover(
        ITestFrameworkDiscoveryOptions discoveryOptions,
        IXunitTestMethod testMethod,
        IFactAttribute factAttribute)
    {
        var retriesCountEnum = (factAttribute as SnowflakeFactAttribute)?.RetriesCount ?? RetriesCount.NoRetries;
        var retriesCount = (int)retriesCountEnum;
        var details = TestIntrospectionHelper.GetTestCaseDetails(discoveryOptions, testMethod, factAttribute);
        var testCase = new SnowflakeTestCase(
            maxRetries: retriesCount,
            testMethod: testMethod,
            testCaseDisplayName: details.TestCaseDisplayName,
            uniqueID: details.UniqueID,
            @explicit: details.Explicit,
            skipExceptions: details.SkipExceptions,
            skipReason: details.SkipReason,
            skipType: details.SkipType,
            skipUnless: details.SkipUnless,
            skipWhen: details.SkipWhen,
            traits: testMethod.Traits.ToReadWrite(StringComparer.OrdinalIgnoreCase),
            timeout: details.Timeout
        );

        return new([testCase]);
    }
}

public sealed class SnowflakeTheoryDiscoverer : TheoryDiscoverer
{
    public override async ValueTask<IReadOnlyCollection<IXunitTestCase>> Discover(
        ITestFrameworkDiscoveryOptions discoveryOptions,
        IXunitTestMethod testMethod,
        IFactAttribute factAttribute)
    {

        discoveryOptions.SetValue("xunit.discovery.PreEnumerateTheories", "True");
        var theoryAttribute = (SnowflakeTheoryAttribute)factAttribute;
        var retriesCountEnum = theoryAttribute.RetriesCount;
        var retriesCount = (int)retriesCountEnum;

        // Delegate to base discovery, then wrap results
        var baseCases = await base.Discover(discoveryOptions, testMethod, factAttribute).ConfigureAwait(false);

        var wrappedCases = new List<IXunitTestCase>(baseCases.Count);
        foreach (var baseCase in baseCases)
        {
            if (baseCase is XunitTestCase xtc)
            {
                // Re-create as SnowflakeTestCase with retry support
                var details = TestIntrospectionHelper.GetTestCaseDetails(discoveryOptions, testMethod, factAttribute);
                wrappedCases.Add(new SnowflakeTestCase(
                    retriesCount,
                    testMethod,
                    baseCase.TestCaseDisplayName,
                    baseCase.UniqueID,
                    details.Explicit,
                    details.SkipExceptions,
                    details.SkipReason,
                    details.SkipType,
                    details.SkipUnless,
                    details.SkipWhen,
                    testMethod.Traits.ToReadWrite(StringComparer.OrdinalIgnoreCase),
                    xtc.TestMethodArguments,
                    timeout: details.Timeout
                ));
            }
            else
            {
                wrappedCases.Add(baseCase);
            }
        }

        return wrappedCases;
    }
}
