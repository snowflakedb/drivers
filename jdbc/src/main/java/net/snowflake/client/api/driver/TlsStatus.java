package net.snowflake.client.api.driver;

/**
 * TLS-only status of this driver's linked Rustls crypto provider and build configuration. Neither
 * value attests to compliance of other cryptography or of the complete driver artifact.
 */
public final class TlsStatus {
  private final boolean tlsProviderIsFips;
  private final boolean fipsTlsBuildEnabled;

  TlsStatus(boolean tlsProviderIsFips, boolean fipsTlsBuildEnabled) {
    this.tlsProviderIsFips = tlsProviderIsFips;
    this.fipsTlsBuildEnabled = fipsTlsBuildEnabled;
  }

  /** Whether Rustls reports the linked TLS crypto provider as FIPS. */
  public boolean isTlsProviderFips() {
    return tlsProviderIsFips;
  }

  /** Whether sf_core was built with its fips-tls feature enabled. */
  public boolean isFipsTlsBuildEnabled() {
    return fipsTlsBuildEnabled;
  }
}
