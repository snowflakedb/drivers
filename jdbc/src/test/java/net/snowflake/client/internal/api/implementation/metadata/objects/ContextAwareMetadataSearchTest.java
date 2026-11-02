package net.snowflake.client.internal.api.implementation.metadata.objects;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.regex.Pattern;
import net.snowflake.common.util.Wildcard;
import org.junit.jupiter.api.Test;

class ContextAwareMetadataSearchTest {

  @Test
  void shouldAcceptQuotedSchemaNameWhenExactSchemaSearchIsOn() {
    Pattern compiled = Wildcard.toRegexPattern("FOO%BAR", true);
    ContextAwareMetadataSearch search = search("FOO%BAR", true);

    assertTrue(search.schemaMatches(compiled, "\"FOO%BAR\""));
  }

  @Test
  void shouldAcceptQuotedSchemaNameThatContainsAQuoteWhenExactSchemaSearchIsOn() {
    Pattern compiled = Wildcard.toRegexPattern("FOO\"BAR", true);
    ContextAwareMetadataSearch search = search("FOO\"BAR", true);

    assertTrue(search.schemaMatches(compiled, "\"FOO\"\"BAR\""));
  }

  @Test
  void shouldRejectDifferentSchemaWhenExactSchemaSearchIsOn() {
    Pattern compiled = Wildcard.toRegexPattern("FOO%BAR", true);
    ContextAwareMetadataSearch search = search("FOO%BAR", true);

    assertFalse(search.schemaMatches(compiled, "OTHER"));
    assertFalse(search.schemaMatches(compiled, "\"OTHER\""));
  }

  @Test
  void shouldRejectSchemaThatDoesNotMatchPatternWhenExactSchemaSearchIsOff() {
    Pattern compiled = Wildcard.toRegexPattern("FOO", true);
    ContextAwareMetadataSearch search = search("FOO", false);

    assertFalse(search.schemaMatches(compiled, "BAR"));
  }

  @Test
  void shouldAcceptSchemaThatMatchesPatternWhenExactSchemaSearchIsOff() {
    Pattern compiled = Wildcard.toRegexPattern("FOO%", true);
    ContextAwareMetadataSearch search = search("FOO%", false);

    assertTrue(search.schemaMatches(compiled, "FOOBAR"));
  }

  @Test
  void shouldAcceptAnySchemaWhenCompiledPatternIsNull() {
    ContextAwareMetadataSearch search = search(null, false);

    assertTrue(search.schemaMatches(null, "ANY_SCHEMA"));
  }

  private static ContextAwareMetadataSearch search(String schema, boolean isExactSchema) {
    return new ContextAwareMetadataSearch(null, schema, isExactSchema, false, true);
  }
}
