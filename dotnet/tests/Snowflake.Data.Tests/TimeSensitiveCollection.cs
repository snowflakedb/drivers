namespace Snowflake.Data.Tests;

/// <summary>
/// Tests that measure elapsed time. They run on their own, after the parallel tests, so that
/// thread-pool load from other tests cannot stretch or reorder the delays they observe.
/// </summary>
[CollectionDefinition(Name, DisableParallelization = true)]
public sealed class TimeSensitiveCollection
{
    public const string Name = "TimeSensitive";
}
