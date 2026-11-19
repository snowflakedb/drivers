using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;
using Snowflake.Data.Tests.Assertions;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class AssembliesMetadataTest
{
    [SnowflakeFact]
    public void TestDriverAssemblies_MustMatchAllProjectDirectories()
    {
        var registeredAssemblies = AssembliesMetadata.DriverAssemblies
            .Select(x => x.ProjectName)
            .OrderBy(x => x)
            .ToArray();

        var actualAssemblies = DiscoverAllAssemblyDirectories()
            .OrderBy(x => x)
            .ToArray();

        actualAssemblies.ShouldBeEquivalent(registeredAssemblies);
    }

    [SnowflakeFact]
    public void TestProdAssemblies_MustMatchAllNonTestProjectDirectories()
    {
        var registeredAssemblies = AssembliesMetadata.DriverProdAssemblies
            .Select(x => x.ProjectName)
            .OrderBy(x => x)
            .ToArray();

        var actualProdAssemblies = DiscoverProdAssemblyDirectories()
            .Select(x => x.Directory)
            .OrderBy(x => x)
            .ToArray();

        actualProdAssemblies.ShouldBeEquivalent(registeredAssemblies);
    }

    [SnowflakeFact]
    public void NoMultipleProjectInSingleDirectory()
    {
        var directoriesWithMultipleProjects = DiscoverProdAssemblyDirectories()
            .Concat(DiscoverTestAssemblyDirectories())
            .GroupBy(x => x.Directory)
            .Where(x => x.Count() > 1)
            .OrderBy(x => x.Key)
            .ToArray();

        directoriesWithMultipleProjects.ShouldBeEmpty(x => $"Directory {x.Key} has multiple projects defined!");
    }

    [SnowflakeTheory]
    [ClassData(typeof(DriverAssemblies))]
    public void OnlyAllowedProjectDependencies(string assemblyMetadataName)
    {
        var dependentAssemblies = AssemblyUtil.LoadAssembly(assemblyMetadataName)
            .GetReferencedAssemblies()
            .Select(x => x.Name)
            .Where(x => AssembliesMetadata.AllAssemblies.Any(y => y.Name == x));

        var allowed = ((AssemblyMetadata)assemblyMetadataName).AllowedProjectDependencies.Select(x => x.Name);
        var disallowedDependencies = dependentAssemblies.Except(allowed);

        disallowedDependencies.ShouldBeEmpty(x => $"Dependency on {x} id not allowed.");
    }

    [SnowflakeFact]
    public void NoOtherAssembliesThanTestAndProd()
    {
        var testAndProd = AssembliesMetadata.DriverTestAssemblies.Concat(AssembliesMetadata.DriverProdAssemblies);
        testAndProd.ShouldBeEquivalent(AssembliesMetadata.DriverAssemblies);
    }

    private static IEnumerable<string> DiscoverAllAssemblyDirectories() =>
        DiscoverProdAssemblyDirectories().Concat(DiscoverTestAssemblyDirectories()).Select(x => x.Directory);

    private static IEnumerable<(string Directory, string Project)> DiscoverProdAssemblyDirectories() =>
        GetProjectsWithDirectories(Path.Combine(AssemblyUtil.SolutionRoot, "src"));

    private static IEnumerable<(string Directory, string Project)> DiscoverTestAssemblyDirectories() =>
        GetProjectsWithDirectories(Path.Combine(AssemblyUtil.SolutionRoot, "tests"));

    private static IEnumerable<(string Directory, string ProjectName)> GetProjectsWithDirectories(string parentDir)
    {
        if (!Directory.Exists(parentDir))
            yield break;

        foreach (var dir in Directory.GetDirectories(parentDir))
        {
            var dirName = Path.GetFileName(dir);
            var csprojPath = Path.Combine(dir, $"{dirName}.csproj");
            if (File.Exists(csprojPath))
                yield return (dirName, csprojPath);
        }
    }
}
