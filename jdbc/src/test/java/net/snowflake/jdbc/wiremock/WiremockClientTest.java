package net.snowflake.jdbc.wiremock;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import net.snowflake.jdbc.testutil.HttpTestClient;
import net.snowflake.jdbc.testutil.HttpTestClient.Response;
import org.json.JSONObject;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;

/** Self-tests for {@link WiremockClient}. Skipped on Java 8 (WireMock 3.x requires JRE 11+). */
public class WiremockClientTest {

  @BeforeAll
  public static void skipOnJava8() {
    Assumptions.assumeTrue(
        WiremockClient.requiresJava11OrHigher(),
        "WireMock 3.x requires JRE 11+ to launch; skipping wiremock tests on Java 8");
  }

  @Test
  public void inlineMappingIsServedAndRecorded() {
    try (WiremockClient client = new WiremockClient();
        HttpTestClient http = new HttpTestClient()) {
      client.start();
      client.addMappingJson(
          new JSONObject()
              .put("request", new JSONObject().put("method", "GET").put("urlPath", "/ping"))
              .put(
                  "response",
                  new JSONObject()
                      .put("status", 200)
                      .put("body", "pong")
                      .put("headers", new JSONObject().put("Content-Type", "text/plain")))
              .toString());

      Response resp = http.get(client.httpUrl() + "/ping");
      assertEquals(200, resp.status());
      assertEquals("pong", resp.body());

      client.verifyRequestCount(1, "/ping");
      List<JSONObject> recorded = client.getRequests("/ping");
      assertEquals(1, recorded.size());

      client.reset();
      assertEquals(0, client.getRequests("/ping").size());
    }
  }

  @Test
  public void fileBackedMappingIsResolvedFromMappingsDir() {
    // tests/wiremock/mappings/session/logout_success.json stubs POST /session?delete=true → 200.
    try (WiremockClient client = new WiremockClient();
        HttpTestClient http = new HttpTestClient()) {
      client.start();
      client.addMapping("session/logout_success.json");

      Response resp = http.post(client.httpUrl() + "/session?delete=true", "{}");
      assertEquals(200, resp.status());
      assertTrue(resp.body().contains("\"success\""), "Expected JSON body, got: " + resp.body());

      client.verifyRequestCount(1, "/session");
    }
  }

  @Test
  public void verifyRequestCountThrowsAssertionErrorOnMismatch() {
    try (WiremockClient client = new WiremockClient()) {
      client.start();
      assertThrows(AssertionError.class, () -> client.verifyRequestCount(1, "/never-called"));
    }
  }
}
