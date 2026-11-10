using Snowflake.Data.Tests.Compatibility;
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

    protected override string ExpectedTlsRejectErrorMessage =>
        BehaviorDifference.OldDriverOnly("BD#tls-min-above-max-message")
            ? "Error: Connection string is invalid: Parameter MINTLS value cannot be higher than MAXTLS value."
            : base.ExpectedTlsRejectErrorMessage;

    public override Task ShouldNegotiateTlsWhenTheServerOffersAVersionInsideTheWindowAsync()
    {
        Skip.OnOldDriver("BD#custom-root-store-path", SkipReason);
        return base.ShouldNegotiateTlsWhenTheServerOffersAVersionInsideTheWindowAsync();
    }

    public override Task ShouldFailTheHandshakeWhenTheServerOnlyOffersAVersionBelowTheMinimumAsync()
    {
        Skip.OnOldDriver("BD#custom-root-store-path", SkipReason);
        return base.ShouldFailTheHandshakeWhenTheServerOnlyOffersAVersionBelowTheMinimumAsync();
    }
}
