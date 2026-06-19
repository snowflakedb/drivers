package net.snowflake.jdbc.utils;

import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;
import java.util.Properties;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import lombok.SneakyThrows;
import org.json.JSONArray;
import org.json.JSONObject;
import org.json.JSONTokener;

/**
 * Test connection parameters from {@code parameters.json} (or {@code PARAMETER_PATH}). Loads the
 * base {@code testconnection} section, then overlays {@code testconnection-jdbc} when present.
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public class TestParameters {

  private static volatile JSONObject params;

  @SneakyThrows
  private static JSONObject get() {
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
        JSONObject parametersJson = new JSONObject(new JSONTokener(new InputStreamReader(input)));
        JSONObject resolvedParams = parametersJson.getJSONObject("testconnection");
        if (parametersJson.has("testconnection-jdbc")) {
          JSONObject overridesJson = parametersJson.getJSONObject("testconnection-jdbc");
          overridesJson.keys().forEachRemaining(k -> resolvedParams.put(k, overridesJson.get(k)));
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
    return TestParameters.get().getString(key);
  }

  public static int getInt(String key) {
    return TestParameters.get().getInt(key);
  }

  public static List<String> getList(String key) {
    List<String> result = new ArrayList<>();
    JSONArray jsonArray = TestParameters.get().getJSONArray(key);
    for (int i = 0; i < jsonArray.length(); i++) {
      result.add(jsonArray.getString(i));
    }
    return result;
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

  public static String buildJdbcUrl() {
    return buildJdbcUrl(loadDefaultConnectionProperties());
  }

  public static String buildJdbcUrl(Properties props) {
    String defaultUrl =
        "jdbc:snowflake://" + props.getProperty("account") + ".snowflakecomputing.com";
    if (props.getProperty("port") != null) {
      defaultUrl += ":" + props.getProperty("port");
    }
    return props.getProperty("url", defaultUrl);
  }
}
