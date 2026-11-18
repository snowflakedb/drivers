using Snowflake.Data.Tests.Compatibility;
using Snowflake.Data.Tests.Reference.Fixtures;

namespace Snowflake.Data.Tests.Reference;

public sealed class CertificateRevocationTest : ReferenceTestBase
{
    public CertificateRevocationTest(ReferenceITFixture fixture, ITestOutputHelper output)
        : base(fixture, output)
    {
    }

    [SnowflakeFact]
    public async Task ShouldOpenConnectionWithCertRevocationCheckEnabledAsync()
    {
        Skip.When(true, "Temporarily ignored");
        Skip.For(SkipCondition.SkipOnCI, "Test is skipped on CI.");

        var crlSetting = BehaviorDifference.OldDriverOnly("BD#cert-revocation-check-mode-key-not-recognized")
            ? "CERTREVOCATIONCHECKMODE=ENABLED"
            : "crl_check_mode=ENABLED";
        using var probe = Fixture.Factory.Create(Output);
        var connectionString = $"{probe.ConnectionString};{crlSetting}";
        using var connection = Fixture.Factory.Create(Output, connectionString);

        await connection.OpenAsync(TestContext.Current.CancellationToken).ConfigureAwait(false);

        Assert.Equal(ConnectionState.Open, connection.State);
    }
}
