#if NETFRAMEWORK
namespace Snowflake.Data.Tests.Interop;

/// <summary>
/// Polyfill for <c>Task.WaitAsync(TimeSpan, CancellationToken)</c> which is only
/// available on .NET 6+.
/// </summary>
internal static class TaskExtensions
{
    public static async Task<T> WaitAsync<T>(this Task<T> task, TimeSpan timeout, CancellationToken cancellationToken)
    {
        var delayTask = Task.Delay(timeout, cancellationToken);
        var completedTask = await Task.WhenAny(task, delayTask).ConfigureAwait(false);

        if (completedTask == delayTask)
        {
            cancellationToken.ThrowIfCancellationRequested();
            throw new TimeoutException($"The operation timed out after {timeout.TotalSeconds:F1}s.");
        }

        return await task.ConfigureAwait(false);
    }

    public static async Task WaitAsync(this Task task, TimeSpan timeout, CancellationToken cancellationToken)
    {
        var delayTask = Task.Delay(timeout, cancellationToken);
        var completedTask = await Task.WhenAny(task, delayTask).ConfigureAwait(false);

        if (completedTask == delayTask)
        {
            cancellationToken.ThrowIfCancellationRequested();
            throw new TimeoutException($"The operation timed out after {timeout.TotalSeconds:F1}s.");
        }

        await task.ConfigureAwait(false);
    }
}
#endif
