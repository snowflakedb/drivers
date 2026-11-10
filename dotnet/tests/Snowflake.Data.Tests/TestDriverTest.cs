using Snowflake.Data.Tests.Compatibility;
using Xunit.Sdk;

namespace Snowflake.Data.Tests;

[Trait("Category", "Unit")]
[Collection(TestDriverCollection.Name)]
public sealed class TestDriverTest : IDisposable
{
    private readonly ITestConnectionFactory? _previousFactory;

    public TestDriverTest()
    {
        _previousFactory = ITFixture.OverriddenFactory;
    }

    public void Dispose()
    {
        ITFixture.OverriddenFactory = _previousFactory;
    }

    [SnowflakeFact]
    public void Current_DefaultsToNewDriver()
    {
        ITFixture.OverriddenFactory = null;
        Assert.Equal(TestDriver.Kind.New, TestDriver.Current);
        Assert.Equal(TestDriver.Kind.New, new DefaultTestConnectionFactory().Driver);
    }

    [SnowflakeFact]
    public void Current_MatchesTheFactoryDriver()
    {
        ITFixture.OverriddenFactory = new StubFactory(TestDriver.Kind.Old);
        Assert.Equal(TestDriver.Kind.Old, TestDriver.Current);
        Assert.Equal(TestDriver.Kind.Old, ITFixture.OverriddenFactory.Driver);
    }

    [SnowflakeFact]
    public void OnOldDriver_DoesNotSkipWhenTheFactoryReportsNew()
    {
        ITFixture.OverriddenFactory = new StubFactory(TestDriver.Kind.New);
        Skip.OnOldDriver("testdriver-unit", "old driver skip");
    }

    [SnowflakeFact]
    public void OnNewDriver_DoesNotSkipWhenTheFactoryReportsOld()
    {
        ITFixture.OverriddenFactory = new StubFactory(TestDriver.Kind.Old);
        Skip.OnNewDriver("testdriver-unit", "new driver skip");
    }

    [SnowflakeFact]
    public void OnOldDriver_SkipsWithTheDifferenceIdWhenTheFactoryReportsOld()
    {
        ITFixture.OverriddenFactory = new StubFactory(TestDriver.Kind.Old);
        string? skipMessage = null;
        try
        {
            Skip.OnOldDriver("testdriver-unit", "old driver skip");
        }
        catch (SkipException ex)
        {
            skipMessage = ex.Message;
        }

        Assert.EndsWith("testdriver-unit: old driver skip", skipMessage);
    }

    [SnowflakeFact]
    public void BehaviorDifference_FollowsTheFactoryDriver()
    {
        ITFixture.OverriddenFactory = new StubFactory(TestDriver.Kind.New);
        Assert.True(BehaviorDifference.NewDriverOnly("testdriver-unit"));
        Assert.False(BehaviorDifference.OldDriverOnly("testdriver-unit"));

        ITFixture.OverriddenFactory = new StubFactory(TestDriver.Kind.Old);
        Assert.False(BehaviorDifference.NewDriverOnly("testdriver-unit"));
        Assert.True(BehaviorDifference.OldDriverOnly("testdriver-unit"));
    }

    private sealed class StubFactory : ITestConnectionFactory
    {
        public StubFactory(TestDriver.Kind driver)
        {
            Driver = driver;
        }

        public TestDriver.Kind Driver { get; }

        public ISnowflakeDbConnection Create(
            ITestOutputHelper? testOutputHelper,
            string? connectionStringOverride = null) =>
            throw new NotSupportedException();
    }
}
