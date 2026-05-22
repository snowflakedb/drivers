package net.snowflake.jdbc.utils;

import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.file.Files;
import java.nio.file.Paths;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;
import org.json.JSONObject;
import org.json.JSONTokener;

@NoArgsConstructor(access = AccessLevel.PRIVATE)
public class TestParameters {

  private static volatile JSONObject params;

  public static JSONObject get() throws Exception {
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
}
