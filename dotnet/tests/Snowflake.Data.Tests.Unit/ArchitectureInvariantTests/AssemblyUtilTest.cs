using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class AssemblyUtilTest
{
    [SnowflakeTheory]
    [InlineData("Snowflake.Data.Tests.E2E.Shared")]
    [InlineData("Snowflake.Data.Tests.E2E")]
    [InlineData("Snowflake.Data.Tests.Wiremock")]
    public void LoadAssembly_LoadsSiblingTestAssemblyFromBinOutputNotObj(string assemblyName)
    {
        var binSegment = $"{Path.DirectorySeparatorChar}bin{Path.DirectorySeparatorChar}";
        var objSegment = $"{Path.DirectorySeparatorChar}obj{Path.DirectorySeparatorChar}";

        var location = AssemblyUtil.LoadAssembly(assemblyName).Location;

        Assert.Contains(binSegment, location, StringComparison.Ordinal);
        Assert.DoesNotContain(objSegment, location, StringComparison.Ordinal);
    }
}
