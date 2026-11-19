using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;
using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class BehaviorDifferenceIdsTest
{
    private const string IdPrefix = "BD#";

    private static readonly HashSet<string> MarkerCalls =
    [
        $"{nameof(Skip)}.{nameof(Skip.OnOldDriver)}",
        $"{nameof(Skip)}.{nameof(Skip.OnNewDriver)}",
        $"{nameof(BehaviorDifference)}.{nameof(BehaviorDifference.NewDriverOnly)}",
        $"{nameof(BehaviorDifference)}.{nameof(BehaviorDifference.OldDriverOnly)}",
    ];

    private static readonly string[] ExpectedViolations =
    [
        $"{nameof(TestDriverTest)}.cs: testdriver-unit",

        // TODO Needs the TEXT and FIXED converters; PR stack #3380 unskips this test.
        "DataReaderTest.cs: BD#reader-get-ordinal-missing-column-throws",
        "DataReaderTest.cs: BD#reader-access-without-current-row-error-code",
        "DataReaderTest.cs: BD#reader-conversion-failures-throw-invalid-data-conversion",
        "DataReaderTest.cs: BD#reader-get-string-invariant-culture",
        "DataReaderTest.cs: BD#reader-fixed-beyond-int64-returns-decimal-on-json"
    ];

    [SnowflakeFact]
    public void EveryMarkerMustReferenceARegisteredBehaviorDifference()
    {
        var registered = RegisteredIds();

        var violations = AssembliesMetadata.DriverTestAssemblies
            .SelectMany(AssemblyUtil.GetSyntaxRoots)
            .SelectMany(file => MarkerIds(file.Root).Select(id => (file.RelativePath, Id: id)))
            .Where(marker => !registered.Contains(marker.Id))
            .Select(marker => $"{marker.RelativePath}: {marker.Id}")
            .Distinct()
            .ToList();

        AssertUtil.AssertOnViolations(ExpectedViolations, violations);
    }

    private static HashSet<string> RegisteredIds()
    {
        var registry = Path.Combine(AssemblyUtil.SolutionRoot, "behavior_differences");
        return Directory.GetFiles(registry, "*.yaml", SearchOption.AllDirectories)
            .Select(file => IdPrefix + Path.GetFileNameWithoutExtension(file))
            .ToHashSet(StringComparer.Ordinal);
    }

    private static IEnumerable<string> MarkerIds(SyntaxNode root) =>
        root.DescendantNodes()
            .OfType<InvocationExpressionSyntax>()
            .Where(call => call.Expression is MemberAccessExpressionSyntax access && MarkerCalls.Contains(access.ToString()))
            .Select(call => call.ArgumentList.Arguments.FirstOrDefault()?.Expression)
            .Select(argument => argument is LiteralExpressionSyntax literal && literal.IsKind(SyntaxKind.StringLiteralExpression)
                ? literal.Token.ValueText
                : $"<non-literal {argument}>");
}
