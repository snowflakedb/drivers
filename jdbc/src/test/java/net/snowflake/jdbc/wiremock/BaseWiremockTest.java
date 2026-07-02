package net.snowflake.jdbc.wiremock;

import java.net.URI;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.TestInstance;

/**
 * Base class for JDBC wiremock tests. Manages one WireMock subprocess per test class and exposes
 * the JDBC URL that targets it.
 *
 * <p>Subclasses are skipped on Java 8 because the vendored WireMock 3.x JAR requires JRE 11+.
 */
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public abstract class BaseWiremockTest {

  protected WiremockClient wiremock;

  @BeforeAll
  protected void startWiremock() {
    Assumptions.assumeTrue(
        WiremockClient.requiresJava11OrHigher(),
        "WireMock 3.x requires JRE 11+ to launch; skipping wiremock tests on Java 8");
    wiremock = new WiremockClient();
    wiremock.start();
  }

  @AfterAll
  protected void stopWiremock() {
    if (wiremock != null) {
      wiremock.stop();
      wiremock = null;
    }
  }

  @AfterEach
  protected void resetWiremock() {
    if (wiremock != null) {
      wiremock.reset();
    }
  }

  protected String wiremockJdbcUrl() {
    URI uri = URI.create(wiremock.httpUrl());
    return "jdbc:snowflake://" + uri.getHost() + ":" + uri.getPort() + "/";
  }
}
