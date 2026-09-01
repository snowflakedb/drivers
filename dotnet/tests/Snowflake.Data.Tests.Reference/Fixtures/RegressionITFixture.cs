namespace Snowflake.Data.Tests.Reference.Fixtures;

public sealed class ReferenceITFixture : ITFixture
{
    public override ITestConnectionFactory Factory { get; } = new TestConnectionFactory();
}
