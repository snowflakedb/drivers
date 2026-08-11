using System.Reflection;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class TestsAttributesInvariantsTest
{
    [SnowflakeTheory]
    [ClassData(typeof(DriverTestAssemblies))]
    public void TestClassNaming_TestClassesMustEndWithTest(string assemblyName)
    {
        var violations = AssemblyUtil.LoadAssembly(assemblyName).GetTypes()
            .Where(t => t is { IsClass: true, IsAbstract: false })
            .Select(t => new { Type = t, Methods = t.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static) })
            .Select(x => new { x.Type, Attributes = x.Methods.SelectMany(y => y.GetCustomAttributes(true)) })
            .Where(x => x.Attributes.Any(a => a is SnowflakeFactAttribute or SnowflakeTheoryAttribute))
            .Select(x => x.Type)
            .Where(x => !x.Name.EndsWith("Test", StringComparison.Ordinal))
            .Select(x => x.FullName ?? x.Name)
            .ToList();

        Dictionary<AssemblyMetadata, List<string>> expectedViolationsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] = [typeof(TestsAttributesDummy).FullName!, typeof(TestsAttributesDummy2).FullName!]
        };
        expectedViolationsMap.TryGetValue(assemblyName, out var expectedViolations);
        AssertUtil.AssertOnViolations(expectedViolations ?? [], violations);
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverTestAssemblies))]
    public void TestNoBareFact_AllTestsMustUseSFFactOrSFTheory(string assemblyName)
    {
        var violations = new List<string>();
        var testTypes = AssemblyUtil.LoadAssembly(assemblyName).GetTypes()
            .Where(t => t is { IsClass: true, IsAbstract: false });

        foreach (var type in testTypes)
        {
            var methods = type.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static);
            foreach (var method in methods)
            {
                var attributes = method.GetCustomAttributes(true);
                var hasBareFactAttribute = attributes.Any(a =>
                    a.GetType() == typeof(FactAttribute));
                var hasBareTheoryAttribute = attributes.Any(a =>
                    a.GetType() == typeof(TheoryAttribute));

                if (hasBareFactAttribute)
                    violations.Add($"{type.FullName}.{method.Name} uses bare [{nameof(FactAttribute)}] instead of [{nameof(SnowflakeFactAttribute)}]");

                if (hasBareTheoryAttribute)
                    violations.Add($"{type.FullName}.{method.Name} uses bare [{nameof(TheoryAttribute)}] instead of [{nameof(SnowflakeTheoryAttribute)}]");
            }
        }

        Dictionary<AssemblyMetadata, List<string>> expectedViolationsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] =
            [
                $"{typeof(TestsAttributesDummy).FullName}.{nameof(TestsAttributesDummy.DummyTest)} uses bare [{nameof(FactAttribute)}] instead of [{nameof(SnowflakeFactAttribute)}]",
                $"{typeof(TestsAttributesDummy).FullName}.{nameof(TestsAttributesDummy.DummyParametrizedTest)} uses bare [{nameof(TheoryAttribute)}] instead of [{nameof(SnowflakeTheoryAttribute)}]",
            ]
        };

        expectedViolationsMap.TryGetValue(assemblyName, out var expectedViolations);
        AssertUtil.AssertOnViolations(expectedViolations ?? [], violations);
    }
}
