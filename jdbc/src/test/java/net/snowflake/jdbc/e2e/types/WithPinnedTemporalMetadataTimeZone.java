package net.snowflake.jdbc.e2e.types;

import java.util.TimeZone;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;

/**
 * Pins the JVM default timezone so TIMESTAMP_LTZ/TZ metadata precision and display size stay stable
 * across machines. Matches {@code SnowflakeResultSetMetaDataImplTemporalTypesTest}.
 */
interface WithPinnedTemporalMetadataTimeZone {

  TimeZone METADATA_TIME_ZONE = TimeZone.getTimeZone("Europe/Warsaw");

  class Holder {
    private static TimeZone originalTimeZone;
  }

  @BeforeAll
  default void pinDefaultTimeZoneForTemporalMetadata() {
    Holder.originalTimeZone = TimeZone.getDefault();
    TimeZone.setDefault(METADATA_TIME_ZONE);
  }

  @AfterAll
  default void restoreDefaultTimeZoneAfterTemporalMetadata() {
    if (Holder.originalTimeZone != null) {
      TimeZone.setDefault(Holder.originalTimeZone);
    }
  }
}
