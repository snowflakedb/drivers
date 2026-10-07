package net.snowflake.client.api.driver;

/**
 * TLS-only status of this driver's linked Rustls crypto provider and build configuration. Neither
 * value attests to compliance of other cryptography or of the complete driver artifact.
 */
public final class TlsStatus {
  private final boolean tlsProviderIsFips;
  private final boolean fipsBuildEnabled;

  TlsStatus(boolean tlsProviderIsFips, boolean fipsBuildEnabled) {
    this.tlsProviderIsFips = tlsProviderIsFips;
    this.fipsBuildEnabled = fipsBuildEnabled;
  }

  /** Whether Rustls reports the linked TLS crypto provider as FIPS. */
  public boolean isTlsProviderFips() {
    return tlsProviderIsFips;
  }

  /** Whether sf_core was built with its fips feature enabled. */
  public boolean isFipsBuildEnabled() {
    return fipsBuildEnabled;
  }
}
