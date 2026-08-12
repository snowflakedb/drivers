using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;
using Snowflake.Data.Tests.Utilities;
using AssemblyMetadata = Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata.AssemblyMetadata;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class DynamicCodeTest
{
    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestNoDynamic_DynamicVariablesMustNotExist(string assemblyMetadata)
    {
        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] = [ExpectedDummyExceptions],
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exceptions);
        TestNoDynamicNodes<VariableDeclarationSyntax>(assemblyMetadata, FormatVariableViolation, exceptions ?? []);
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestNoDynamic_DynamicParametersMustNotExist(string assemblyMetadata) =>
        TestNoDynamicNodes<ParameterSyntax>(assemblyMetadata, FormatParameterViolation);

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestNoDynamic_DynamicReturnTypesMustNotExist(string assemblyMetadata) =>
        TestNoDynamicNodes<MethodDeclarationSyntax>(assemblyMetadata, FormatReturnTypeViolation);

    private static void TestNoDynamicNodes<T>(AssemblyMetadata assemblyMetadata, Func<T, string, string?> formatter, params string[] exceptions) where T : SyntaxNode
    {
        var violations = new List<string>();

        foreach (var (relativePath, root) in AssemblyUtil.GetSyntaxRoots(assemblyMetadata))
            foreach (var node in root.DescendantNodes().OfType<T>())
            {
                var violation = formatter(node, relativePath);
                if (violation is not null)
                    violations.Add(violation);
            }

        AssertUtil.AssertOnViolations(exceptions, violations, new StartsWithEqualityComparer());
    }

    private static string? FormatVariableViolation(VariableDeclarationSyntax variable, string relativePath)
    {
        if (variable.Type is not IdentifierNameSyntax { Identifier.Text: "dynamic" })
            return null;

        var lineNumber = variable.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
        var names = string.Join(", ", variable.Variables.Select(v => v.Identifier.Text));
        return $"{relativePath}:{lineNumber}: dynamic {names}";
    }

    private static string? FormatParameterViolation(ParameterSyntax parameter, string relativePath)
    {
        if (parameter.Type is not IdentifierNameSyntax { Identifier.Text: "dynamic" })
            return null;

        var lineNumber = parameter.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
        return $"{relativePath}:{lineNumber}: dynamic {parameter.Identifier.Text}";
    }

    private static string? FormatReturnTypeViolation(MethodDeclarationSyntax method, string relativePath)
    {
        if (method.ReturnType is not IdentifierNameSyntax { Identifier.Text: "dynamic" })
            return null;

        var lineNumber = method.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
        return $"{relativePath}:{lineNumber}: dynamic return in {method.Identifier.Text}";
    }

    private const string ExpectedDummyExceptions = $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy2)}.cs:12: dynamic x";
}
