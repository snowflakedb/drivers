using Microsoft.CodeAnalysis.CSharp.Syntax;
using Snowflake.Data.Interop;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;
using Snowflake.Data.Tests.Utilities;
using AssemblyMetadata = Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata.AssemblyMetadata;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

/// <summary>
/// The callbacks of an Arrow C stream are called only by <see cref="NativeArrowStream"/>, which admits one call at
/// a time and keeps the stream alive until the call ends.
/// </summary>
[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class ArrowStreamCallsTest
{
    private const string CallsTypeName = "ArrowStreamCalls";

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestArrowStreamCallsOnlyFromNativeArrowStream_NoOtherCodeCallsArrowStreamCallbacks(string assemblyMetadata)
    {
        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.RootAssembly] = [$"Interop/{nameof(NativeArrowStream)}.cs"],
            [AssembliesMetadata.TestsAssembly] =
            [
                $"Core/{nameof(Core.UnitStreamRpcs)}.cs",
                $"ArchitectureInvariantTests/{nameof(UnsafeCodeTest)}",
            ],
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exceptions);

        var violations = new List<string>();
        foreach (var (relativePath, root) in AssemblyUtil.GetSyntaxRoots(assemblyMetadata))
        {
            violations.AddRange(root.DescendantNodes()
                .OfType<IdentifierNameSyntax>()
                .Where(name => name.Identifier.Text == CallsTypeName)
                .Select(name => $"{relativePath}:{name.GetLocation().GetLineSpan().StartLinePosition.Line + 1}"));
        }

        AssertUtil.AssertOnViolations(exceptions ?? [], violations, new StartsWithEqualityComparer());
    }
}
