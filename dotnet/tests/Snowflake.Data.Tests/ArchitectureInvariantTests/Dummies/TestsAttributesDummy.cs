namespace Snowflake.Data.Tests.ArchitectureInvariantTests.Dummies;

// do not move/remove/rename/modify this.
public sealed class TestsAttributesDummy
{
    [Fact]
    public void DummyTest() => Assert.True(true);

    [Theory]
    [InlineData(true)]
    [InlineData(false)]
    public void DummyParametrizedTest(bool _) => Assert.True(true);

    [SnowflakeFact]
    public void DummyTest2() => Assert.True(true);
}

public sealed class TestsAttributesDummy2
{
    [SnowflakeTheory]
    [InlineData(true)]
    public void DummyParametrizedTest2(bool _) => Assert.True(true);
}
