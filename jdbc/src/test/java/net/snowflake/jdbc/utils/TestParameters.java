package net.snowflake.jdbc.utils;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Properties;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import lombok.SneakyThrows;

/**
 * Test connection parameters from {@code parameters.json} (or {@code PARAMETER_PATH}). Loads the
 * base {@code testconnection} section, then overlays {@code testconnection-jdbc} when present.
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public class TestParameters {

  private static volatile ObjectNode params;

  @SneakyThrows
  private static ObjectNode get() {
    if (params != null) {
      return params;
    }
    synchronized (TestParameters.class) {
      if (params != null) {
        return params;
      }
      String paramPath = System.getenv("PARAMETER_PATH");
      if (paramPath == null) {
        paramPath = "/parameters.json";
      }
      try (InputStream input = Files.newInputStream(Paths.get(paramPath))) {
        JsonNode parametersJson = JsonTestUtils.mapper().readTree(input);
        ObjectNode resolvedParams = (ObjectNode) parametersJson.get("testconnection");
        if (parametersJson.has("testconnection-jdbc")) {
          resolvedParams.setAll((ObjectNode) parametersJson.get("testconnection-jdbc"));
        }
        params = resolvedParams;
      }
    }
    return params;
  }

  public static boolean has(String key) {
    return TestParameters.get().has(key);
  }

  public static String get(String key) {
    return TestParameters.get().get(key).asText();
  }

  public static int getInt(String key) {
    return TestParameters.get().get(key).asInt();
  }

  public static List<String> getList(String key) {
    List<String> result = new ArrayList<>();
    for (JsonNode element : TestParameters.get().get(key)) {
      result.add(element.asText());
    }
    return result;
  }

  /** Builds a {@link Properties} from alternating key/value arguments. */
  public static Properties props(String... keyValues) {
    Properties p = new Properties();
    for (int i = 0; i < keyValues.length; i += 2) {
      p.setProperty(keyValues[i], keyValues[i + 1]);
    }
    return p;
  }

  /**
   * Loads the base connection properties that are independent of authentication: {@code account},
   * {@code host}, {@code role}, {@code schema}, {@code db}, {@code warehouse}, and the optional
   * {@code port}, {@code server_url} and {@code protocol}. No credentials (user/password) are set;
   * callers choose an authentication method explicitly (e.g. via {@link #withSnowflakeAuth}).
   */
  public static Properties loadDefaultConnectionProperties() {
    Properties props = new Properties();
    props.setProperty("account", get("SNOWFLAKE_TEST_ACCOUNT"));
    props.setProperty("host", get("SNOWFLAKE_TEST_HOST"));
    props.setProperty("role", get("SNOWFLAKE_TEST_ROLE"));

    props.setProperty("schema", get("SNOWFLAKE_TEST_SCHEMA"));
    props.setProperty("db", get("SNOWFLAKE_TEST_DATABASE"));
    props.setProperty(
        "warehouse",
        has("SNOWFLAKE_TEST_WAREHOUSE_JDBC")
            ? get("SNOWFLAKE_TEST_WAREHOUSE_JDBC")
            : get("SNOWFLAKE_TEST_WAREHOUSE"));

    if (has("SNOWFLAKE_TEST_PORT")) {
      props.setProperty("port", String.valueOf(getInt("SNOWFLAKE_TEST_PORT")));
    }
    if (has("SNOWFLAKE_TEST_SERVER_URL")) {
      props.setProperty("server_url", get("SNOWFLAKE_TEST_SERVER_URL"));
    }
    if (has("SNOWFLAKE_TEST_PROTOCOL")) {
      props.setProperty("protocol", get("SNOWFLAKE_TEST_PROTOCOL"));
    }
    return props;
  }

  public static Properties withSnowflakeAuth(Properties props) {
    props.setProperty("user", get("SNOWFLAKE_TEST_USER"));
    props.setProperty("password", get("SNOWFLAKE_TEST_PASSWORD"));
    return props;
  }

  /**
   * Configures key pair (SNOWFLAKE_JWT) auth using SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS. Preferred
   * over password auth on accounts that enforce MFA for password logins.
   */
  private static Properties withPrivateKeyAuth(Properties props) {
    props.setProperty("user", get("SNOWFLAKE_TEST_USER"));
    String pemContent = String.join("\n", getList("SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS"));
    props.setProperty(
        "private_key_base64",
        Base64.getEncoder().encodeToString(pemContent.getBytes(StandardCharsets.UTF_8)));
    if (has("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD")
        && !get("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD").isEmpty()) {
      props.setProperty("private_key_pwd", get("SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD"));
    }
    props.setProperty("authenticator", "SNOWFLAKE_JWT");
    return props;
  }

  /**
   * Returns props configured with the best available auth method. Uses key pair (SNOWFLAKE_JWT)
   * auth by default; falls back to password auth (SNOWFLAKE_TEST_USER / SNOWFLAKE_TEST_PASSWORD)
   * when SNOWFLAKE_TEST_IS_USUT is set, because uSUT only provisions password credentials for the
   * test user — no RSA key is registered.
   */
  public static Properties withDefaultAuth(Properties props) {
    if (has("SNOWFLAKE_TEST_IS_USUT")) {
      return withSnowflakeAuth(props);
    }
    return withPrivateKeyAuth(props);
  }

  public static String buildJdbcUrl() {
    return buildJdbcUrl(loadDefaultConnectionProperties());
  }

  public static String buildJdbcUrl(Properties props) {
    String url;
    if (props.getProperty("host") != null) {
      url = "jdbc:snowflake://" + props.getProperty("host");
    } else {
      url = "jdbc:snowflake://" + props.getProperty("account") + ".snowflakecomputing.com";
    }
    if (props.getProperty("port") != null) {
      url += ":" + props.getProperty("port");
    }
    return props.getProperty("url", url);
  }
}
