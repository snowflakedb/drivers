namespace Snowflake.Data.Tests.E2E.Fixtures;

public sealed class E2EITFixture : ITFixture
{
    public override ITestConnectionFactory Factory { get; } = new Snowflake.Data.Tests.E2E.TestConnectionFactory();
}
