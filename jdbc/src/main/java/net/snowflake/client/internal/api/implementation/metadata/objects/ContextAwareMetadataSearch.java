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

  // TODO(SNOW-3740734): I think that the schemaMatchesExactly branch is redundant in practice.
  //  The compiledSchemaPattern is built by Wildcard.toRegexPattern which treats '_'
  //  as a single-char wildcard (regex '.'), so matches() already accepts every schema name
  //  that schemaMatchesExactly would. The exact-match branch can never widen the result set beyond
  //  what the wildcard match already returns.
  //  Keeping for parity with the legacy driver's intent.
  boolean schemaMatches(Pattern compiledSchemaPattern, String schemaName) {
    return matches(compiledSchemaPattern, schemaName) || schemaMatchesExactly(schemaName);
  }

  boolean schemaMatchesExactly(String schemaName) {
    return isExactSchema && schema.equals(schemaName);
  }
}
