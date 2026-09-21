package net.snowflake.jdbc.wiremock;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.util.List;
import java.util.Properties;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.jdbc.utils.JsonTestUtils;
import org.junit.jupiter.api.Test;

/**
 * Verifies that when {@code CLIENT_STAGE_ARRAY_BINDING_THRESHOLD} selects stage (CSV) binding and
 * the {@code SYSTEM$BIND} stage creation is rejected, {@code PreparedBatch} retries the same
 * statement with inline JSON bindings instead of failing outright.
 */
public class StageBindingDisabledRetryWiremockTest extends BaseWiremockTest {

  @Test
  public void shouldRetryWithInlineJsonBindingsWhenStageBindingIsDisabled() throws Exception {
    wiremock.addMapping("auth/login_success_low_stage_binding_threshold.json");
    wiremock.addMapping("query/describe_array_bind_supported.json");
    wiremock.addMapping("query/create_stage_binding_disabled.json");
    wiremock.addMapping("query/insert_success_after_stage_binding_retry.json");

    Properties props = new Properties();
    props.setProperty("account", "test_account");
    props.setProperty("user", "test_user");
    props.setProperty("password", "test_password");
    props.setProperty("protocol", "http");

    Class.forName(SnowflakeDriver.class.getName());
    try (Connection conn = DriverManager.getConnection(wiremockJdbcUrl(), props);
        PreparedStatement ps = conn.prepareStatement("INSERT INTO t (id) VALUES (?)")) {
      ps.setInt(1, 1);
      ps.addBatch();
      ps.executeBatch();
    }

    List<JsonNode> requests = wiremock.getRequests("/queries/v1/query-request.*");
    assertEquals(3, requests.size());

    int describeCount = 0;
    int createStageCount = 0;
    int retriedInsertCount = 0;
    for (JsonNode request : requests) {
      JsonNode body = JsonTestUtils.parseJson(request.get("body").asText());
      String sqlText = body.get("sqlText").asText();
      if (body.has("describeOnly")) {
        describeCount++;
      } else if (sqlText.contains("CREATE TEMPORARY STAGE")) {
        createStageCount++;
      } else {
        retriedInsertCount++;
        assertTrue(body.has("bindings"));
        assertFalse(body.has("bindStage"));
      }
    }
    assertEquals(1, describeCount);
    assertEquals(1, createStageCount);
    assertEquals(1, retriedInsertCount);
  }
}
