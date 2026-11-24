namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
[Trait("Category", "TestingInfrastructure")]
public sealed class SnowflakeTheoryDiscovererTest
{
    public static IEnumerable<object[]> RowsXunitCannotSerialize()
    {
        yield return new object[] { new UnserializableValue(), 1 };
        yield return new object[] { new UnserializableValue(), 2 };
    }

    [SnowflakeTheory]
    [MemberData(nameof(RowsXunitCannotSerialize))]
    public void PassesTheoryRowsThatXunitCannotSerialize(UnserializableValue value, int number)
    {
        Assert.NotNull(value);
        Assert.True(number > 0);
    }

    public sealed class UnserializableValue
    {
    }
}
