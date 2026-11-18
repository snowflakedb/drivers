using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Snowflake.Data.Core;
using Snowflake.Data.Core.Handles;
using Snowflake.Data.Interop;
using Snowflake.Data.Interop.Handles;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

/// <summary>
/// Driver code disposes the native handles below through a <c>using</c> scope: each is created as the initializer
/// of a <c>using</c> declaration or statement, or inside a listed factory that returns it and disposes it in a
/// rethrowing <c>catch</c>, and every call to such a factory is itself the initializer of a <c>using</c>. A listed
/// ownership transfer may also create a handle outside a <c>using</c> when it disposes it in a rethrowing <c>catch</c>,
/// because the object it hands the handle to disposes it from then on. A listed out-parameter factory creates a handle
/// only by assigning it to one of its <c>out</c> parameters, and every call to it disposes the variable it passes for
/// that parameter through a <c>using</c> in the same method.
/// </summary>
[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class UsingScopedHandlesTest
{
    private const string FactoryFile = $"Core/{nameof(SnowflakeStatement)}.cs";
    private const string NativeStreamFile = $"Interop/{nameof(NativeArrowStream)}.cs";
    private const string FactoryMethod = "CreateNativeStatementAsync";
    private const string LeaseMethod = nameof(NativeArrowStream.TryEnter);
    private const string TransferMethod = "OpenReaderAsync";

    private static readonly HashSet<string> ScopedTypes =
    [
        nameof(NativeStatementHandle),
        nameof(NativeResultSetHandle),
        nameof(ArrowArrayHandle),
        nameof(ArrowSchemaHandle),
        nameof(NativeArrowStream),
        nameof(NativeArrowStream.Lease)
    ];

    private static readonly HashSet<(string File, string Method)> Factories =
    [
        (FactoryFile, FactoryMethod),
    ];

    private static readonly HashSet<(string File, string Method)> OutParameterFactories =
    [
        (NativeStreamFile, LeaseMethod),
    ];

    private static readonly HashSet<string> OutParameterFactoryNames = [.. OutParameterFactories.Select(factory => factory.Method)];

    private static readonly HashSet<(string File, string Method)> OwnershipTransfers =
    [
        (FactoryFile, TransferMethod),
    ];

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestUsingScopedHandles_DriverCodeDisposesNativeHandlesThroughUsing()
    {
        var violations = AssemblyUtil.GetSyntaxRoots(AssembliesMetadata.RootAssembly)
            .SelectMany(file => FindViolations(file.RelativePath, file.Root));

        AssertUtil.AssertOnViolations([], violations);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestUsingScopedHandles_UsingDeclarationsStatementsFactoriesAndOwnershipTransfersPass()
    {
        const string source = """
            class C
            {
                void A() { using var Handle = new ArrowArrayHandle(); }
                void B() { using (var resultSet = new NativeResultSetHandle(d, h, l)) { } }
                void D() { using ArrowSchemaHandle Handle = new(); }
                async Task E() { using var statement = await CreateNativeStatementAsync(ctx, false, t).ConfigureAwait(false); }
                async Task<NativeStatementHandle> CreateNativeStatementAsync(object ctx, bool a, CancellationToken t)
                {
                    var statement = new NativeStatementHandle(d, h, l);
                    try
                    {
                        await Set(statement).ConfigureAwait(false);
                        return statement;
                    }
                    catch
                    {
                        statement.Dispose();
                        throw;
                    }
                }
                async Task<object> OpenReaderAsync(object resultSet, CancellationToken t)
                {
                    var handle = new NativeResultSetHandle(d, h, l);
                    try
                    {
                        return Create(handle, await Fetch(t).ConfigureAwait(false));
                    }
                    catch
                    {
                        handle.Dispose();
                        throw;
                    }
                }
            }
            """;

        Assert.Empty(FindViolations(FactoryFile, CSharpSyntaxTree.ParseText(source).GetRoot()));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestUsingScopedHandles_HandlesOutsideUsingAreReported()
    {
        const string source = """
            class C
            {
                void A() { var Handle = new ArrowArrayHandle(); }
                void B() { ArrowSchemaHandle Handle = new(); }
                void D() { Keep(new NativeResultSetHandle(d, h, l)); }
                async Task E() { var statement = await CreateNativeStatementAsync(ctx, false, t); }
                async Task<NativeStatementHandle> CreateNativeStatementAsync(object ctx, bool a, CancellationToken t)
                {
                    var statement = new NativeStatementHandle(d, h, l);
                    await Set(statement);
                    return statement;
                }
                NativeStatementHandle Other() => new NativeStatementHandle(d, h, l);
                object OpenReaderAsync(object resultSet)
                {
                    var handle = new NativeResultSetHandle(d, h, l);
                    return Create(handle);
                }
            }
            """;

        var violations = FindViolations(FactoryFile, CSharpSyntaxTree.ParseText(source).GetRoot());

        Assert.Equal(
            [
                $"{FactoryFile}:3: new {nameof(ArrowArrayHandle)}",
                $"{FactoryFile}:4: new {nameof(ArrowSchemaHandle)}",
                $"{FactoryFile}:5: new {nameof(NativeResultSetHandle)}",
                $"{FactoryFile}:9: new {nameof(NativeStatementHandle)}",
                $"{FactoryFile}:13: new {nameof(NativeStatementHandle)}",
                $"{FactoryFile}:16: new {nameof(NativeResultSetHandle)}",
                $"{FactoryFile}:6: {FactoryMethod}(...)",
            ],
            violations);
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestUsingScopedHandles_LeasesAssignedToOutParameterAndDisposedThroughUsingPass()
    {
        const string source = """
            class C
            {
                Result TryEnter(out Lease? lease)
                {
                    lease = null;
                    lease = new Lease(this);
                    return Result.Entered;
                }
                void A(NativeArrowStream stream) { stream.TryEnter(out var lease); using (lease) { } }
                void B(NativeArrowStream stream) { if (stream.TryEnter(out var lease) == Result.Entered) { using var scoped = lease; } }
                void D(NativeArrowStream stream) { Lease? lease; stream.TryEnter(out lease); using (var scoped = lease) { } }
            }
            """;

        Assert.Empty(FindViolations(NativeStreamFile, CSharpSyntaxTree.ParseText(source).GetRoot()));
    }

    [SnowflakeFact(RetriesCount = RetriesCount.NoRetries)]
    public void TestUsingScopedHandles_LeasesOutsideOutParameterOrNotDisposedThroughUsingAreReported()
    {
        const string source = """
            class C
            {
                Result TryEnter(out Lease? lease)
                {
                    var created = new Lease(this);
                    lease = created;
                    return Result.Entered;
                }
                Lease Other() => new Lease(this);
                void A(NativeArrowStream stream) { stream.TryEnter(out var lease); lease?.Dispose(); }
                void B(NativeArrowStream stream) { stream.TryEnter(out _); }
                void D(NativeArrowStream stream) { stream.TryEnter(out var _); }
            }
            """;

        var violations = FindViolations(NativeStreamFile, CSharpSyntaxTree.ParseText(source).GetRoot());

        Assert.Equal(
            [
                $"{NativeStreamFile}:5: new {nameof(NativeArrowStream.Lease)}",
                $"{NativeStreamFile}:9: new {nameof(NativeArrowStream.Lease)}",
                $"{NativeStreamFile}:10: {LeaseMethod}(...)",
                $"{NativeStreamFile}:11: {LeaseMethod}(...)",
                $"{NativeStreamFile}:12: {LeaseMethod}(...)",
            ],
            violations);
    }

    private static IEnumerable<string> FindViolations(string relativePath, SyntaxNode root)
    {
        var path = relativePath.Replace('\\', '/');
        foreach (var creation in root.DescendantNodes().OfType<BaseObjectCreationExpressionSyntax>())
        {
            var typeName = CreatedTypeName(creation);
            if (typeName is null || !ScopedTypes.Contains(typeName) || IsUsingScoped(creation))
                continue;

            var method = creation.FirstAncestorOrSelf<MethodDeclarationSyntax>();
            if (method is not null && Factories.Contains((path, method.Identifier.Text)) && IsFactoryShaped(method, creation))
                continue;

            if (method is not null && OwnershipTransfers.Contains((path, method.Identifier.Text)) && DisposesOnFailure(method, creation))
                continue;

            if (method is not null && OutParameterFactories.Contains((path, method.Identifier.Text)) && AssignsOutParameter(method, creation))
                continue;

            yield return $"{path}:{Line(creation)}: new {typeName}";
        }

        foreach (var invocation in root.DescendantNodes().OfType<InvocationExpressionSyntax>())
        {
            var name = InvokedName(invocation);
            if (name is null)
                continue;

            if (Factories.Contains((path, name)) && !IsUsingScoped(invocation))
                yield return $"{path}:{Line(invocation)}: {name}(...)";
            else if (OutParameterFactoryNames.Contains(name) && OutArgument(invocation) is { } argument && !DisposesOutVariable(invocation, argument))
                yield return $"{path}:{Line(invocation)}: {name}(...)";
        }
    }

    private static bool AssignsOutParameter(MethodDeclarationSyntax method, BaseObjectCreationExpressionSyntax creation) =>
        creation.Parent is AssignmentExpressionSyntax { Left: IdentifierNameSyntax { Identifier.Text: var target } } assignment
        && assignment.Right == creation
        && method.ParameterList.Parameters.Any(parameter =>
            parameter.Identifier.Text == target && parameter.Modifiers.Any(SyntaxKind.OutKeyword));

    private static ArgumentSyntax? OutArgument(InvocationExpressionSyntax invocation) =>
        invocation.ArgumentList.Arguments.FirstOrDefault(argument => argument.RefKindKeyword.IsKind(SyntaxKind.OutKeyword));

    private static bool DisposesOutVariable(InvocationExpressionSyntax invocation, ArgumentSyntax argument)
    {
        var variable = argument.Expression switch
        {
            DeclarationExpressionSyntax { Designation: SingleVariableDesignationSyntax designation } => designation.Identifier.Text,
            IdentifierNameSyntax { Identifier.Text: var name } when name != "_" => name,
            _ => null,
        };
        if (variable is null)
            return false;

        var scope = invocation.Ancestors().FirstOrDefault(node =>
            node is BaseMethodDeclarationSyntax or LocalFunctionStatementSyntax or AnonymousFunctionExpressionSyntax or AccessorDeclarationSyntax);
        return (scope ?? invocation.SyntaxTree.GetRoot()).DescendantNodes().Any(node => node switch
        {
            UsingStatementSyntax { Expression: IdentifierNameSyntax { Identifier.Text: var disposed } } => disposed == variable,
            UsingStatementSyntax { Declaration: { } declaration } => InitializesFrom(declaration, variable),
            LocalDeclarationStatementSyntax local when local.UsingKeyword.IsKind(SyntaxKind.UsingKeyword) => InitializesFrom(local.Declaration, variable),
            _ => false,
        });
    }

    private static bool InitializesFrom(VariableDeclarationSyntax declaration, string variable) =>
        declaration.Variables.Any(declarator => declarator.Initializer?.Value is IdentifierNameSyntax { Identifier.Text: var source } && source == variable);

    private static string? CreatedTypeName(BaseObjectCreationExpressionSyntax creation) =>
        creation switch
        {
            ObjectCreationExpressionSyntax { Type: var type } => SimpleName(type),
            ImplicitObjectCreationExpressionSyntax implicitCreation =>
                implicitCreation.Parent is EqualsValueClauseSyntax { Parent: VariableDeclaratorSyntax { Parent: VariableDeclarationSyntax declaration } }
                    ? SimpleName(declaration.Type)
                    : null,
            _ => null,
        };

    private static string? SimpleName(TypeSyntax type) =>
        type switch
        {
            IdentifierNameSyntax identifier => identifier.Identifier.Text,
            QualifiedNameSyntax qualified => qualified.Right.Identifier.Text,
            _ => null,
        };

    private static string? InvokedName(InvocationExpressionSyntax invocation) =>
        invocation.Expression switch
        {
            IdentifierNameSyntax identifier => identifier.Identifier.Text,
            MemberAccessExpressionSyntax access => access.Name.Identifier.Text,
            _ => null,
        };

    private static bool IsUsingScoped(ExpressionSyntax expression)
    {
        SyntaxNode node = expression;
        while (true)
        {
            if (node.Parent is ParenthesizedExpressionSyntax or AwaitExpressionSyntax)
            {
                node = node.Parent;
                continue;
            }

            if (node.Parent is MemberAccessExpressionSyntax { Name.Identifier.Text: nameof(Task.ConfigureAwait) } access
                && access.Parent is InvocationExpressionSyntax configured)
            {
                node = configured;
                continue;
            }

            break;
        }

        return node.Parent switch
        {
            UsingStatementSyntax usingStatement => usingStatement.Expression == node,
            EqualsValueClauseSyntax { Parent: VariableDeclaratorSyntax { Parent: VariableDeclarationSyntax declaration } } =>
                declaration.Parent is UsingStatementSyntax
                || declaration.Parent is LocalDeclarationStatementSyntax local && local.UsingKeyword.IsKind(SyntaxKind.UsingKeyword),
            _ => false,
        };
    }

    private static bool IsFactoryShaped(MethodDeclarationSyntax method, BaseObjectCreationExpressionSyntax creation)
    {
        if (creation.Parent is not EqualsValueClauseSyntax { Parent: VariableDeclaratorSyntax declarator })
            return false;

        var local = declarator.Identifier.Text;
        var returnsIt = method.DescendantNodes().OfType<ReturnStatementSyntax>()
            .Any(statement => statement.Expression is IdentifierNameSyntax { Identifier.Text: var returned } && returned == local);
        return returnsIt && DisposesOnFailure(method, creation);
    }

    private static bool DisposesOnFailure(MethodDeclarationSyntax method, BaseObjectCreationExpressionSyntax creation) =>
        creation.Parent is EqualsValueClauseSyntax { Parent: VariableDeclaratorSyntax declarator }
        && method.DescendantNodes().OfType<CatchClauseSyntax>()
            .Any(clause => DisposesAndRethrows(clause.Block, declarator.Identifier.Text));

    private static bool DisposesAndRethrows(BlockSyntax block, string local) =>
        block.DescendantNodes().OfType<InvocationExpressionSyntax>().Any(invocation =>
            invocation.Expression is MemberAccessExpressionSyntax
            {
                Expression: IdentifierNameSyntax { Identifier.Text: var target },
                Name.Identifier.Text: nameof(IDisposable.Dispose),
            } && target == local)
        && block.DescendantNodes().OfType<ThrowStatementSyntax>().Any(statement => statement.Expression is null);

    private static int Line(SyntaxNode node) => node.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
}
