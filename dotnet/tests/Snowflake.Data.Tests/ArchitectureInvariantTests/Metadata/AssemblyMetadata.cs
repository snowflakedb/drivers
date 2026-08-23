namespace Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;

public sealed class AssemblyMetadata
{
    public string Name { get; }

    public string ProjectName { get; }

    public AssemblyMetadata[] AllowedProjectDependencies { get; }

    public AssemblyMetadata(string name, params AssemblyMetadata[] allowedProjectDependencies) : this(name, name,
        allowedProjectDependencies)
    {
    }

    public AssemblyMetadata(string name, string projectName, params AssemblyMetadata[] allowedProjectDependencies)
    {
        Name = name;
        ProjectName = projectName;
        AllowedProjectDependencies = allowedProjectDependencies;
    }

    public static implicit operator string(AssemblyMetadata assemblyMetadata) => assemblyMetadata.Name;

    public static implicit operator AssemblyMetadata(string assemblyName) => AssembliesMetadata.FromName(assemblyName);

    private bool Equals(AssemblyMetadata other) => string.Equals(Name, other.Name, StringComparison.OrdinalIgnoreCase);

    public override bool Equals(object? obj) => ReferenceEquals(this, obj) || obj is AssemblyMetadata other && Equals(other);

    public override int GetHashCode() => StringComparer.OrdinalIgnoreCase.GetHashCode(Name);
}


