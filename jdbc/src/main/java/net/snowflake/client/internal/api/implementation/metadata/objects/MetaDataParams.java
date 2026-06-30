package net.snowflake.client.internal.api.implementation.metadata.objects;

import java.sql.SQLException;
import lombok.RequiredArgsConstructor;
import lombok.Value;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;
import net.snowflake.client.internal.util.NotImplementedException;

/**
 * Session-derived parameters that drive {@link java.sql.DatabaseMetaData} command construction,
 * ported from the legacy snowflake-jdbc driver's {@code SFBaseSession} flags.
 *
 * <p>The flags are read once from the session via {@link CoreDriverApi#connectionGetParameter}. The
 * core driver does not yet surface these parameters, so {@code connectionGetParameter} currently
 * returns no value and the legacy defaults apply &mdash; this class is effectively a stub until
 * core adds them, but it already uses the real RPC path so no call sites change when core catches
 * up.
 */
@RequiredArgsConstructor
class MetaDataParams {

  @Value
  static class ContextAwareMetadataSearch {
    String database;
    String schema;
    boolean isExactSchema;
    boolean useSessionSchema;
  }

  // Snowflake session parameter names (see SFSessionProperty / SessionUtil in snowflake-jdbc).
  private static final String CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX =
      "CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX";
  private static final String CLIENT_METADATA_USE_SESSION_DATABASE =
      "CLIENT_METADATA_USE_SESSION_DATABASE";
  private static final String ENABLE_EXACT_SCHEMA_SEARCH_ENABLED =
      "ENABLE_EXACT_SCHEMA_SEARCH_ENABLED";
  private static final String ENABLE_WILDCARDS_IN_SHOW_METADATA_COMMANDS =
      "ENABLE_WILDCARDS_IN_SHOW_METADATA_COMMANDS";

  // Connection property controlling whether pattern (wildcard) searches are allowed for
  // getPrimaryKeys/getImportedKeys/getExportedKeys/getCrossReference. Mirrors the legacy
  // SFSessionProperty.ENABLE_PATTERN_SEARCH property key.
  private static final String ENABLE_PATTERN_SEARCH = "enablePatternSearch";

  // Legacy defaults from SFBaseSession.
  private static final boolean DEFAULT_METADATA_REQUEST_USE_CONNECTION_CTX = false;
  private static final boolean DEFAULT_METADATA_REQUEST_USE_SESSION_DATABASE = false;
  private static final boolean DEFAULT_ENABLE_EXACT_SCHEMA_SEARCH = false;
  private static final boolean DEFAULT_ENABLE_WILDCARDS_IN_SHOW_METADATA_COMMANDS = true;
  private static final boolean DEFAULT_ENABLE_PATTERN_SEARCH = true;

  private final InternalSnowflakeConnection connection;
  private final CoreDriverApi coreDriverApi;

  private boolean metadataRequestUseConnectionCtx() throws SQLException {
    return readBooleanParameter(
        CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX, DEFAULT_METADATA_REQUEST_USE_CONNECTION_CTX);
  }

  private boolean metadataRequestUseSessionDatabase() throws SQLException {
    return readBooleanParameter(
        CLIENT_METADATA_USE_SESSION_DATABASE, DEFAULT_METADATA_REQUEST_USE_SESSION_DATABASE);
  }

  private boolean enableExactSchemaSearch() throws SQLException {
    return readBooleanParameter(
        ENABLE_EXACT_SCHEMA_SEARCH_ENABLED, DEFAULT_ENABLE_EXACT_SCHEMA_SEARCH);
  }

  boolean isEnableWildcardsInShowMetadataCommands() throws SQLException {
    return readBooleanParameter(
        ENABLE_WILDCARDS_IN_SHOW_METADATA_COMMANDS,
        DEFAULT_ENABLE_WILDCARDS_IN_SHOW_METADATA_COMMANDS);
  }

  boolean isEnablePatternSearch() throws SQLException {
    return readBooleanParameter(ENABLE_PATTERN_SEARCH, DEFAULT_ENABLE_PATTERN_SEARCH);
  }

  /**
   * Applies session context to a metadata search, mirroring the legacy driver. When the catalog
   * (and schema, under {@code CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX}) are unspecified, they
   * are filled from the session.
   *
   * @param catalog the requested catalog, may be {@code null}
   * @param schemaPattern the requested schema pattern, may be {@code null}
   * @return the resolved search context
   */
  ContextAwareMetadataSearch applySessionContext(String catalog, String schemaPattern)
      throws SQLException {
    boolean useSessionSchema = false;
    if (metadataRequestUseConnectionCtx()) {
      // CLIENT_METADATA_REQUEST_USE_CONNECTION_CTX = TRUE
      if (catalog == null) {
        catalog = sessionDatabase();
      }
      if (schemaPattern == null) {
        schemaPattern = sessionSchema();
        useSessionSchema = true;
      }
    } else if (metadataRequestUseSessionDatabase() && catalog == null) {
      catalog = sessionDatabase();
    }

    boolean isExactSchema =
        (enableExactSchemaSearch() && useSessionSchema)
            || !isEnableWildcardsInShowMetadataCommands();
    return new ContextAwareMetadataSearch(catalog, schemaPattern, isExactSchema, useSessionSchema);
  }

  // Session database/schema come from the connection. Until Connection#getCatalog/#getSchema are
  // implemented in the new driver, these throw NotImplementedException; treat that as "no session
  // context available" so metadata still works under the legacy (non-context) defaults.
  private String sessionDatabase() throws SQLException {
    try {
      return connection.getCatalog();
    } catch (NotImplementedException e) {
      return null;
    }
  }

  private String sessionSchema() throws SQLException {
    try {
      return connection.getSchema();
    } catch (NotImplementedException e) {
      return null;
    }
  }

  private boolean readBooleanParameter(String key, boolean defaultValue) throws SQLException {
    ConnectionGetParameterResponse response =
        coreDriverApi.connectionGetParameter(connection.getHandle(), key);
    if (response != null && response.hasValue()) {
      return Boolean.parseBoolean(response.getValue());
    }
    return defaultValue;
  }
}
