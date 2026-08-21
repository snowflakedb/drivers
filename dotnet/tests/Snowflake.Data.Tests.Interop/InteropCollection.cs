namespace Snowflake.Data.Tests.Interop;

[CollectionDefinition("Interop", DisableParallelization = true)]
public sealed class InteropCollection : ICollectionFixture<InteropCollection.Fixture>
{
    public sealed class Fixture;
}
