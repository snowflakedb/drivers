using Xunit.Sdk;

namespace Snowflake.Data.Tests.Compatibility;

// TODO will be used in the future
public static class Skip
{
    public static void When(bool condition, string rationale)
    {
        if (condition)
            throw SkipException.ForSkip(rationale);
    }
}
