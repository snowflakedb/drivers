namespace Snowflake.Data.Tests.Fixtures;

public static class ITEnvironment
{
    private static readonly CancellationTokenSource Cts = new();
    private static volatile int _initState;

    static ITEnvironment()
    {
        Cts.CancelAfter(TimeSpan.FromHours(1));
        Cts.Token.Register(TerminateTestRun);
    }

    public static void Init()
    {
        if (Interlocked.Exchange(ref _initState, 1) == 0)
            Console.WriteLine("Test environment initialized.");
    }

    private static void TerminateTestRun()
    {
        Console.WriteLine("Terminating test run, as it's unlikely it can recover.");
        Environment.Exit(-1);
    }
}
