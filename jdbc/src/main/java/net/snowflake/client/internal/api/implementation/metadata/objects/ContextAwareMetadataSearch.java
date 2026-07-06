package net.snowflake.client.internal.api.implementation.metadata.objects;

import static net.snowflake.client.internal.api.implementation.metadata.objects.MatchingUtils.matches;

import java.util.regex.Pattern;
import lombok.Value;

@Value
class ContextAwareMetadataSearch {
  String database;
  String schema;
  boolean isExactSchema;
  boolean useSessionSchema;

  boolean schemaMatches(Pattern compiledSchemaPattern, String schemaName) {
    return matches(compiledSchemaPattern, schemaName) || schemaMatchesExactly(schemaName);
  }

  boolean schemaMatchesExactly(String schemaName) {
    return isExactSchema && schema.equals(schemaName);
  }
}
