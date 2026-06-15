package net.snowflake.jdbc.e2e.parity;

/** Snowflake date/time types covered by the parity matrix. */
public enum SfType {
  DATE("DATE_OUTPUT_FORMAT", false),
  TIME("TIME_OUTPUT_FORMAT", true),
  TIMESTAMP_NTZ("TIMESTAMP_NTZ_OUTPUT_FORMAT", true),
  TIMESTAMP_LTZ("TIMESTAMP_LTZ_OUTPUT_FORMAT", true),
  TIMESTAMP_TZ("TIMESTAMP_TZ_OUTPUT_FORMAT", true);

  private final String outputFormatParam;
  private final boolean scaled;

  SfType(String outputFormatParam, boolean scaled) {
    this.outputFormatParam = outputFormatParam;
    this.scaled = scaled;
  }

  public String outputFormatParam() {
    return outputFormatParam;
  }

  public boolean isScaled() {
    return scaled;
  }

  /** SQL cast spec at a given scale, e.g. {@code TIME(6)} or {@code TIMESTAMP_NTZ(9)}. */
  public String castSpec(int scale) {
    return scaled ? name() + "(" + scale + ")" : name();
  }
}
