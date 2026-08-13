namespace Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;

// do not move/remove/rename/modify this.
public static class AsyncMethodDummy
{
    // Violation: async method without "Async" suffix
    public static async Task DoWork()
    {
        await Task.Delay(1).ConfigureAwait(false);
    }

    // Violation: returns Task without "Async" suffix
    public static Task RunJob()
    {
        return Task.CompletedTask;
    }

    // Violation: async method without CancellationToken
    public static async Task ProcessAsync()
    {
        await Task.Delay(1).ConfigureAwait(false);
    }

    // Violation: passes default CancellationToken at call-site
    public static async Task CallWithDefaultAsync(CancellationToken cancellationToken)
    {
        await Task.Delay(1, default).ConfigureAwait(false);
    }

    // Violation: passes CancellationToken.None at call-site
    public static async Task CallWithNoneAsync(CancellationToken cancellationToken)
    {
        await Task.Delay(1, CancellationToken.None).ConfigureAwait(false);
    }

    // Violation: passes default(CancellationToken) at call-site
    public static async Task CallWithDefaultOfAsync(CancellationToken cancellationToken)
    {
        await Task.Delay(1, default(CancellationToken)).ConfigureAwait(false);
    }

    // Violation: uses .GetAwaiter().GetResult()
    public static void BlockingCall()
    {
        Task.Delay(1).GetAwaiter().GetResult();
    }

    // Violation: uses .Wait()
    public static void BlockingWait()
    {
        Task.Delay(1).Wait();
    }

    // Violation: uses .Result
    public static int BlockingResult()
    {
        return Task.FromResult(42).Result;
    }

    // Compliant: proper async method with CancellationToken
    public static async Task CompliantAsync(CancellationToken cancellationToken)
    {
        await Task.Delay(1, cancellationToken).ConfigureAwait(false);
    }
}
