namespace Snowflake.Data.Tests.Compatibility;

// TODO differenceId is not looked up anywhere yet: the .NET driver has no BehaviorDifferences.yaml and nothing validates ids against it.
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
