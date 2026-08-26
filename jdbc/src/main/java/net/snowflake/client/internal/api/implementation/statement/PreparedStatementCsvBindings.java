package net.snowflake.client.internal.api.implementation.statement;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import net.snowflake.client.internal.api.implementation.statement.PreparedStatementBindingSerializer.NativeBindings;
import net.snowflake.client.internal.api.implementation.statement.PreparedStatementBindingSerializer.ParameterValue;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.QueryBindings;

/**
 * Serializes a column-major batch to the CSV format the SYSTEM$BIND stage expects, mirroring legacy
 * {@code snowflake-jdbc}'s {@code BindUploader}/{@code SnowflakeTypeUtil}. Core creates the stage,
 * PUTs the bytes, and rewrites the request to reference the stage path.
 */
final class PreparedStatementCsvBindings {

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
    List<List<String>> orderedColumns = new ArrayList<>(columns.size());
    for (ParameterValue column : new TreeMap<>(columns).values()) {
      @SuppressWarnings("unchecked")
      List<String> values = (List<String>) column.value();
      orderedColumns.add(values);
    }
    StringBuilder sb = new StringBuilder();
    for (int row = 0; row < rowCount; row++) {
      for (int col = 0; col < orderedColumns.size(); col++) {
        if (col > 0) {
          sb.append(',');
        }
        sb.append(escapeForCsv(orderedColumns.get(col).get(row)));
      }
      sb.append('\n'); // every row, including the last, is terminated
    }
    return sb.toString().getBytes(StandardCharsets.UTF_8);
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
