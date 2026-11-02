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

  // SHOW can return a quoted identifier, such as "FOO%BAR" for schema FOO%BAR.
  // A null catalog makes getProcedures and getFunctions run SHOW in account, which
  // is not limited to that schema.
  boolean schemaMatches(Pattern compiledSchemaPattern, String schemaName) {
    return matches(compiledSchemaPattern, schemaName) || schemaMatchesExactly(schemaName);
  }

  private boolean schemaMatchesExactly(String schemaName) {
    return isExactSchema && schema != null && schema.equals(unquoteIdentifier(schemaName));
  }

  private static String unquoteIdentifier(String identifier) {
    if (identifier == null
        || identifier.length() < 2
        || identifier.charAt(0) != '"'
        || identifier.charAt(identifier.length() - 1) != '"') {
      return identifier;
    }
    return identifier.substring(1, identifier.length() - 1).replace("\"\"", "\"");
  }
}
