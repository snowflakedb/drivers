package net.snowflake.jdbc.e2e.session;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.lang.management.ManagementFactory;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.attribute.PosixFilePermissions;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Properties;
import java.util.concurrent.TimeUnit;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.jdbc.utils.DriverCompatibility;
import net.snowflake.jdbc.utils.SkipOldDriver;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.condition.EnabledOnOs;
import org.junit.jupiter.api.condition.OS;
import org.junit.jupiter.api.io.TempDir;

/**
 * File-backed coverage for {@code jdbc:snowflake:auto}: real {@code connections.toml} and {@code
 * config.toml} files on disk, reached through {@code SNOWFLAKE_HOME}.
 *
 * <p>{@code SNOWFLAKE_HOME} is read by sf_core through the process environment, so each case forks
 * a worker JVM with the variable set. The worker records the SQLState, vendor code and message for
 * each profile it resolves into a properties file the test then asserts on.
 *
 * <p>When run against the new driver, no probe reaches the network: {@code FIXTURE_AUTHENTICATOR}
 * is not an allowed authenticator, so settings validation fails before an account, server URL or
 * HTTP client is built. That value is what keeps these cases offline — the fixture's {@code
 * account} is syntactically valid and would resolve to a real host if the authenticator were ever
 * made legitimate.
 *
 * <p>Restricted to POSIX hosts: the fixtures rely on file modes, and sf_core performs the
 * permission check on Unix only.
 */
@EnabledOnOs({OS.LINUX, OS.MAC})
class AutoConnectionConfigFileTests {

  private static final String PROFILE_NAME = "fromFile";

  private static final String ABSENT_PROFILE_NAME = "absentProfile";

  private static final String FIXTURE_AUTHENTICATOR = "not_a_real_authenticator";

  private static final String DEFAULT_PROFILE_AUTHENTICATOR = "default_profile_authenticator";

  private static final String BARE_AUTO_OUTCOME = "bareAuto";

  private static final String DIRECT_DRIVER_CONNECT_OUTCOME = "directDriverConnect";

  /**
   * Each probe fails in settings validation, which takes well under a second. A worker still
   * running after this long has reached the network, and the bound turns that into a readable
   * failure instead of a hung Gradle task.
   */
  private static final int WORKER_TIMEOUT_SECONDS = 60;

  private static final String CONNECTIONS_TOML =
      "["
          + PROFILE_NAME
          + "]\n"
          + "account = \"testaccount\"\n"
          + "user = \"testuser\"\n"
          + "authenticator = \""
          + FIXTURE_AUTHENTICATOR
          + "\"\n"
          + "[default]\n"
          + "account = \"testaccount\"\n"
          + "user = \"testuser\"\n"
          + "authenticator = \""
          + DEFAULT_PROFILE_AUTHENTICATOR
          + "\"\n";

  @Test
  void shouldRejectAWorldWritableConnectionsTomlUnderSnowflakeHome(@TempDir Path snowflakeHome)
      throws Exception {
    // Given a connections.toml under SNOWFLAKE_HOME that anyone may write to
    Path connectionsToml = writeConnectionsToml(snowflakeHome, "rw-rw-rw-");

    // When a JDBC connection is requested with jdbc:snowflake:auto?connectionName=fromFile
    Properties outcome = probe(snowflakeHome, PROFILE_NAME);

    // Then the config loader refuses the file and names it in the failure
    String failure = message(outcome, PROFILE_NAME);
    String expectedPermissionFailure =
        DriverCompatibility.isOldDriver()
            ? "writable by group or others"
            : "Insecure file permissions";
    assertTrue(
        failure.contains(expectedPermissionFailure),
        "Expected the loader to reject the world-writable fixture, got: " + failure);
    assertTrue(
        failure.contains(connectionsToml.toString()),
        "Expected the failure to name " + connectionsToml + ", got: " + failure);
    if (!DriverCompatibility.isOldDriver()) {
      assertEquals(
          null,
          outcome.getProperty(PROFILE_NAME + ".sqlState"),
          "Expected an unmapped permission failure to carry no SQLState");
      assertEquals(
          "0",
          outcome.getProperty(PROFILE_NAME + ".errorCode"),
          "Expected an unmapped permission failure to carry vendor code 0");
    }
  }

  @Test
  @SkipOldDriver("BD#71")
  void shouldResolveADefinedProfileAndRejectAnUndefinedOneFromConnectionsTomlUnderSnowflakeHome(
      @TempDir Path snowflakeHome) throws Exception {
    // Given an owner-only connections.toml under SNOWFLAKE_HOME defining the fromFile profile
    writeConnectionsToml(snowflakeHome, "rw-------");

    // When JDBC connections are requested for a defined and an undefined profile name
    Properties outcome = probe(snowflakeHome, PROFILE_NAME, ABSENT_PROFILE_NAME);

    // Then the undefined name is reported as a missing parameter with the auto-URL error identity
    String absentFailure = message(outcome, ABSENT_PROFILE_NAME);
    assertEquals(jdbcDriverMissingProfileMessage(ABSENT_PROFILE_NAME), absentFailure);
    assertEquals(
        ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(),
        outcome.getProperty(ABSENT_PROFILE_NAME + ".sqlState"));
    assertEquals(
        String.valueOf(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode()),
        outcome.getProperty(ABSENT_PROFILE_NAME + ".errorCode"));

    // And the defined profile is located, so its own settings drive the failure instead
    String loadedFailure = message(outcome, PROFILE_NAME);
    assertFalse(
        loadedFailure.contains("not found in connections.toml file."),
        "Expected the "
            + PROFILE_NAME
            + " profile to be located in the fixture, got: "
            + loadedFailure);
    assertTrue(
        loadedFailure.contains(FIXTURE_AUTHENTICATOR),
        "Expected the failure to come from the fixture's own authenticator value, got: "
            + loadedFailure);
  }

  @Test
  @SkipOldDriver("BD#71")
  void shouldLoadTheDefaultProfileForABareAutoConnectionUrl(@TempDir Path snowflakeHome)
      throws Exception {
    // Given an owner-only connections.toml under SNOWFLAKE_HOME defining the default profile
    writeConnectionsToml(snowflakeHome, "rw-------");

    // When a JDBC connection is requested with the bare auto URL
    Properties outcome = probe(snowflakeHome, BARE_AUTO_OUTCOME);

    // Then the default profile is located and its own authenticator drives the failure
    String failure = message(outcome, BARE_AUTO_OUTCOME);
    assertTrue(
        failure.contains(DEFAULT_PROFILE_AUTHENTICATOR),
        "Expected the default profile's authenticator to drive the failure, got: " + failure);
  }

  @Test
  @SkipOldDriver("BD#71")
  void shouldLoadTheProfileNamedByConfigTomlForABareAutoConnectionUrl(@TempDir Path snowflakeHome)
      throws Exception {
    // Given config.toml names the fromFile profile as the default
    writeConnectionsToml(snowflakeHome, "rw-------");
    writeConfigToml(snowflakeHome, "default_connection_name = \"" + PROFILE_NAME + "\"\n");

    // When a JDBC connection is requested with the bare auto URL
    Properties outcome = probe(snowflakeHome, BARE_AUTO_OUTCOME);

    // Then the config-selected profile's authenticator drives the failure
    String failure = message(outcome, BARE_AUTO_OUTCOME);
    assertTrue(
        failure.contains(FIXTURE_AUTHENTICATOR),
        "Expected the config-selected profile's authenticator to drive the failure, got: "
            + failure);
  }

  @Test
  @SkipOldDriver("BD#71")
  void shouldLoadTheProfileDefinedInConfigTomlWhenConnectionsTomlIsAbsent(
      @TempDir Path snowflakeHome) throws Exception {
    // Given only config.toml defines the fromFile profile under [connections.fromFile]
    writeConfigToml(
        snowflakeHome,
        "[connections."
            + PROFILE_NAME
            + "]\n"
            + "account = \"testaccount\"\n"
            + "user = \"testuser\"\n"
            + "authenticator = \""
            + FIXTURE_AUTHENTICATOR
            + "\"\n");

    // When a JDBC connection is requested with jdbc:snowflake:auto?connectionName=fromFile
    Properties outcome = probe(snowflakeHome, PROFILE_NAME);

    // Then the config.toml profile is located and its own authenticator drives the failure
    String failure = message(outcome, PROFILE_NAME);
    assertTrue(
        failure.contains(FIXTURE_AUTHENTICATOR),
        "Expected the config.toml profile's authenticator to drive the failure, got: " + failure);
  }

  @Test
  @SkipOldDriver("BD#71")
  void shouldReportTheDefaultProfileMissingWhenNoConfigFilesExist(@TempDir Path snowflakeHome)
      throws Exception {
    // When a JDBC connection is requested with the bare auto URL
    Properties outcome = probe(snowflakeHome, BARE_AUTO_OUTCOME);

    // Then the literal default profile is reported with the auto-URL error identity
    String failure = message(outcome, BARE_AUTO_OUTCOME);
    assertEquals(jdbcDriverMissingProfileMessage("default"), failure);
    assertEquals(
        ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(),
        outcome.getProperty(BARE_AUTO_OUTCOME + ".sqlState"));
    assertEquals(
        String.valueOf(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode()),
        outcome.getProperty(BARE_AUTO_OUTCOME + ".errorCode"));
  }

  @Test
  @SkipOldDriver("BD#71")
  void shouldLoadTheProfileNamedByTheEnvironmentForABareAutoConnectionUrl(
      @TempDir Path snowflakeHome) throws Exception {
    // Given an owner-only connections.toml and a process default naming the fromFile profile
    writeConnectionsToml(snowflakeHome, "rw-------");

    // When a JDBC connection is requested with the bare auto URL
    Properties outcome =
        probeWithDefaultConnectionName(snowflakeHome, PROFILE_NAME, BARE_AUTO_OUTCOME);

    // Then the environment-selected profile's authenticator drives the failure
    String failure = message(outcome, BARE_AUTO_OUTCOME);
    assertTrue(
        failure.contains(FIXTURE_AUTHENTICATOR),
        "Expected the environment-selected profile's authenticator to drive the failure, got: "
            + failure);
  }

  @Test
  @SkipOldDriver("BD#71")
  void shouldReportAMissingProfileNamedByTheEnvironmentForABareAutoConnectionUrl(
      @TempDir Path snowflakeHome) throws Exception {
    // Given an owner-only connections.toml and a process default naming an absent profile
    writeConnectionsToml(snowflakeHome, "rw-------");

    // When a JDBC connection is requested with the bare auto URL
    Properties outcome =
        probeWithDefaultConnectionName(snowflakeHome, ABSENT_PROFILE_NAME, BARE_AUTO_OUTCOME);

    // Then the environment-selected name is reported with the auto-URL error identity
    String failure = message(outcome, BARE_AUTO_OUTCOME);
    assertEquals(jdbcDriverMissingProfileMessage(ABSENT_PROFILE_NAME), failure);
    assertEquals(
        ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(),
        outcome.getProperty(BARE_AUTO_OUTCOME + ".sqlState"));
    assertEquals(
        String.valueOf(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode()),
        outcome.getProperty(BARE_AUTO_OUTCOME + ".errorCode"));
  }

  @Test
  @SkipOldDriver("BD#71")
  void shouldReportAMissingProfileFromDirectDriverConnectWhenTheEnvironmentNamesAnAbsentProfile(
      @TempDir Path snowflakeHome) throws Exception {
    // Given an owner-only connections.toml and a process default naming an absent profile
    writeConnectionsToml(snowflakeHome, "rw-------");

    // When SnowflakeDriver.connect() is called directly with that process default
    Properties outcome =
        probeWithDefaultConnectionName(
            snowflakeHome, ABSENT_PROFILE_NAME, DIRECT_DRIVER_CONNECT_OUTCOME);

    // Then the environment-selected name matches the reference JDBC driver's missing-profile
    // wording
    assertEquals(
        jdbcDriverMissingProfileMessage(ABSENT_PROFILE_NAME),
        message(outcome, DIRECT_DRIVER_CONNECT_OUTCOME));
    assertEquals(
        ErrorCode.INVALID_PARAMETER_VALUE.getSqlState(),
        outcome.getProperty(DIRECT_DRIVER_CONNECT_OUTCOME + ".sqlState"));
    assertEquals(
        String.valueOf(ErrorCode.INVALID_PARAMETER_VALUE.getMessageCode()),
        outcome.getProperty(DIRECT_DRIVER_CONNECT_OUTCOME + ".errorCode"));
  }

  /**
   * Worker entry point. {@code args[0]} is the properties file to write; the remaining arguments
   * are the profile names to resolve through {@code jdbc:snowflake:auto}.
   */
  public static void main(String[] args) throws IOException {
    Properties outcome = new Properties();
    for (String connectionName : Arrays.asList(args).subList(1, args.length)) {
      record(outcome, connectionName);
    }
    try (OutputStream out = Files.newOutputStream(Paths.get(args[0]))) {
      outcome.store(out, null);
    }
  }

  private static void record(Properties outcome, String connectionName) {
    try (Connection ignored = openAutoConnection(connectionName)) {
      outcome.setProperty(connectionName + ".message", "<connected>");
    } catch (SQLException e) {
      outcome.setProperty(connectionName + ".message", String.valueOf(e.getMessage()));
      outcome.setProperty(connectionName + ".errorCode", String.valueOf(e.getErrorCode()));
      if (e.getSQLState() != null) {
        outcome.setProperty(connectionName + ".sqlState", e.getSQLState());
      }
    }
  }

  private static Connection openAutoConnection(String connectionName) throws SQLException {
    if (DIRECT_DRIVER_CONNECT_OUTCOME.equals(connectionName)) {
      return new SnowflakeDriver().connect("jdbc:snowflake:auto", new Properties());
    }
    String url =
        BARE_AUTO_OUTCOME.equals(connectionName)
            ? "jdbc:snowflake:auto"
            : "jdbc:snowflake:auto?connectionName=" + connectionName;
    return DriverManager.getConnection(url);
  }

  private static String jdbcDriverMissingProfileMessage(String profileName) {
    return "The Connection " + profileName + " not found in connections.toml file.";
  }

  private static String message(Properties outcome, String connectionName) {
    String failure = outcome.getProperty(connectionName + ".message");
    assertNotNull(failure, "Worker recorded no outcome for " + connectionName);
    assertFalse(failure.isEmpty(), "Worker recorded an empty outcome for " + connectionName);
    return failure;
  }

  private static Path writeConnectionsToml(Path snowflakeHome, String posixPermissions)
      throws IOException {
    Path connectionsToml = snowflakeHome.resolve("connections.toml");
    Files.write(connectionsToml, CONNECTIONS_TOML.getBytes(StandardCharsets.UTF_8));
    Files.setPosixFilePermissions(
        connectionsToml, PosixFilePermissions.fromString(posixPermissions));
    return connectionsToml;
  }

  private static void writeConfigToml(Path snowflakeHome, String contents) throws IOException {
    Path configToml = snowflakeHome.resolve("config.toml");
    Files.write(configToml, contents.getBytes(StandardCharsets.UTF_8));
    Files.setPosixFilePermissions(configToml, PosixFilePermissions.fromString("rw-------"));
  }

  private static Properties probe(Path snowflakeHome, String... connectionNames) throws Exception {
    return runProbe(snowflakeHome, null, connectionNames);
  }

  private static Properties probeWithDefaultConnectionName(
      Path snowflakeHome, String defaultConnectionName, String... connectionNames)
      throws Exception {
    return runProbe(snowflakeHome, defaultConnectionName, connectionNames);
  }

  private static Properties runProbe(
      Path snowflakeHome, String defaultConnectionName, String... connectionNames)
      throws Exception {
    Path outcomeFile = Files.createTempFile("auto-connection-probe", ".properties");
    Path outputFile = Files.createTempFile("auto-connection-probe", ".log");
    try {
      ProcessBuilder worker = new ProcessBuilder(buildWorkerCommand(outcomeFile, connectionNames));
      worker.environment().put("SNOWFLAKE_HOME", snowflakeHome.toString());
      if (defaultConnectionName != null) {
        worker.environment().put("SNOWFLAKE_DEFAULT_CONNECTION_NAME", defaultConnectionName);
      } else {
        worker.environment().remove("SNOWFLAKE_DEFAULT_CONNECTION_NAME");
      }
      String corePath = System.getenv("CORE_PATH");
      if (corePath != null) {
        worker.environment().put("CORE_PATH", corePath);
      }
      worker.redirectErrorStream(true);
      worker.redirectOutput(outputFile.toFile());

      awaitWorkerCompletion(worker.start(), outputFile);

      Properties outcome = new Properties();
      try (InputStream in = Files.newInputStream(outcomeFile)) {
        outcome.load(in);
      }
      return outcome;
    } finally {
      Files.deleteIfExists(outcomeFile);
      Files.deleteIfExists(outputFile);
    }
  }

  private static void awaitWorkerCompletion(Process process, Path outputFile) throws Exception {
    try {
      boolean exited = process.waitFor(WORKER_TIMEOUT_SECONDS, TimeUnit.SECONDS);
      String output = new String(Files.readAllBytes(outputFile), StandardCharsets.UTF_8);
      assertTrue(
          exited,
          "Worker JVM did not exit within "
              + WORKER_TIMEOUT_SECONDS
              + "s; a probe reached the network instead of failing on "
              + FIXTURE_AUTHENTICATOR
              + ":\n"
              + output);
      assertEquals(0, process.exitValue(), "Worker JVM failed:\n" + output);
    } finally {
      if (process.isAlive()) {
        process.destroyForcibly();
        assertTrue(
            process.waitFor(WORKER_TIMEOUT_SECONDS, TimeUnit.SECONDS),
            "Worker JVM did not terminate after forced destruction");
      }
    }
  }

  private static List<String> buildWorkerCommand(Path outcomeFile, String... connectionNames) {
    List<String> command = new ArrayList<>();
    command.add(Paths.get(System.getProperty("java.home"), "bin", "java").toString());
    for (String arg : ManagementFactory.getRuntimeMXBean().getInputArguments()) {
      if (arg.startsWith("--add-opens") || arg.startsWith("--enable-native-access")) {
        command.add(arg);
      }
    }
    command.add("-cp");
    command.add(System.getProperty("java.class.path"));
    command.add(AutoConnectionConfigFileTests.class.getName());
    command.add(outcomeFile.toString());
    command.addAll(Arrays.asList(connectionNames));
    return command;
  }
}
