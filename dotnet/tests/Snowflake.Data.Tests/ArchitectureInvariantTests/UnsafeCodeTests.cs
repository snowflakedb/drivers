using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.Callback;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;
using Snowflake.Data.Tests.Utilities;
using AssemblyMetadata = Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata.AssemblyMetadata;
using SfCoreNativeMethods =
#if NETFRAMEWORK
    Snowflake.Data.Interop.TfmDependent.Framework.SfCoreNativeMethods;
#else
    Snowflake.Data.Interop.TfmDependent.SfCoreNativeMethods;
#endif

#if NETFRAMEWORK
using Snowflake.Data.Interop.TfmDependent.Framework;
#endif

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class UnsafeCodeTest
{
    private static readonly string[] AllowedUnsafeNamespacePrefixes =
    [
        typeof(SfCoreNativeMethods).Namespace!,
        typeof(Proto.IDatabaseDriverService).Namespace!,
    ];

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestUnsafeOnlyInInterop_UnsafeCodeMethodsMustBeInAllowedNamespaces(string assemblyMetadata)
    {
        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.RootAssembly] = [$"Interop/{nameof(SfCoreTransport)}.cs"]
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exception);
        TestUnsafeSyntaxNodes<MethodDeclarationSyntax>(assemblyMetadata, exception ?? []);
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestUnsafeOnlyInInterop_UnsafeCodeTypesMustBeInAllowedNamespaces(string assemblyMetadata)
    {
        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.RootAssembly] =
            [
#if NETFRAMEWORK
                $"Interop/TfmDependent/{nameof(InteropStringHelper)}.cs",
                $"Interop/TfmDependent/{nameof(SfCoreNativeMethods)}.cs",
#endif
                $"Interop/{nameof(ISfCoreInterop)}.cs",
                $"Interop/{nameof(IInteropStringHelper)}.cs",
                $"Interop/Callback/{nameof(ProtoAsyncCallbackProvider)}.cs",
                $"Interop/Callback/{nameof(LogCallbackProvider)}.cs",
                $"Interop/Callback/{nameof(SfCoreAsyncCallData)}.cs"
            ],
            [AssembliesMetadata.TestsAssembly] = ["ArchitectureInvariantTests/Dummies/ClassesDummy.cs"],
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exception);
        TestUnsafeSyntaxNodes<TypeDeclarationSyntax>(assemblyMetadata, exception ?? []);
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestUnsafeOnlyInInterop_UnsafeCodeBlocksMustBeInAllowedNamespaces(string assemblyMetadata) =>
        TestUnsafeSyntaxNodes<UnsafeStatementSyntax>(assemblyMetadata);

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestUnsafeOnlyInInterop_UnsafeCodeConstructorsMustBeInAllowedNamespaces(string assemblyMetadata)
    {
        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.RootAssembly] = ["SnowflakeDbDataReader.cs"] // TODO this shouldn't be unsafe
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exception);
        TestUnsafeSyntaxNodes<ConstructorDeclarationSyntax>(assemblyMetadata, exception ?? []);
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestUnsafeOnlyInInterop_UnsafeCodePropertiesMustBeInAllowedNamespaces(string assemblyMetadata)
    {
        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] = ["ArchitectureInvariantTests/Dummies/ClassesDummy.cs"],
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exception);
        TestUnsafeSyntaxNodes<PropertyDeclarationSyntax>(assemblyMetadata, exception ?? []);
    }

    private static string GetContainingNamespace(SyntaxNode node)
    {
        // Check for file-scoped namespace first
        var fileScopedNs = node.Ancestors().OfType<FileScopedNamespaceDeclarationSyntax>().FirstOrDefault();
        if (fileScopedNs is not null)
            return fileScopedNs.Name.ToString();

        // Check for block-scoped namespace
        var blockNs = node.Ancestors().OfType<NamespaceDeclarationSyntax>().FirstOrDefault();
        if (blockNs is not null)
            return blockNs.Name.ToString();

        return string.Empty;
    }

    private static void TestUnsafeSyntaxNodes<T>(AssemblyMetadata assemblyName, params string[] exceptions) where T : SyntaxNode
    {
        var violations = new List<string>();
        foreach (var (relativePath, root) in AssemblyUtil.GetSyntaxRoots(assemblyName))
        {
            var nodes = root.DescendantNodes().OfType<T>()
                .Where(m => m is not MemberDeclarationSyntax m2 || m2.Modifiers.Any(SyntaxKind.UnsafeKeyword));

            violations.AddRange(nodes.Select(method => new { method, ns = GetContainingNamespace(method) })
                .Where(t => !AllowedUnsafeNamespacePrefixes.Any(prefix => t.ns.StartsWith(prefix, StringComparison.Ordinal)))
                .Select(t => new { t, lineNumber = t.method.GetLocation().GetLineSpan().StartLinePosition.Line + 1 })
                .Select(t => $"{relativePath}:{t.lineNumber}: in {t.t.ns}"));
        }

        AssertUtil.AssertOnViolations(exceptions, violations, new StartsWithEqualityComparer());
    }
}
