using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;
using Snowflake.Data.Tests.Discovery;
using Snowflake.Data.Tests.Utilities;
using AssemblyMetadata = Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata.AssemblyMetadata;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class AsyncMethodInvariantsTest
{
    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestConfigureAwaitFalse_AllAwaitsInProductionCodeMustHaveConfigureAwaitFalse(string assemblyMetadata)
    {
        var violations = new List<string>();

        foreach (var (relativePath, root) in AssemblyUtil.GetSyntaxRoots(assemblyMetadata))
            foreach (var awaitExpr in root.DescendantNodes().OfType<AwaitExpressionSyntax>())
            {
                InvocationExpressionSyntax? invocation;
                if (awaitExpr.Expression is not InvocationExpressionSyntax invocationExpr)
                {
                    invocation = null;
                }
                else if (invocationExpr is { Expression: MemberAccessExpressionSyntax { Name.Identifier.Text: "ConfigureAwait" }, ArgumentList.Arguments.Count: 1 })
                {
                    invocation = invocationExpr;
                    var argument = invocation.ArgumentList.Arguments[0].Expression;
                    if (argument is LiteralExpressionSyntax literal && literal.IsKind(SyntaxKind.FalseLiteralExpression))
                        continue;
                }
                else
                {
                    invocation = invocationExpr;
                }

                var expressionStr = awaitExpr.Expression.ToString().Trim().Replace("\n", string.Empty);

                if (invocation?.Expression.ToString().Equals("Task.Yield") == true)
                    continue;

                if (invocation?.Expression.ToString().Equals("Task.Run") == true)
                    expressionStr = "Task.Run(...);";

                var lineNumber = awaitExpr.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                violations.Add($"{relativePath}:{lineNumber}: {expressionStr}");
            }

        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] = ExpectedDummyExceptions
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exceptions);
        AssertUtil.AssertOnViolations(exceptions ?? [], violations);
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestAsyncSuffix_AllAsyncMethodsMustEndWithAsync(string assemblyMetadata)
    {
        var violations = new List<string>();

        foreach (var (relativePath, root) in AssemblyUtil.GetSyntaxRoots(assemblyMetadata))
            foreach (var method in root.DescendantNodes().OfType<MethodDeclarationSyntax>())
            {
                if (IsOverride(method) || !IsAsyncMethod(method))
                    continue;

                if (method.Identifier.Text.EndsWith("Async", StringComparison.Ordinal))
                    continue;

                var lineNumber = method.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                violations.Add($"{relativePath}:{lineNumber}: {method.Identifier.Text}");
            }

        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] = ExpectedDummyExceptions2,
            [AssembliesMetadata.TestsCommonsAssembly] =
            [
                "Discovery/SnowflakeTestCaseRunner.cs", // cover old xunit and the new one
                "Discovery/SnowflakeTestCaseDiscoverer", // cover old xunit and the new one
                $"Discovery/{nameof(SnowflakeTestCase)}.cs",
            ],
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exceptions);
        AssertUtil.AssertOnViolations(exceptions ?? [], violations, new StartsWithEqualityComparer());
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestCancellationToken_AllAsyncMethodsMustAcceptCancellationToken(string assemblyMetadata)
    {
        var violations = new List<string>();

        foreach (var (relativePath, root) in AssemblyUtil.GetSyntaxRoots(assemblyMetadata))
            foreach (var method in root.DescendantNodes().OfType<MethodDeclarationSyntax>())
            {
                if (IsOverride(method) || !IsAsyncMethod(method) || IsTestMethod(method))
                    continue;

                if (method.ParameterList.Parameters.Any(p => GetTypeName(p.Type) is nameof(CancellationToken)))
                    continue;

                var lineNumber = method.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                violations.Add($"{relativePath}:{lineNumber}: {method.Identifier.Text}");
            }

        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] = ExpectedDummyExceptions3,
            [AssembliesMetadata.TestsCommonsAssembly] =
            [
                $"Fixtures/{nameof(ITFixture)}.cs",
                "Discovery/SnowflakeTestCaseRunner.cs", // cover old xunit and the new one
                "Discovery/SnowflakeTestCaseDiscoverer", // cover old xunit and the new one
                $"Discovery/{nameof(SnowflakeTestCase)}.cs",
            ],
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exceptions);
        AssertUtil.AssertOnViolations(exceptions ?? [], violations, new StartsWithEqualityComparer());
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverProdAssemblies))]
    public void TestNoDefaultCancellationToken_CallSitesMustNotPassDefaultOrNone(string assemblyMetadata)
    {
        var violations = new List<string>();

        foreach (var (relativePath, root) in AssemblyUtil.GetSyntaxRoots(assemblyMetadata))
            foreach (var invocation in root.DescendantNodes().OfType<InvocationExpressionSyntax>())
            {
                var methodName = GetInvokedMethodName(invocation);

                foreach (var argument in invocation.ArgumentList.Arguments)
                {
                    if (!IsDefaultCancellationToken(argument.Expression, methodName))
                        continue;

                    var lineNumber = argument.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                    violations.Add($"{relativePath}:{lineNumber}: {methodName}({argument.Expression})");
                }
            }

        AssertUtil.AssertOnViolations([], violations);
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestNoBlockingCalls_NoWaitGetAwaiterGetResultOrResult(string assemblyMetadata)
    {
        var violations = new List<string>();

        foreach (var (relativePath, root) in AssemblyUtil.GetSyntaxRoots(assemblyMetadata))
        {
            foreach (var invocation in root.DescendantNodes().OfType<InvocationExpressionSyntax>())
            {
                if (invocation is { Expression: MemberAccessExpressionSyntax { Name.Identifier.Text: "Wait" } waitAccess, ArgumentList.Arguments.Count: 0 })
                {
                    var lineNumber = invocation.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                    violations.Add($"{relativePath}:{lineNumber}: {waitAccess.Expression}.Wait()");
                    continue;
                }

                if (invocation.Expression is not MemberAccessExpressionSyntax
                    {
                        Name.Identifier.Text: "GetResult",
                        Expression: InvocationExpressionSyntax
                        {
                            Expression: MemberAccessExpressionSyntax
                            {
                                Name.Identifier.Text: "GetAwaiter"
                            } getAwaiterAccess
                        }
                    })
                    continue;

                var lineNumber2 = invocation.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                var expr = getAwaiterAccess.Expression.ToString().Trim().Replace("\n", string.Empty);
                violations.Add($"{relativePath}:{lineNumber2}: {expr}.GetAwaiter().GetResult()");
            }

            foreach (var memberAccess in root.DescendantNodes().OfType<MemberAccessExpressionSyntax>())
            {
                if (memberAccess.Name.Identifier.Text != "Result")
                    continue;
                if (memberAccess.Parent is MemberAccessExpressionSyntax)
                    continue;
                if (!IsTaskExpression(memberAccess.Expression))
                    continue;

                var lineNumber = memberAccess.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                var expr = memberAccess.Expression.ToString().Trim().Replace("\n", string.Empty);
                violations.Add($"{relativePath}:{lineNumber}: {expr}.Result");
            }
        }

        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.RootAssembly] =
            [
                "SnowflakeDbDataReader.cs:71: _arrowStream.ReadNextRecordBatchAsync().GetAwaiter().GetResult()", // TODO this shouldn't do that
            ],
            [AssembliesMetadata.TestsAssembly] = ExpectedDummyExceptions4,
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exceptions);
        AssertUtil.AssertOnViolations(exceptions ?? [], violations);
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestNoAsyncVoid_AsyncVoidMethodsMustNotExist(string assemblyMetadata)
    {
        var violations = new List<string>();

        foreach (var (relativePath, root) in AssemblyUtil.GetSyntaxRoots(assemblyMetadata))
            foreach (var method in root.DescendantNodes().OfType<MethodDeclarationSyntax>())
            {
                if (!method.Modifiers.Any(SyntaxKind.AsyncKeyword))
                    continue;
                if (method.ReturnType.ToString() != "void")
                    continue;

                var lineNumber = method.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                violations.Add($"{relativePath}:{lineNumber}: {method.Identifier.Text}");
            }

        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] = ExpectedDummyExceptions5,
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exceptions);
        AssertUtil.AssertOnViolations(exceptions ?? [], violations);
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestNoThreadSleep_ThreadSleepMustNotBeUsed(string assemblyMetadata)
    {
        var violations = new List<string>();

        foreach (var (relativePath, root) in AssemblyUtil.GetSyntaxRoots(assemblyMetadata))
            foreach (var invocation in root.DescendantNodes().OfType<InvocationExpressionSyntax>())
            {
                if (invocation.Expression is not MemberAccessExpressionSyntax memberAccess)
                    continue;

                var fullExpression = memberAccess.ToString();
                if (fullExpression is not ("Thread.Sleep" or "Threading.Thread.Sleep" or "System.Threading.Thread.Sleep"))
                    continue;

                var lineNumber = invocation.GetLocation().GetLineSpan().StartLinePosition.Line + 1;
                violations.Add($"{relativePath}:{lineNumber}: {fullExpression}(...)");
            }

        Dictionary<AssemblyMetadata, string[]> exceptionsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] = ExpectedDummyExceptions7,
        };
        exceptionsMap.TryGetValue(assemblyMetadata, out var exceptions);
        AssertUtil.AssertOnViolations(exceptions ?? [], violations);
    }

    private static bool IsTaskExpression(ExpressionSyntax expression)
    {
        if (expression is InvocationExpressionSyntax invocation && GetInvokedMethodName(invocation).EndsWith("Async", StringComparison.Ordinal))
            return true;

        if (expression is not InvocationExpressionSyntax { Expression: MemberAccessExpressionSyntax taskMember })
            return false;

        return taskMember.Expression.ToString() is "Task" or "ValueTask";
    }

    private static bool IsAsyncMethod(MethodDeclarationSyntax method)
    {
        if (method.Modifiers.Any(SyntaxKind.AsyncKeyword))
            return true;

        return GetBaseTypeName(method.ReturnType) is "Task" or "ValueTask";
    }

    private static bool IsTestMethod(MethodDeclarationSyntax method)
    {
        var factName = nameof(SnowflakeFactAttribute).Replace(nameof(Attribute), "");
        var theoryName = nameof(SnowflakeTheoryAttribute).Replace(nameof(Attribute), "");
        if (method.AttributeLists.SelectMany(x => x.Attributes).Any(x => x.Name.ToString() == factName || x.Name.ToString() == theoryName))
            return true;

        return false;
    }

    private static string GetBaseTypeName(TypeSyntax? type) => type switch
    {
        GenericNameSyntax generic => generic.Identifier.Text,
        IdentifierNameSyntax identifier => identifier.Identifier.Text,
        QualifiedNameSyntax qualified => GetBaseTypeName(qualified.Right),
        _ => string.Empty,
    };

    private static string? GetTypeName(TypeSyntax? type) => type switch
    {
        IdentifierNameSyntax identifier => identifier.Identifier.Text,
        QualifiedNameSyntax qualified => GetTypeName(qualified.Right),
        NullableTypeSyntax nullable => GetTypeName(nullable.ElementType),
        _ => null,
    };

    private static bool IsOverride(MethodDeclarationSyntax method) =>
        method.Modifiers.Any(SyntaxKind.OverrideKeyword);

    private static bool IsDefaultCancellationToken(ExpressionSyntax expression, string methodName)
    {
        switch (expression)
        {
            case DefaultExpressionSyntax defaultExpr:
                return GetTypeName(defaultExpr.Type) is "CancellationToken";
            case MemberAccessExpressionSyntax { Name.Identifier.Text: "None" } memberAccess:
                return memberAccess.Expression.ToString() is "CancellationToken" or "System.Threading.CancellationToken";
        }

        if (expression.IsKind(SyntaxKind.DefaultLiteralExpression))
            return methodName.EndsWith("Async", StringComparison.Ordinal);

        return false;
    }

    private static string GetInvokedMethodName(InvocationExpressionSyntax invocation) => invocation.Expression switch
    {
        MemberAccessExpressionSyntax memberAccess => memberAccess.Name.Identifier.Text,
        IdentifierNameSyntax identifier => identifier.Identifier.Text,
        _ => invocation.Expression.ToString(),
    };

    private static readonly string[] ExpectedDummyExceptions = [
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:25: {nameof(Task)}.{nameof(Task.Delay)}(11)",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:10: {nameof(Task)}.{nameof(Task.Run)}(...);",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:16: {nameof(Task)}.{nameof(Task.Run)}(...);",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:11: {nameof(Task)}.{nameof(Task.Run)}(...);",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:8: {nameof(Task)}.{nameof(Task.Delay)}(1)",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:17: {nameof(AsyncInvariantsDummy.DoSomething2)}()",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:15: {nameof(Task)}.{nameof(Task.Run)}(...);",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:14: {nameof(Task)}.{nameof(Task.Run)}(...);",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:13: {nameof(Task)}.{nameof(Task.Run)}(...);",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:12: {nameof(Task)}.{nameof(Task.Run)}(...);",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:18: {nameof(Task)}.{nameof(Task.Run)}(...);",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:18: {nameof(Task)}.{nameof(Task.Delay)}(6)",
    ];

    private static readonly string[] ExpectedDummyExceptions2 = [
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy)}.cs:7: {nameof(AsyncMethodDummy.DoWork)}",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy)}.cs:13: {nameof(AsyncMethodDummy.RunJob)}",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy2)}.cs:6: {nameof(AsyncMethodDummy2.FireAndForget)}",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:6: {nameof(AsyncInvariantsDummy.DoSomething)}",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:21: {nameof(AsyncInvariantsDummy.DoSomething2)}",
    ];

    private static readonly string[] ExpectedDummyExceptions3 = [
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy)}.cs:7: {nameof(AsyncMethodDummy.DoWork)}",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy)}.cs:13: {nameof(AsyncMethodDummy.RunJob)}",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy)}.cs:19: {nameof(AsyncMethodDummy.ProcessAsync)}",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy2)}.cs:6: {nameof(AsyncMethodDummy2.FireAndForget)}",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:6: {nameof(AsyncInvariantsDummy.DoSomething)}",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncInvariantsDummy)}.cs:21: {nameof(AsyncInvariantsDummy.DoSomething2)}",
    ];

    private static readonly string[] ExpectedDummyExceptions4 = [
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy)}.cs:45: Task.Delay(1).GetAwaiter().GetResult()",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy)}.cs:51: Task.Delay(1).Wait()",
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy)}.cs:57: Task.FromResult(42).Result",
    ];

    private static readonly string[] ExpectedDummyExceptions5 = [
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy2)}.cs:6: {nameof(AsyncMethodDummy2.FireAndForget)}",
    ];

    private static readonly string[] ExpectedDummyExceptions7 = [
        $"ArchitectureInvariantTests/Dummies/{nameof(AsyncMethodDummy2)}.cs:8: Thread.Sleep(...)",
    ];
}
