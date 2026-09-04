package net.snowflake.client.internal.api.implementation.statement;

import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.time.ZoneId;
import java.time.ZoneOffset;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeFormatterBuilder;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.statement.PreparedStatementBindingSerializer.NativeBindings;
import net.snowflake.client.internal.api.implementation.statement.PreparedStatementBindingSerializer.ParameterValue;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;

/**
 * Serializes a column-major batch to the CSV format the SYSTEM$BIND stage expects, mirroring legacy
 * {@code snowflake-jdbc}'s {@code BindUploader}/{@code SnowflakeTypeUtil}. Core creates the stage,
 * PUTs the bytes, and rewrites the request to reference the stage path.
 */
final class PreparedStatementCsvBindings {
  private static final BigInteger NANOS_PER_SECOND = BigInteger.valueOf(1_000_000_000L);
  private static final DateTimeFormatter TIMESTAMP_FORMATTER =
      new DateTimeFormatterBuilder()
          .appendPattern("yyyy-MM-dd HH:mm:ss.SSSSSSSSS ")
          .appendOffset("+HH:MM", "Z")
          .toFormatter();

  private PreparedStatementCsvBindings() {}

  static NativeBindings serialize(Map<Integer, ParameterValue> columns, int rowCount) {
    if (columns.isEmpty() || rowCount == 0) {
      return PreparedStatementBindingSerializer.emptyBindings();
    }
    byte[] csv = buildCsv(columns, rowCount);
    return PreparedStatementBindingSerializer.allocateNativeBindings(
        csv, ptr -> QueryBindings.newBuilder().setCsv(ptr).build());
  }

  /**
   * Order columns by parameter index, transpose to row-major, join cells with {@code ','}.
   *
   * <p>Package-private so unit tests can assert the exact bytes without reaching into native
   * memory.
   */
  static byte[] buildCsv(Map<Integer, ParameterValue> columns, int rowCount) {
    List<ParameterValue> orderedColumns = new ArrayList<>(columns.size());
    for (ParameterValue column : new TreeMap<>(columns).values()) {
      orderedColumns.add(column);
    }
    StringBuilder sb = new StringBuilder();
    for (int row = 0; row < rowCount; row++) {
      for (int col = 0; col < orderedColumns.size(); col++) {
        if (col > 0) {
          sb.append(',');
        }
        ParameterValue column = orderedColumns.get(col);
        @SuppressWarnings("unchecked")
        List<String> values = (List<String>) column.value();
        sb.append(
            escapeForCsv(
                formatStageValue(column.bindType(), values.get(row), ZoneId.systemDefault())));
      }
      sb.append('\n'); // every row, including the last, is terminated
    }
    return sb.toString().getBytes(StandardCharsets.UTF_8);
  }

  /**
   * SYSTEM$BIND reads temporal CSV fields as formatted values, while inline bindings read their
   * wire-unit representation. Convert only the types transformed by legacy BindUploader.
   */
  static String formatStageValue(SnowflakeType type, String value, ZoneId localZone) {
    if (value == null) {
      return null;
    }
    if (type == SnowflakeType.DATE) {
      return Instant.ofEpochMilli(Long.parseLong(value))
          .atZone(ZoneOffset.UTC)
          .toLocalDate()
          .format(DateTimeFormatter.ISO_LOCAL_DATE);
    }
    if (type != SnowflakeType.TIMESTAMP_LTZ && type != SnowflakeType.TIMESTAMP_NTZ) {
      return value;
    }

    BigInteger[] secondsAndNanos = new BigInteger(value).divideAndRemainder(NANOS_PER_SECOND);
    long seconds = secondsAndNanos[0].longValueExact();
    int nanos = secondsAndNanos[1].intValueExact();
    if (nanos < 0) {
      seconds--;
      nanos += NANOS_PER_SECOND.intValueExact();
    }
    ZoneId zone = type == SnowflakeType.TIMESTAMP_LTZ ? localZone : ZoneOffset.UTC;
    return Instant.ofEpochSecond(seconds, nanos).atZone(zone).format(TIMESTAMP_FORMATTER);
  }

  /**
   * Quotes every non-null cell, matching ODBC's {@code append_escaped_csv_cell} rather than legacy
   * {@code SnowflakeTypeUtil.escapeForCSV} (which quotes only cells containing {@code "}, {@code
   * \n}, {@code ,} or {@code \\} and leaves everything else bare). The SYSTEM$BIND stage is created
   * with {@code escape_unenclosed_field=NONE}, so a bare cell containing a lone {@code \r} — which
   * legacy does not quote — would reach the server as a raw record delimiter. Quoting
   * unconditionally is the canonical form shared across the driver family and stays correct
   * regardless of future file-format tweaks. {@code null} stays an unquoted empty field so the
   * server reads it as SQL NULL.
   */
  private static String escapeForCsv(String value) {
    if (value == null) {
      return "";
    }
    return '"' + value.replace("\"", "\"\"") + '"';
  }
}
