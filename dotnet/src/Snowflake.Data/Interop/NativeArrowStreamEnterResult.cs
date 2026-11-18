namespace Snowflake.Data.Interop;

/// <summary>The outcome of <see cref="NativeArrowStream.TryEnter"/>.</summary>
internal enum NativeArrowStreamEnterResult
{
    /// <summary>The caller holds the stream until it disposes the lease.</summary>
    Entered,

    /// <summary>Another use of the stream is in progress.</summary>
    Busy,

    /// <summary>The stream is disposed, or its producer has released it.</summary>
    Released,
}
