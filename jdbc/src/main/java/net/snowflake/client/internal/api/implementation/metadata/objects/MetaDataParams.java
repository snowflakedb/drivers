package net.snowflake.client.internal.api.implementation.metadata.objects;

import java.sql.SQLException;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.unicore.CoreDriverApi;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionGetParameterResponse;

/**
 * Session- and connection-derived parameters that drive {@link java.sql.DatabaseMetaData} command
 * construction and result shaping, ported from the legacy snowflake-jdbc driver's {@code
 * SFBaseSession} flags.
 *
 * <p>Session flags are read via {@link CoreDriverApi#connectionGetParameter}. The core driver does
 * not yet surface these parameters, so {@code connectionGetParameter} currently returns no value
 * and the legacy defaults apply &mdash; this class is effectively a stub until core adds them, but
 * it already uses the real RPC path so no call sites change when core catches up.
 */
class MetaDataParams {

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

  // Session parameter (SFSessionProperty.STRINGS_QUOTED in snowflake-jdbc).
  private static final String STRINGS_QUOTED_FOR_COLUMN_DEF = "STRINGS_QUOTED_FOR_COLUMN_DEF";

  // Session parameter (see SFBaseSession#isJdbcTreatDecimalAsInt in snowflake-jdbc).
  private static final String JDBC_TREAT_DECIMAL_AS_INT = "JDBC_TREAT_DECIMAL_AS_INT";

  // Legacy defaults from SFBaseSession.
  private static final boolean DEFAULT_METADATA_REQUEST_USE_CONNECTION_CTX = false;
  private static final boolean DEFAULT_METADATA_REQUEST_USE_SESSION_DATABASE = false;
  private static final boolean DEFAULT_ENABLE_EXACT_SCHEMA_SEARCH = false;
  private static final boolean DEFAULT_ENABLE_WILDCARDS_IN_SHOW_METADATA_COMMANDS = true;
  private static final boolean DEFAULT_ENABLE_PATTERN_SEARCH = true;
  private static final boolean DEFAULT_STRINGS_QUOTED = false;
  private static final boolean DEFAULT_JDBC_TREAT_DECIMAL_AS_INT = true;
  private static final boolean DEFAULT_ENABLE_RETURN_TIMESTAMP_WITH_TIMEZONE = true;

  private final InternalSnowflakeConnection connection;
  private final CoreDriverApi coreDriverApi;

  MetaDataParams(InternalSnowflakeConnection connection, CoreDriverApi coreDriverApi) {
    this.connection = connection;
    this.coreDriverApi = coreDriverApi;
  }

  boolean isStringsQuoted() throws SQLException {
    return readBooleanParameter(STRINGS_QUOTED_FOR_COLUMN_DEF, DEFAULT_STRINGS_QUOTED);
  }

  boolean isJdbcTreatDecimalAsInt() throws SQLException {
    return readBooleanParameter(JDBC_TREAT_DECIMAL_AS_INT, DEFAULT_JDBC_TREAT_DECIMAL_AS_INT);
  }

  boolean isEnableReturnTimestampWithTimeZone() throws SQLException {
    return readBooleanParameter(
        "ENABLE_RETURN_TIMESTAMP_WITH_TIMEZONE", DEFAULT_ENABLE_RETURN_TIMESTAMP_WITH_TIMEZONE);
  }

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
        catalog = connection.getCatalog();
      }
      if (schemaPattern == null) {
        schemaPattern = connection.getSchema();
        useSessionSchema = true;
      }
    } else if (metadataRequestUseSessionDatabase() && catalog == null) {
      catalog = connection.getCatalog();
    }

    // The second arm (!enableWildcards) forces isExactSchema=true even when the caller provided
    // an explicit schema pattern. This is copied from the legacy driver and is effectively a no-op:
    // - likeSchema() only escapes wildcards when isExactSchema && enableWildcards (both true),
    //   so when wildcards are disabled the escaping branch is skipped anyway.
    // - isSchemaNameWildcardPattern() independently returns false when wildcards are disabled,
    //   so the IN-clause scoping doesn't depend on isExactSchema either.
    // - schemaMatches() checks matches(compiledPattern, ...) first, which uses the same wildcard
    //   semantics as SHOW's LIKE, so schemaMatchesExactly never widens the result set.
    boolean isExactSchema =
        (enableExactSchemaSearch() && useSessionSchema)
            || !isEnableWildcardsInShowMetadataCommands();
    return new ContextAwareMetadataSearch(catalog, schemaPattern, isExactSchema, useSessionSchema);
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
