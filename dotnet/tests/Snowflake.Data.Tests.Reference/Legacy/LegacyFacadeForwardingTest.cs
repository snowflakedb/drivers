using System.Runtime.CompilerServices;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;

namespace Snowflake.Data.Tests.Reference.Legacy;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
[Trait("Category", "TestingInfrastructure")]
[Trait("Driver", "Reference")]
public sealed class LegacyFacadeForwardingTest
{
    private const string Inner = "_inner";

    [SnowflakeFact]
    public void MethodsForwardToTheSameNamedInnerMethodWithTheSameArguments()
    {
        var violations = Members<MethodDeclarationSyntax>()
            .Where(method => Body(method.ExpressionBody, method.Body) is not InvocationExpressionSyntax call
                             || !IsInnerMember(call.Expression, method.Identifier.Text)
                             || !PassesParameters(call.ArgumentList, method.ParameterList))
            .Select(Describe);

        AssertNone(violations);
    }

    [SnowflakeFact]
    public void ConstructorsCreateTheInnerInstanceWithTheSameArguments()
    {
        var violations = Members<ConstructorDeclarationSyntax>()
            .Where(ctor => !ForwardsConstructor(ctor))
            .Select(Describe);

        AssertNone(violations);
    }

    [SnowflakeFact]
    public void PropertiesForwardGetAndSetToTheSameNamedInnerProperty()
    {
        var violations = Members<PropertyDeclarationSyntax>()
            .Where(property => !ForwardsProperty(property))
            .Select(Describe);

        AssertNone(violations);
    }

    [SnowflakeFact]
    public void AMethodThatForwardsADifferentNameIsReported()
    {
        var violations = Members<MethodDeclarationSyntax>(Parse(
            """
            class LegacyBad
            {
                private readonly object _inner;
                public void Open() => _inner.Close();
            }
            """)).Select(Describe).ToList();

        Assert.Contains("LegacyBad", string.Join("\n", violations));
    }

    [SnowflakeFact]
    public void AMethodThatCallsTwoInnerMethodsIsReported()
    {
        var violations = Members<MethodDeclarationSyntax>(Parse(
            """
            class LegacyBad
            {
                private readonly object _inner;
                public void Open()
                {
                    _inner.Open();
                    _inner.Close();
                }
            }
            """)).Select(Describe).ToList();

        Assert.Contains("LegacyBad", string.Join("\n", violations));
    }

    private static bool ForwardsConstructor(ConstructorDeclarationSyntax ctor)
    {
        if (Body(ctor.ExpressionBody, ctor.Body) is not AssignmentExpressionSyntax
            {
                Left: IdentifierNameSyntax { Identifier.Text: Inner },
                Right: BaseObjectCreationExpressionSyntax creation,
            })
        {
            return false;
        }

        var arguments = creation.ArgumentList?.Arguments.Select(argument => WithoutCast(argument.Expression)) ?? Enumerable.Empty<string>();
        var initializer = creation.Initializer?.Expressions.OfType<AssignmentExpressionSyntax>()
            .Select(assignment => WithoutCast(assignment.Right)) ?? Enumerable.Empty<string>();
        var passed = arguments.Concat(initializer);
        return passed.SequenceEqual(ctor.ParameterList.Parameters.Select(parameter => parameter.Identifier.Text));
    }

    private static bool ForwardsProperty(PropertyDeclarationSyntax property)
    {
        var name = property.Identifier.Text;
        if (property.ExpressionBody is not null)
            return IsInnerMember(property.ExpressionBody.Expression, name);

        return property.AccessorList!.Accessors.All(accessor => (accessor.Keyword.Kind(), Body(accessor.ExpressionBody, accessor.Body)) switch
        {
            (SyntaxKind.GetKeyword, var get) => IsInnerMember(get, name),
            (SyntaxKind.SetKeyword, AssignmentExpressionSyntax set) => IsInnerMember(set.Left, name) && IsForwardedValue(set.Right),
            _ => false,
        });
    }

    private static bool IsForwardedValue(ExpressionSyntax expression) =>
        expression is IdentifierNameSyntax { Identifier.Text: "value" }
        || (expression is BinaryExpressionSyntax { RawKind: (int)SyntaxKind.CoalesceExpression } coalesce
            && IsForwardedValue(coalesce.Left));

    private static IEnumerable<T> Members<T>(IEnumerable<SyntaxTree>? trees = null, [CallerFilePath] string thisFile = "")
        where T : MemberDeclarationSyntax
    {
        var facades = (trees ?? FacadeTrees(thisFile))
            .SelectMany(tree => tree.GetRoot().DescendantNodes().OfType<ClassDeclarationSyntax>())
            .Where(type => type.Members.OfType<FieldDeclarationSyntax>().Any(field => field.Declaration.Variables.Any(variable => variable.Identifier.Text == Inner)))
            .ToList();

        Assert.NotEmpty(facades);
        return facades.SelectMany(facade => facade.Members.OfType<T>());
    }

    private static IEnumerable<SyntaxTree> FacadeTrees(string thisFile) =>
        Directory.GetFiles(Path.GetDirectoryName(thisFile)!, "*.cs")
            .Where(file => !file.EndsWith("Test.cs", StringComparison.Ordinal))
            .Select(file => CSharpSyntaxTree.ParseText(File.ReadAllText(file)));

    private static IEnumerable<SyntaxTree> Parse(string source) =>
        new[] { CSharpSyntaxTree.ParseText(source) };

    private static ExpressionSyntax? Body(ArrowExpressionClauseSyntax? arrow, BlockSyntax? block)
    {
        if (arrow is not null)
            return arrow.Expression;
        if (block is null || block.Statements.Count != 1)
            return null;
        return block.Statements[0] switch
        {
            ReturnStatementSyntax ret => ret.Expression,
            ExpressionStatementSyntax expression => expression.Expression,
            _ => null,
        };
    }

    private static bool IsInnerMember(ExpressionSyntax? expression, string name) =>
        expression is MemberAccessExpressionSyntax { Expression: IdentifierNameSyntax { Identifier.Text: Inner } } access
        && access.Name.Identifier.Text == name;

    private static bool PassesParameters(ArgumentListSyntax? arguments, ParameterListSyntax parameters) =>
        (arguments?.Arguments.Select(argument => WithoutCast(argument.Expression)) ?? Enumerable.Empty<string>())
            .SequenceEqual(parameters.Parameters.Select(parameter => parameter.Identifier.Text));

    private static string WithoutCast(ExpressionSyntax expression) =>
        expression is CastExpressionSyntax cast ? WithoutCast(cast.Expression) : expression.ToString();

    private static string Describe(MemberDeclarationSyntax member) =>
        $"{member.FirstAncestorOrSelf<ClassDeclarationSyntax>()!.Identifier.Text}:{member.GetLocation().GetLineSpan().StartLinePosition.Line + 1}: {member.WithAttributeLists([]).ToString().Trim()}";

    private static void AssertNone(IEnumerable<string> violations)
    {
        var list = violations.ToList();
        Assert.True(list.Count == 0, "Facade members that do not forward to _inner:\n" + string.Join("\n", list));
    }
}
