package net.snowflake.client.jdbc;

public enum SnowflakeType {
  ANY,
  ARRAY,
  BINARY,
  BOOLEAN,
  CHAR,
  DATE,
  DECFLOAT,
  FIXED,
  INTEGER,
  OBJECT,
  MAP,
  REAL,
  TEXT,
  TIME,
  TIMESTAMP,
  TIMESTAMP_LTZ,
  TIMESTAMP_NTZ,
  TIMESTAMP_TZ,
  INTERVAL_YEAR_MONTH,
  INTERVAL_DAY_TIME,
  VARIANT,
  GEOGRAPHY,
  GEOMETRY,
  VECTOR;

  public static SnowflakeType fromString(String name) {
    return SnowflakeType.valueOf(name.toUpperCase());
  }
}
