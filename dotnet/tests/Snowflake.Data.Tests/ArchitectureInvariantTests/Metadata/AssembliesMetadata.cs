namespace Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;

public sealed class DriverAssemblies : TheoryData<string>, IEnumerable<object[]>
{
    public DriverAssemblies()
    {
        foreach (var assemblyMetadata in AssembliesMetadata.DriverAssemblies)
        {
            Add(assemblyMetadata);
        }
    }

    IEnumerator<object[]> IEnumerable<object[]>.GetEnumerator() => AssembliesMetadata.DriverAssemblies.Select(x => new object[] { (string)x }).GetEnumerator();
}

public sealed class DriverTestAssemblies : TheoryData<string>, IEnumerable<object[]>
{
    public DriverTestAssemblies()
    {
        foreach (var assemblyMetadata in AssembliesMetadata.DriverTestAssemblies)
        {
            Add(assemblyMetadata);
        }
    }

    IEnumerator<object[]> IEnumerable<object[]>.GetEnumerator() => AssembliesMetadata.DriverTestAssemblies.Select(x => new object[] { (string)x }).GetEnumerator();
}

public sealed class DriverProdAssemblies : TheoryData<string>, IEnumerable<object[]>
{
    public DriverProdAssemblies()
    {
        foreach (var assemblyMetadata in AssembliesMetadata.DriverProdAssemblies)
        {
            Add(assemblyMetadata);
        }
    }

    IEnumerator<object[]> IEnumerable<object[]>.GetEnumerator() => AssembliesMetadata.DriverProdAssemblies.Select(x => new object[] { (string)x }).GetEnumerator();
}

public static class AssembliesMetadata
{
    public static readonly AssemblyMetadata OldDriverAssembly = new("Snowflake.Data");
    public static readonly AssemblyMetadata ProtoAssembly = new("Snowflake.Data.Proto");
    public static readonly AssemblyMetadata RootAssembly = new("Snowflake.Data.UD", "Snowflake.Data", ProtoAssembly);
    public static readonly AssemblyMetadata TestsCommonsAssembly = new("Snowflake.Data.Tests.Common", RootAssembly, ProtoAssembly);
    public static readonly AssemblyMetadata RegressionTestsAssembly = new("Snowflake.Data.Tests.Reference", RootAssembly, ProtoAssembly, OldDriverAssembly, TestsCommonsAssembly);
    public static readonly AssemblyMetadata InteropTestsAssembly = new("Snowflake.Data.Tests.Interop", RootAssembly, ProtoAssembly, TestsCommonsAssembly);
    public static readonly AssemblyMetadata TestsAssembly = new("Snowflake.Data.Tests", RootAssembly, ProtoAssembly, TestsCommonsAssembly);

    public static AssemblyMetadata[] AllAssemblies => DriverAssemblies.Concat([OldDriverAssembly]).ToArray();

    public static AssemblyMetadata[] DriverAssemblies =>
    [
        RootAssembly,
        ProtoAssembly,
        TestsAssembly,
        TestsCommonsAssembly,
        InteropTestsAssembly,
        RegressionTestsAssembly
    ];

    public static AssemblyMetadata[] DriverProdAssemblies =>
    [
        RootAssembly,
        ProtoAssembly,
    ];

    public static AssemblyMetadata[] DriverTestAssemblies =>
    [
        TestsAssembly,
        TestsCommonsAssembly,
        RegressionTestsAssembly,
        InteropTestsAssembly,
    ];

    public static AssemblyMetadata FromName(string name) => AllAssemblies.Single(x => string.Equals(x.Name, name, StringComparison.OrdinalIgnoreCase));
}
