using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class ReferenceCounterpartTest
{
    [SnowflakeFact]
    public void EveryITFixtureTestClassMustHaveReferenceCounterpart()
    {
        var testsAssembly = AssemblyUtil.LoadAssembly(AssembliesMetadata.TestsAssembly);
        var referenceAssembly = AssemblyUtil.LoadAssembly(AssembliesMetadata.RegressionTestsAssembly);

        var iClassFixtureOfITFixture = typeof(IClassFixture<>).MakeGenericType(typeof(ITFixture));

        var testClasses = testsAssembly.GetTypes()
            .Where(t => t is { IsClass: true, IsAbstract: false })
            .Where(iClassFixtureOfITFixture.IsAssignableFrom)
            .ToList();

        var referenceTypes = referenceAssembly.GetTypes()
            .Where(t => t is { IsClass: true, IsAbstract: false })
            .ToList();

        var violations = testClasses
            .Where(testClass => !referenceTypes.Any(refType => refType.Name == testClass.Name && testClass.IsAssignableFrom(refType)))
            .Select(t => t.FullName ?? t.Name)
            .ToList();

        List<string> expectedViolations =
        [
            typeof(ConnectionTest).FullName!,
            typeof(DataReaderTest).FullName!,
        ];

        AssertUtil.AssertOnViolations(expectedViolations, violations);
    }
}
