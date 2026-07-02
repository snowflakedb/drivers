package net.snowflake.client.internal.api.implementation.resultset.metadata;

import java.sql.Types;
import net.snowflake.client.api.resultset.SnowflakeType;

/**
 * Maps an internal Snowflake column type to the JDBC type exposed to callers.
 *
 * <p>Ported from {@code net.snowflake.client.internal.core.ColumnTypeHelper} in the legacy driver.
 * The legacy version took an {@code SFBaseSession} to read {@code
 * getEnableReturnTimestampWithTimeZone()}; here that flag is passed directly as a boolean, sourced
 * from connection parameters by the caller.
 */
class ColumnTypeHelper {

  /**
   * Translates an internal column type into the external JDBC type.
   *
   * @param internalColumnType the internal Snowflake column type (see {@link SnowflakeType} extra
   *     type codes)
   * @param enableReturnTimestampWithTimeZone whether TIMESTAMP_TZ should be reported as {@link
   *     Types#TIMESTAMP_WITH_TIMEZONE} (otherwise {@link Types#TIMESTAMP})
   * @return the external JDBC type
   */
  static int getColumnType(int internalColumnType, boolean enableReturnTimestampWithTimeZone) {
    int externalColumnType = internalColumnType;

    if (internalColumnType == SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ) {
      externalColumnType = Types.TIMESTAMP;
    } else if (internalColumnType == SnowflakeType.EXTRA_TYPES_TIMESTAMP_TZ) {
      externalColumnType =
          enableReturnTimestampWithTimeZone ? Types.TIMESTAMP_WITH_TIMEZONE : Types.TIMESTAMP;
    }
    return externalColumnType;
  }
}
