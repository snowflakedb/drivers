using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Config;
using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

[Trait("Driver", "Reference")]
public class TlsVersionTest : Snowflake.Data.Tests.TlsVersionTest, IReferenceTest
{
    private const string SkipReason =
        "Old driver rejects untrusted root certificates unconditionally — no connection-string bypass for WireMock's self-signed CA";

    public TlsVersionTest(ReferenceITFixture fixture, ITestOutputHelper output) : base(fixture, output)
    {
        Fixture = fixture;
    }

    public new ReferenceITFixture Fixture { get; }
    public new ITestOutputHelper Output => base.Output;

    public override Task ShouldNegotiateTlsWhenTheServerOffersAVersionInsideTheWindowAsync()
    {
        Skip.When(GlobalState.UseOldDriver, SkipReason);
        return base.ShouldNegotiateTlsWhenTheServerOffersAVersionInsideTheWindowAsync();
    }

    public override Task ShouldFailTheHandshakeWhenTheServerOnlyOffersAVersionBelowTheMinimumAsync()
    {
        Skip.When(GlobalState.UseOldDriver, SkipReason);
        return base.ShouldFailTheHandshakeWhenTheServerOnlyOffersAVersionBelowTheMinimumAsync();
    }

    protected override string ExpectedTlsRejectErrorMessage => GlobalState.UseOldDriver
        ? "Error: Connection string is invalid: Parameter MINTLS value cannot be higher than MAXTLS value."
        : base.ExpectedTlsRejectErrorMessage;
}
