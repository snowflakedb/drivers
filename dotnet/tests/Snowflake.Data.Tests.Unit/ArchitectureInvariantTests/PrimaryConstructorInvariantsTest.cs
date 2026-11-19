using Microsoft.CodeAnalysis.CSharp.Syntax;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;
using AssemblyMetadata = Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata.AssemblyMetadata;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class PrimaryConstructorInvariantsTest
{
    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestClassesAndStructsDoNotDeclarePrimaryConstructors(string assemblyMetadata)
    {
        var violations = FindPrimaryConstructors(assemblyMetadata);

        AssertUtil.AssertOnViolations([], violations);
    }

    private static IEnumerable<string> FindPrimaryConstructors(AssemblyMetadata assemblyMetadata) =>
        AssemblyUtil.GetSyntaxRoots(assemblyMetadata)
            .SelectMany(file => file.Root.DescendantNodes()
                .OfType<TypeDeclarationSyntax>()
                .Where(type => type is ClassDeclarationSyntax or StructDeclarationSyntax && type.ParameterList is not null)
                .Select(type => $"{file.RelativePath}:{type.GetLocation().GetLineSpan().StartLinePosition.Line + 1}: {type.Identifier.Text}"))
            .ToArray();
}
