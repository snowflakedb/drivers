package net.snowflake.jdbc.e2e.authentication;

import static net.snowflake.jdbc.utils.DriverCompatibility.isNewDriver;
import static net.snowflake.jdbc.utils.DriverCompatibility.isOldDriver;
import static net.snowflake.jdbc.utils.TestParameters.loadDefaultConnectionProperties;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.Connection;
import java.sql.SQLException;
import java.util.Properties;
import net.snowflake.jdbc.utils.RequiresNoMfa;
import net.snowflake.jdbc.utils.TestParameters;
import net.snowflake.jdbc.utils.WithConnect;
import net.snowflake.jdbc.utils.WithQueryUtils;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

/**
 * Cross-driver behavior for WIF-only connection params supplied under a non-WIF authenticator.
 *
 * <p>BD#48 (jdbc/BehaviorDifferences.yaml): legacy snowflake-jdbc copies workloadIdentityProvider /
 * workloadIdentityEntraResource / workloadIdentityImpersonationPath into SFLoginInput regardless of
 * authenticator and never reads them unless authenticator=WORKLOAD_IDENTITY, so the params are
 * silently ignored and the connection succeeds. The universal driver's sf_core::validate_settings
 * rejects the combination (ConflictingParameters, Error) — matching legacy
 * snowflake-connector-python's long-standing rejection (ProgrammingError errno 251017).
 */
@RequiresNoMfa
class WorkloadIdentityCrossParamTests implements WithQueryUtils, WithConnect {

  private static final String USER = TestParameters.get("SNOWFLAKE_TEST_USER");
  private static final String PASSWORD = TestParameters.get("SNOWFLAKE_TEST_PASSWORD");

  @Test
  void shouldRejectWifParamUnderNonWifAuthenticatorOnNewDriverButIgnoreOnOldDriver()
      throws Exception {
    // Given a WIF-only param is set but the authenticator is snowflake (not WORKLOAD_IDENTITY)
    Properties props = loadDefaultConnectionProperties();
    props.setProperty("authenticator", "snowflake");
    props.setProperty("user", USER);
    props.setProperty("password", PASSWORD);
    props.setProperty("workload_identity_provider", "AWS");

    // When Trying to Connect
    // Then the new driver rejects the cross-param combination while the legacy driver silently
    // ignores the WIF param and connects (BD#48).
    if (isNewDriver()) {
      // sf_core rejects the cross-param combination before any network I/O (BD#48). Core
      // config-validation errors carry no SQLSTATE / vendor code, so assert on the message.
      // Anchor on both the offending param name AND the distinguishing rejection phrase
      // ("was not set to workload_identity") — the exact text sf_core's ConflictingParameters
      // check emits — so this can't be satisfied by an unrelated error that merely mentions
      // the param name in passing (e.g. a network or attestation failure).
      Executable connect = () -> connect(props);
      SQLException exception = assertThrows(SQLException.class, connect);
      String message = exception.getMessage().toLowerCase();
      assertTrue(
          message.contains("workload_identity_provider"),
          () ->
              "Expected the WIF cross-param rejection to name the offending parameter "
                  + "'workload_identity_provider', but got: "
                  + exception.getMessage());
      assertTrue(
          message.contains("was not set to workload_identity"),
          () ->
              "Expected the WIF cross-param rejection to include the distinguishing phrase "
                  + "'was not set to WORKLOAD_IDENTITY', but got: "
                  + exception.getMessage());
    }

    if (isOldDriver()) {
      // Legacy snowflake-jdbc silently ignores the WIF param and the connection succeeds (BD#48).
      try (Connection conn = connect(props)) {
        assertSimpleQuerySucceeds(conn);
      }
    }
  }
}
