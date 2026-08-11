namespace Snowflake.Data.Tests.Reference.Fixtures;

public sealed class RegressionITFixture : ITFixture
{
    public override ITestConnectionFactory Factory { get; } = new TestConnectionFactory();
}
