namespace Snowflake.Data.Tests;

/// <summary>
/// Tests that read native memory after freeing it. They run on their own, after the parallel tests, so that
/// native allocations from other tests cannot reuse and overwrite the freed memory before they read it.
/// </summary>
[CollectionDefinition(Name, DisableParallelization = true)]
public sealed class NativeMemorySensitiveCollection
{
    public const string Name = "NativeMemorySensitive";
}
