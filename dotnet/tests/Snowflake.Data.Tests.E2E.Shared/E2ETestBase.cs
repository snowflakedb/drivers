using Snowflake.Data.Tests.E2E.Fixtures;

namespace Snowflake.Data.Tests.E2E;

#pragma warning disable xUnit1056
public interface IE2ETest : IClassFixture<E2EITFixture>
#pragma warning restore xUnit1056
{
    public E2EITFixture Fixture { get; }

    public ITestOutputHelper Output { get; }
}

public abstract class E2ETestBase : IE2ETest
{
    public E2ETestBase(E2EITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    public E2EITFixture Fixture { get; }

    public ITestOutputHelper Output { get; }
}

[Trait("Category", "E2E")]
public abstract class LiveE2ETestBase : E2ETestBase
{
    public LiveE2ETestBase(E2EITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }
}
