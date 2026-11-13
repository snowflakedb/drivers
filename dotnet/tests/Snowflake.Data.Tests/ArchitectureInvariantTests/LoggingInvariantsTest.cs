using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;
using AssemblyMetadata = Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata.AssemblyMetadata;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class LoggingInvariantsTest
{
    private const string EventIdBaseName = "EventIdBase";
    private const int EventIdRangeSize = 1000;

    private static readonly HashSet<string> LogMethodNames =
        ["Log", "LogTrace", "LogDebug", "LogInformation", "LogWarning", "LogError", "LogCritical"];

    private static readonly HashSet<string> FlatteningMembers = ["Message", "ToString", "StackTrace"];

    private static readonly string[] ExpectedDummyViolations =
    [
        $"ArchitectureInvariantTests/Dummies/{nameof(LoggingDummy)}.cs:17: ex.Message",
        $"ArchitectureInvariantTests/Dummies/{nameof(LoggingDummy)}.cs:20: ex.ToString()",
        $"ArchitectureInvariantTests/Dummies/{nameof(LoggingDummy)}.cs:23: ex",
        $"ArchitectureInvariantTests/Dummies/{nameof(LoggingDummy)}.cs:26: $\"{{ex}}\"",
    ];

    private static readonly string[] ExpectedEventIdDummyViolations =
    [
        $"ArchitectureInvariantTests/Dummies/LoggerMessageEventIdDummy.cs:7: {nameof(ValidEventIdDummyLog)}: EventIdBase 1000 is declared by 2 classes",
        $"ArchitectureInvariantTests/Dummies/LoggerMessageEventIdDummy.cs:19: {nameof(OverlappingEventIdDummyLog)}: EventIdBase 1000 is declared by 2 classes",
        $"ArchitectureInvariantTests/Dummies/LoggerMessageEventIdDummy.cs:32: {nameof(InvalidEventIdDummyLog)}.Missing: EventId is not set",
        $"ArchitectureInvariantTests/Dummies/LoggerMessageEventIdDummy.cs:36: {nameof(InvalidEventIdDummyLog)}.HardCoded: EventId 2002 is not EventIdBase + n",
        $"ArchitectureInvariantTests/Dummies/LoggerMessageEventIdDummy.cs:40: {nameof(InvalidEventIdDummyLog)}.OutOfRange: EventId offset 1000 is outside 1..999",
        $"ArchitectureInvariantTests/Dummies/LoggerMessageEventIdDummy.cs:47: {nameof(InvalidEventIdDummyLog)}.Duplicate: EventId offset 1 is already used by Unique",
        $"ArchitectureInvariantTests/Dummies/LoggerMessageEventIdDummy.cs:52: {nameof(MisalignedEventIdDummyLog)}: EventIdBase 4500 is not a positive multiple of 1000",
        $"ArchitectureInvariantTests/Dummies/LoggerMessageEventIdDummy.cs:61: {nameof(MissingBaseEventIdDummyLog)}: declares no const int EventIdBase with a literal value",
    ];

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestExceptionPassedAsExceptionArgument_LogCallsMustNotFlattenCaughtExceptions(string assemblyMetadata)
    {
        Dictionary<AssemblyMetadata, string[]> expectedViolationsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] = ExpectedDummyViolations,
        };
        expectedViolationsMap.TryGetValue(assemblyMetadata, out var expectedViolations);

        var violations = AssemblyUtil.GetSyntaxRoots(assemblyMetadata)
            .SelectMany(file => file.Root.DescendantNodes()
                .OfType<InvocationExpressionSyntax>()
                .Where(IsLogCall)
                .SelectMany(FindFlattenedExceptionArguments)
                .Select(argument => $"{file.RelativePath}:{argument.GetLocation().GetLineSpan().StartLinePosition.Line + 1}: {argument}"))
            .ToArray();

        AssertUtil.AssertOnViolations(expectedViolations ?? [], violations);
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void TestLoggerMessageEventIds_EventIdsMustBeUniqueOffsetsFromAClassRangeBase(string assemblyMetadata)
    {
        Dictionary<AssemblyMetadata, string[]> expectedViolationsMap = new()
        {
            [AssembliesMetadata.TestsAssembly] = ExpectedEventIdDummyViolations,
        };
        expectedViolationsMap.TryGetValue(assemblyMetadata, out var expectedViolations);

        var logClasses = AssemblyUtil.GetSyntaxRoots(assemblyMetadata)
            .SelectMany(file => file.Root.DescendantNodes()
                .OfType<ClassDeclarationSyntax>()
                .Select(declaration => new ClassPart(file.RelativePath, declaration)))
            .GroupBy(part => QualifiedName(part.Declaration))
            .Select(group => group.ToArray())
            .Where(parts => parts.SelectMany(FindLoggerMessageMethods).Any());

        var violations = new List<string>();
        var classesByBase = new Dictionary<int, List<string>>();
        foreach (var parts in logClasses)
        {
            var className = parts[0].Declaration.Identifier.Text;
            var classLocation = $"{LineOf(parts[0].RelativePath, parts[0].Declaration)}: {className}";
            var eventIdBase = parts.Select(part => FindEventIdBase(part.Declaration)).FirstOrDefault(value => value is not null);
            if (eventIdBase is not { } rangeBase)
            {
                violations.Add($"{classLocation}: declares no const int {EventIdBaseName} with a literal value");
                continue;
            }

            if (rangeBase <= 0 || rangeBase % EventIdRangeSize != 0)
            {
                violations.Add($"{classLocation}: {EventIdBaseName} {rangeBase} is not a positive multiple of {EventIdRangeSize}");
            }
            else
            {
                if (!classesByBase.TryGetValue(rangeBase, out var classes))
                    classesByBase[rangeBase] = classes = [];
                classes.Add(classLocation);
            }

            var methodsByOffset = new Dictionary<int, string>();
            foreach (var (relativePath, method, attribute) in parts.SelectMany(FindLoggerMessageMethods))
            {
                var problem = FindEventIdProblem(attribute, method.Identifier.Text, methodsByOffset);
                if (problem is not null)
                    violations.Add($"{LineOf(relativePath, attribute)}: {className}.{method.Identifier.Text}: {problem}");
            }
        }

        violations.AddRange(classesByBase
            .Where(entry => entry.Value.Count > 1)
            .SelectMany(entry => entry.Value.Select(classLocation => $"{classLocation}: {EventIdBaseName} {entry.Key} is declared by {entry.Value.Count} classes")));

        AssertUtil.AssertOnViolations(expectedViolations ?? [], violations);
    }

    private static bool IsLogCall(InvocationExpressionSyntax invocation) =>
        invocation.Expression is MemberAccessExpressionSyntax memberAccess
        && LogMethodNames.Contains(memberAccess.Name.Identifier.Text);

    private static IEnumerable<ArgumentSyntax> FindFlattenedExceptionArguments(InvocationExpressionSyntax invocation)
    {
        var caughtExceptions = invocation.Ancestors()
            .OfType<CatchClauseSyntax>()
            .Select(catchClause => catchClause.Declaration?.Identifier.Text)
            .OfType<string>()
            .Where(name => name.Length > 0)
            .ToHashSet(StringComparer.Ordinal);
        if (caughtExceptions.Count == 0)
            return [];

        var arguments = invocation.ArgumentList.Arguments;
        var templateIndex = arguments.IndexOf(argument => IsStringTemplate(argument.Expression));

        return arguments.Where((argument, index) =>
            IsFlattened(argument.Expression, caughtExceptions)
            || (templateIndex >= 0 && index > templateIndex && IsCaughtException(argument.Expression, caughtExceptions)));
    }

    private static bool IsStringTemplate(ExpressionSyntax expression) =>
        expression is InterpolatedStringExpressionSyntax
        || expression.IsKind(SyntaxKind.StringLiteralExpression);

    private static bool IsFlattened(ExpressionSyntax expression, HashSet<string> caughtExceptions) =>
        expression.DescendantNodesAndSelf().Any(node => node switch
        {
            MemberAccessExpressionSyntax memberAccess =>
                IsCaughtException(memberAccess.Expression, caughtExceptions)
                && FlatteningMembers.Contains(memberAccess.Name.Identifier.Text),
            InterpolationSyntax interpolation => IsCaughtException(interpolation.Expression, caughtExceptions),
            _ => false,
        });

    private static bool IsCaughtException(ExpressionSyntax expression, HashSet<string> caughtExceptions) =>
        expression is IdentifierNameSyntax identifier && caughtExceptions.Contains(identifier.Identifier.Text);

    private static string QualifiedName(ClassDeclarationSyntax declaration) =>
        string.Join(".", declaration.AncestorsAndSelf()
            .Select(node => node switch
            {
                BaseNamespaceDeclarationSyntax namespaceDeclaration => namespaceDeclaration.Name.ToString(),
                TypeDeclarationSyntax typeDeclaration => typeDeclaration.Identifier.Text,
                _ => null,
            })
            .OfType<string>()
            .Reverse());

    private static IEnumerable<(string RelativePath, MethodDeclarationSyntax Method, AttributeSyntax Attribute)> FindLoggerMessageMethods(ClassPart part) =>
        part.Declaration.Members
            .OfType<MethodDeclarationSyntax>()
            .SelectMany(method => method.AttributeLists
                .SelectMany(list => list.Attributes)
                .Where(IsLoggerMessageAttribute)
                .Select(attribute => (part.RelativePath, method, attribute)));

    private static bool IsLoggerMessageAttribute(AttributeSyntax attribute) =>
        attribute.Name switch
        {
            QualifiedNameSyntax qualified => qualified.Right.Identifier.Text,
            AliasQualifiedNameSyntax aliasQualified => aliasQualified.Name.Identifier.Text,
            SimpleNameSyntax simple => simple.Identifier.Text,
            _ => attribute.Name.ToString(),
        } is "LoggerMessage" or "LoggerMessageAttribute";

    private static int? FindEventIdBase(ClassDeclarationSyntax declaration) =>
        declaration.Members
            .OfType<FieldDeclarationSyntax>()
            .Where(field => field.Modifiers.Any(SyntaxKind.ConstKeyword))
            .SelectMany(field => field.Declaration.Variables)
            .Where(variable => variable.Identifier.Text == EventIdBaseName)
            .Select(variable => variable.Initializer?.Value is LiteralExpressionSyntax { Token.Value: int value } ? value : (int?)null)
            .FirstOrDefault();

    private static string? FindEventIdProblem(AttributeSyntax attribute, string methodName, Dictionary<int, string> methodsByOffset)
    {
        var eventId = attribute.ArgumentList?.Arguments
            .FirstOrDefault(argument => argument.NameEquals?.Name.Identifier.Text == "EventId")
            ?.Expression;
        if (eventId is null)
            return "EventId is not set";

        if (eventId is not BinaryExpressionSyntax
            {
                Left: IdentifierNameSyntax { Identifier.Text: EventIdBaseName },
                Right: LiteralExpressionSyntax { Token.Value: int offset },
            } binary
            || !binary.IsKind(SyntaxKind.AddExpression))
            return $"EventId {eventId} is not {EventIdBaseName} + n";

        if (offset < 1 || offset >= EventIdRangeSize)
            return $"EventId offset {offset} is outside 1..{EventIdRangeSize - 1}";

        if (methodsByOffset.TryGetValue(offset, out var existingMethod))
            return $"EventId offset {offset} is already used by {existingMethod}";

        methodsByOffset[offset] = methodName;
        return null;
    }

    private static string LineOf(string relativePath, SyntaxNode node) =>
        $"{relativePath}:{node.GetLocation().GetLineSpan().StartLinePosition.Line + 1}";

    private struct ClassPart
    {
        public ClassPart(string relativePath, ClassDeclarationSyntax declaration)
        {
            RelativePath = relativePath;
            Declaration = declaration;
        }

        public string RelativePath { get; }
        public ClassDeclarationSyntax Declaration { get; }
    }
}
