namespace Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;

// do not move/remove/rename/modify this.
public static class AsyncInvariantsDummy
{
    public static async Task DoSomething()
    {
        await Task.Delay(1);
        await Task.Yield();
        await Task.Run(() => Task.Delay(2));
        await Task.Run(DoSomething2);
        await Task.Run(() => Task.Delay(3).ConfigureAwait(false));
        await Task.Run(async () => Task.Delay(4).ConfigureAwait(false));
        await Task.Run(async () => await Task.Delay(5).ConfigureAwait(false));
        await Task.Run(() => DoSomething2().ConfigureAwait(true));
        await Task.Run(async () => await DoSomething2().ConfigureAwait(false));
        await Task.Run(async () => await DoSomething2()).ConfigureAwait(false);
        await Task.Run(async () => await Task.Delay(6));
    }

    public static async Task DoSomething2()
    {
        await Task.Delay(10).ConfigureAwait(false);
        await Task.Yield();
        await Task.Delay(11);
        await DoSomething().ConfigureAwait(false);
    }
}
