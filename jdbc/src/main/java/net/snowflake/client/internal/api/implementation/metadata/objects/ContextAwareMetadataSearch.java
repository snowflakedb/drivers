package net.snowflake.client.internal.api.implementation.metadata.objects;

import static net.snowflake.client.internal.api.implementation.metadata.objects.MatchingUtils.matches;

import java.util.regex.Pattern;
import lombok.Value;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.parameters.Parameter;
import net.snowflake.client.internal.api.implementation.parameters.ParametersRegistry;

@Value
class ContextAwareMetadataSearch {

  String database;
  String schema;
  boolean isExactSchema;
  boolean useSessionSchema;
  boolean enableWildcards;

  static ContextAwareMetadataSearch fromSession(
      InternalSnowflakeConnection connection, String catalog, String schemaPattern) {
    ParametersRegistry params = connection.getParameters();
    boolean useSessionSchema = false;
    if (params.getBool(Parameter.CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX)) {
      if (catalog == null) {
        catalog = connection.getCatalog();
      }
      if (schemaPattern == null) {
        schemaPattern = connection.getSchema();
        useSessionSchema = true;
      }
    } else if (params.getBool(Parameter.CLIENT_METADATA_USE_SESSION_DATABASE) && catalog == null) {
      catalog = connection.getCatalog();
    }

    boolean enableWildcards = params.getBool(Parameter.ENABLE_WILDCARDS_IN_SHOW_METADATA_COMMANDS);
    boolean isExactSchema =
        (params.getBool(Parameter.ENABLE_EXACT_SCHEMA_SEARCH_ENABLED) && useSessionSchema)
            || !enableWildcards;
    return new ContextAwareMetadataSearch(
        catalog, schemaPattern, isExactSchema, useSessionSchema, enableWildcards);
  }

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
