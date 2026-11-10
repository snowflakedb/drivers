using Snowflake.Data.Proto.Errors;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;
using Snowflake.Data.Tests.Compatibility;
using AssemblyMetadata = Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata.AssemblyMetadata;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class UniversalCoreExceptionsTest
{
    [SnowflakeFact]
    public void TestCoreFailures_EveryExceptionDefinedInProtoIsACoreFailure()
    {
        var exceptionTypes = ConcreteExceptionTypes(AssembliesMetadata.ProtoAssembly);

        var unrecognized = exceptionTypes
            .Where(type => !UniversalCoreExceptions.IsUniversalCoreException((Exception)UninitializedObject.Create(type)))
            .Select(type => type.FullName ?? type.Name);

        Assert.NotEmpty(exceptionTypes);
        AssertUtil.AssertOnViolations([], unrecognized);
    }

    [SnowflakeFact]
    public void TestCoreFailures_NoExceptionDefinedInTheDriverIsACoreFailure()
    {
        var exceptionTypes = ConcreteExceptionTypes(AssembliesMetadata.RootAssembly);

        var misclassified = exceptionTypes
            .Where(type => UniversalCoreExceptions.IsUniversalCoreException((Exception)UninitializedObject.Create(type)))
            .Select(type => type.FullName ?? type.Name);

        Assert.NotEmpty(exceptionTypes);
        AssertUtil.AssertOnViolations([], misclassified);
    }

    private static IReadOnlyList<Type> ConcreteExceptionTypes(AssemblyMetadata assemblyMetadata) =>
        AssemblyUtil.LoadAssembly(assemblyMetadata).GetTypes()
            .Where(type => typeof(Exception).IsAssignableFrom(type) &&
                           type is { IsAbstract: false, IsGenericTypeDefinition: false })
            .ToArray();
}
