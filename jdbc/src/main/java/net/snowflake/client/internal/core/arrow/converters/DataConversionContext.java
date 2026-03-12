package net.snowflake.client.internal.core.arrow.converters;

public interface DataConversionContext {
  // TODO: Populate from the session's DATE_OUTPUT_FORMAT parameter.
  // TODO: Once populated, implement Snowflake-to-Java format translation before use in
  //  DateConverter (Snowflake "YYYY-MM-DD" != Java "yyyy-MM-dd").
  /** Returns the date output format as a Java DateTimeFormatter pattern. */
  default String getDateOutputFormat() {
    return "yyyy-MM-dd";
  }
}
