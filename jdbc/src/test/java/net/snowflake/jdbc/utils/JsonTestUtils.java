package net.snowflake.jdbc.utils;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.IOException;
import lombok.AccessLevel;
import lombok.NoArgsConstructor;

/**
 * Shared Jackson helpers for tests: a single {@link ObjectMapper} plus node-factory and parse
 * helpers, so test classes don't each declare their own mapper and {@code parseJson} boilerplate.
 */
@NoArgsConstructor(access = AccessLevel.PRIVATE)
public final class JsonTestUtils {

  private static final ObjectMapper MAPPER = new ObjectMapper();

  /** The shared mapper, for callers that need {@code readTree(InputStream)} or other APIs. */
  public static ObjectMapper mapper() {
    return MAPPER;
  }

  /** Parses {@code content} as JSON, wrapping the checked {@link IOException} unchecked. */
  public static JsonNode parseJson(String content) {
    try {
      return MAPPER.readTree(content);
    } catch (IOException e) {
      throw new RuntimeException("Failed to parse JSON: " + content, e);
    }
  }

  public static ObjectNode objectNode() {
    return MAPPER.createObjectNode();
  }

  public static ArrayNode arrayNode() {
    return MAPPER.createArrayNode();
  }
}
