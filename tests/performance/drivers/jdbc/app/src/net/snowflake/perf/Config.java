package net.snowflake.perf;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Properties;
// Jackson is bundled in the fat jar, relocated by shadowJar (jdbc/build.gradle).
import net.snowflake.client.jdbc.internal.com.fasterxml.jackson.databind.JsonNode;
import net.snowflake.client.jdbc.internal.com.fasterxml.jackson.databind.ObjectMapper;

/**
 * Config from env vars + the {@code PARAMETERS_JSON} blob. Connection property names follow the
 * JDBC driver's convention (see {@code TestParameters}): {@code db}, {@code private_key_base64} +
 * {@code authenticator=SNOWFLAKE_JWT}, etc. — not the python driver's names.
 */
final class Config {

  final String driverType;
  final String testType;
  final String sqlCommand;
  final String testName;
  final int iterations;
  final int warmupIterations;

  private static final ObjectMapper MAPPER = new ObjectMapper();

  private final JsonNode conn;
  private final String setupQueriesJson;

  Config() {
    this.driverType = envOrDefault("DRIVER_TYPE", "universal");
    this.testType = envOrDefault("TEST_TYPE", "select");
    this.sqlCommand = System.getenv("SQL_COMMAND");
    this.testName = System.getenv("TEST_NAME");
    this.iterations = Integer.parseInt(envOrDefault("PERF_ITERATIONS", "1"));
    this.warmupIterations = Integer.parseInt(envOrDefault("PERF_WARMUP_ITERATIONS", "0"));
    this.setupQueriesJson = System.getenv("SETUP_QUERIES");

    String paramsJson = System.getenv("PARAMETERS_JSON");
    if (sqlCommand == null || testName == null || paramsJson == null) {
      throw new IllegalStateException(
          "Missing required environment variables (SQL_COMMAND, TEST_NAME, PARAMETERS_JSON)");
    }
    JsonNode testConnection = parseJson(paramsJson).get("testconnection");
    if (testConnection == null || !testConnection.isObject()) {
      throw new IllegalStateException("PARAMETERS_JSON is missing a 'testconnection' object");
    }
    this.conn = testConnection;
  }

  List<String> setupQueries() {
    List<String> result = new ArrayList<>();
    if (setupQueriesJson != null && !setupQueriesJson.isEmpty()) {
      JsonNode arr = parseJson(setupQueriesJson);
      for (int i = 0; i < arr.size(); i++) {
        result.add(arr.get(i).asText());
      }
    }
    return result;
  }

  /** Builds the JDBC connection {@link Properties} for the configured driver. */
  Properties connectionProperties() {
    Properties props = new Properties();
    setIfPresent(props, "account", "SNOWFLAKE_TEST_ACCOUNT", "account");
    setIfPresent(props, "host", "SNOWFLAKE_TEST_HOST", "host");
    setIfPresent(props, "user", "SNOWFLAKE_TEST_USER", "user");
    setIfPresent(props, "role", "SNOWFLAKE_TEST_ROLE", "role");
    setIfPresent(props, "schema", "SNOWFLAKE_TEST_SCHEMA", "schema");
    setIfPresent(props, "db", "SNOWFLAKE_TEST_DATABASE", "database");
    // JDBC prefers the JDBC-specific warehouse when present (mirrors TestParameters).
    if (conn.has("SNOWFLAKE_TEST_WAREHOUSE_JDBC")) {
      props.setProperty("warehouse", conn.get("SNOWFLAKE_TEST_WAREHOUSE_JDBC").asText());
    } else {
      setIfPresent(props, "warehouse", "SNOWFLAKE_TEST_WAREHOUSE", "warehouse");
    }
    setIfPresent(props, "port", "SNOWFLAKE_TEST_PORT", "port");
    setIfPresent(props, "server_url", "SNOWFLAKE_TEST_SERVER_URL", "server_url");
    setIfPresent(props, "protocol", "SNOWFLAKE_TEST_PROTOCOL", "protocol");

    applyKeyPairAuth(props);
    applyWiremockProxy(props);
    return props;
  }

  /**
   * Opt the universal driver into HTTP(S)_PROXY env detection for the recorded-HTTP (WireMock) lane.
   *
   * <p>The WireMock harness routes traffic via proxy env vars, not explicit proxy properties, and
   * sf_core ignores those env vars unless {@code use_proxy_env=true} (mirrors the odbc app's
   * {@code USE_PROXY_ENV=true} and python's {@code _enable_proxy_env_for_wiremock}). No CA/OCSP knob
   * is needed: the driver's TLS runs in sf_core, which loads the OS CA bundle the Dockerfile appends
   * the WireMock CA to, and defaults to OCSP FAIL_OPEN.
   */
  private void applyWiremockProxy(Properties props) {
    boolean proxyEnvSet =
        !isBlank(System.getenv("HTTPS_PROXY")) || !isBlank(System.getenv("HTTP_PROXY"));
    if (proxyEnvSet) {
      props.setProperty("use_proxy_env", "true");
      System.out.println("Universal driver: use_proxy_env=true (WireMock proxy env detected)");
    }
  }

  String jdbcUrl(Properties props) {
    String url;
    if (props.getProperty("host") != null) {
      url = "jdbc:snowflake://" + props.getProperty("host");
    } else {
      url = "jdbc:snowflake://" + props.getProperty("account") + ".snowflakecomputing.com";
    }
    if (props.getProperty("port") != null) {
      url += ":" + props.getProperty("port");
    }
    return url;
  }

  /** Key-pair (SNOWFLAKE_JWT) auth: PEM passed as base64 {@code private_key_base64}. */
  private void applyKeyPairAuth(Properties props) {
    List<String> pemLines = getList("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS");
    if (pemLines.isEmpty()) {
      throw new IllegalStateException(
          "No SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS in testconnection; key-pair auth is required");
    }
    // user was resolved in connectionProperties(); JWT needs it set.
    if (props.getProperty("user") == null) {
      throw new IllegalStateException(
          "No user in testconnection (SNOWFLAKE_TEST_USER/user); key-pair auth is required");
    }
    String pem = String.join("\n", pemLines);
    props.setProperty(
        "private_key_base64",
        Base64.getEncoder().encodeToString(pem.getBytes(StandardCharsets.UTF_8)));
    if (conn.has("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")
        && !conn.get("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD").asText().isEmpty()) {
      props.setProperty(
          "private_key_pwd", conn.get("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD").asText());
    }
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
  }

  private void setIfPresent(Properties props, String propKey, String primary, String fallback) {
    if (conn.has(primary)) {
      props.setProperty(propKey, conn.get(primary).asText());
    } else if (conn.has(fallback)) {
      props.setProperty(propKey, conn.get(fallback).asText());
    }
  }

  private List<String> getList(String key) {
    List<String> result = new ArrayList<>();
    if (conn.has(key)) {
      JsonNode arr = conn.get(key);
      for (int i = 0; i < arr.size(); i++) {
        result.add(arr.get(i).asText());
      }
    }
    return result;
  }

  private static JsonNode parseJson(String content) {
    try {
      return MAPPER.readTree(content);
    } catch (IOException e) {
      throw new RuntimeException("Failed to parse JSON: " + content, e);
    }
  }

  private static String envOrDefault(String key, String def) {
    String v = System.getenv(key);
    return (v == null || v.isEmpty()) ? def : v;
  }

  private static boolean isBlank(String s) {
    return s == null || s.trim().isEmpty();
  }
}
