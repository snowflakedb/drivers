namespace Snowflake.Data.Tests.Compatibility;

public static class BehaviorDifference
{
    public static bool NewDriverOnly(string differenceId)
    {
        _ = differenceId;
        return TestDriver.Current == TestDriver.Kind.New;
    }

    public static bool OldDriverOnly(string differenceId)
    {
        _ = differenceId;
        return TestDriver.Current == TestDriver.Kind.Old;
    }
}
