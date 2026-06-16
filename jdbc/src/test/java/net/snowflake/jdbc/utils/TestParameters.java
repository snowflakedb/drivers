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
import org.json.JSONArray;
import org.json.JSONObject;
import org.json.JSONTokener;

@NoArgsConstructor(access = AccessLevel.PRIVATE)
public class TestParameters {

  private static volatile JSONObject params;

  private static JSONObject get() throws Exception {
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
        JSONObject params = new JSONObject(new JSONTokener(new InputStreamReader(input)));
        TestParameters.params = params.getJSONObject("testconnection");
      }
      return params;
    }
  }

  public static String get(String key) throws Exception {
    return TestParameters.get().getString(key);
  }

  public static List<String> getList(String key) throws Exception {
    List<String> result = new ArrayList<>();
    JSONArray jsonArray = TestParameters.get().getJSONArray(key);
    for (int i = 0; i < jsonArray.length(); i++) {
      result.add(jsonArray.getString(i));
    }
    return result;
  }

  public static Properties loadConnectionProperties() throws Exception {
    JSONObject params = TestParameters.get();

    Properties props = new Properties();
    props.setProperty("account", params.getString("SNOWFLAKE_TEST_ACCOUNT"));
    props.setProperty("host", params.getString("SNOWFLAKE_TEST_HOST"));
    props.setProperty("role", params.getString("SNOWFLAKE_TEST_ROLE"));

    props.setProperty("schema", params.getString("SNOWFLAKE_TEST_SCHEMA"));
    props.setProperty("db", params.getString("SNOWFLAKE_TEST_DATABASE"));
    props.setProperty(
        "warehouse",
        params.has("SNOWFLAKE_TEST_WAREHOUSE_JDBC")
            ? params.getString("SNOWFLAKE_TEST_WAREHOUSE_JDBC")
            : params.getString("SNOWFLAKE_TEST_WAREHOUSE"));

    addOptionalConnectionProperties(params, props);
    return props;
  }

  private static void addOptionalConnectionProperties(JSONObject params, Properties props) {
    if (params.has("SNOWFLAKE_TEST_USER")) {
      props.setProperty("user", params.getString("SNOWFLAKE_TEST_USER"));
    }
    if (params.has("SNOWFLAKE_TEST_PASSWORD")) {
      props.setProperty("password", params.getString("SNOWFLAKE_TEST_PASSWORD"));
    }

    if (params.has("SNOWFLAKE_TEST_PORT")) {
      props.setProperty("port", String.valueOf(params.getInt("SNOWFLAKE_TEST_PORT")));
    }
    if (params.has("SNOWFLAKE_TEST_SERVER_URL")) {
      props.setProperty("server_url", params.getString("SNOWFLAKE_TEST_SERVER_URL"));
    }
    if (params.has("SNOWFLAKE_TEST_PROTOCOL")) {
      props.setProperty("protocol", params.getString("SNOWFLAKE_TEST_PROTOCOL"));
    }
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
