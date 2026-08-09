package net.snowflake.client.internal.api.implementation.telemetry;

import lombok.Getter;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.util.NotImplementedException;

/**
 * Pre-classified category of a wrapper-caught error, sent to core as the {@code error_source} wire
 * string of {@code telemetrySendWrapperError}. The snake_case {@link #getWireValue()} strings are a
 * contract shared with the Python and ODBC front-ends, so they must stay stable and identical
 * across drivers; the raw error itself never crosses the wire.
 */
@Getter
@RequiredArgsConstructor
public enum ErrorSource {
  CONNECTIVITY("connectivity"),
  SERVER_ERROR("server_error"),
  DATA_CONVERSION("data_conversion"),
  CURSOR_STATE("cursor_state"),
  API_MISUSE("api_misuse"),
  CONFIG_PARSING("config_parsing"),
  INTERNAL_ERROR("internal_error"),
  UNSUPPORTED("unsupported"),
  UNKNOWN("unknown");

  /** The snake_case string sent on the wire (e.g. {@code "server_error"}). */
  private final String wireValue;

  /**
   * Classifies a wrapper-caught throwable into its {@code error_source} category. Deliberately a
   * defensible subset, refined later without changing callers.
   */
  public static ErrorSource of(Throwable t) {
    if (t instanceof SFSQLException || t instanceof SnowflakeSQLException) {
      return SERVER_ERROR;
    }
    if (t instanceof NotImplementedException) {
      return UNSUPPORTED;
    }
    // TODO: classify into the remaining categories (CONNECTIVITY, DATA_CONVERSION, CURSOR_STATE,
    //  API_MISUSE, CONFIG_PARSING) as the impl migration surfaces the throwable types that map to
    //  them; until then everything unrecognized falls through to INTERNAL_ERROR.
    return INTERNAL_ERROR;
  }
}
