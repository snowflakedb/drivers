namespace Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;

// do not move/remove/rename/modify this.
public static class AsyncMethodDummy2
{
    public static async void FireAndForget() => await Task.Delay(1).ConfigureAwait(false);

    public static void SleepyMethod() => Thread.Sleep(100);

    public static object UseDynamic()
    {
        dynamic x = 42;
        return x;
    }
}
