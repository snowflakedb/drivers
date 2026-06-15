package net.snowflake.jdbc.e2e.parity;

import java.util.ArrayList;
import java.util.List;

/**
 * Multi-column read query plus the (column to scale, value) layout describing what each column
 * carries. Built once per (type, format, tz) and used to drive a single round-trip per driver.
 */
final class ReadLayout {

  final String sql;
  final List<Cell> cells;

  private ReadLayout(String sql, List<Cell> cells) {
    this.sql = sql;
    this.cells = cells;
  }

  static ReadLayout build(SfType type, List<Integer> scales, List<String> values) {
    StringBuilder sb = new StringBuilder("SELECT ");
    List<Cell> cells = new ArrayList<>();
    int col = 1;
    boolean first = true;
    for (int v = 0; v < values.size(); v++) {
      String value = values.get(v);
      for (int s = 0; s < scales.size(); s++) {
        int scale = scales.get(s);
        if (!first) {
          sb.append(", ");
        }
        first = false;
        sb.append("'")
            .append(escape(value))
            .append("'::")
            .append(type.castSpec(scale))
            .append(" AS c")
            .append(col);
        cells.add(new Cell(col, scale, value));
        col++;
      }
    }
    return new ReadLayout(sb.toString(), cells);
  }

  private static String escape(String s) {
    return s.replace("'", "''");
  }

  static final class Cell {
    final int columnIdx;
    final int scale;
    final String value;

    Cell(int columnIdx, int scale, String value) {
      this.columnIdx = columnIdx;
      this.scale = scale;
      this.value = value;
    }
  }
}
