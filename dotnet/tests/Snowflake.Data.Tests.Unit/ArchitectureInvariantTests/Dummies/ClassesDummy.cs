namespace Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;

// do not move/remove/rename/modify this.
public class UnsealedNoProtected
{
    public int Value { get; set; }
}

public sealed class SealedClass
{
    public int Value { get; set; }
}

public class HasProtectedMember
{
    protected int ComputeValue() => 42;
}

public abstract class AbstractClass
{
    public abstract void DoWork();
}

public static class StaticClass
{
    public static int Value => 42;
}

public static unsafe class UnsafeClass
{
    public static int Value => 42;
}

public static class UnsafeMethodClass
{
    public static unsafe int Value => 42;

    public static int Safe => 21;
}
