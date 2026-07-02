package net.snowflake.jdbc.e2e.parity;

import java.util.ArrayList;
import java.util.List;

/**
 * Multi-column bind query plus the (parameter to setSink, scale, value) layout. Each column ends up
 * holding the round-tripped form of a bound parameter. We compare the read-back string from both
 * drivers; the read-side sinks are already exercised by {@link ReadLayout}.
 */
final class WriteLayout {

  final String sql;
  final List<Cell> cells;

  private WriteLayout(String sql, List<Cell> cells) {
    this.sql = sql;
    this.cells = cells;
  }

  static WriteLayout build(
      SfType type, List<Integer> scales, List<String> values, List<SetSink> setSinks) {
    StringBuilder sb = new StringBuilder("SELECT ");
    List<Cell> cells = new ArrayList<>();
    int col = 1;
    boolean first = true;
    for (int v = 0; v < values.size(); v++) {
      String value = values.get(v);
      for (int s = 0; s < scales.size(); s++) {
        int scale = scales.get(s);
        for (int k = 0; k < setSinks.size(); k++) {
          SetSink sink = setSinks.get(k);
          if (!first) {
            sb.append(", ");
          }
          first = false;
          sb.append("?::").append(type.castSpec(scale)).append(" AS c").append(col);
          cells.add(new Cell(col, col, scale, value, sink));
          col++;
        }
      }
    }
    return new WriteLayout(sb.toString(), cells);
  }

  static final class Cell {
    final int paramIdx;
    final int columnIdx;
    final int scale;
    final String value;
    final SetSink sink;

    Cell(int paramIdx, int columnIdx, int scale, String value, SetSink sink) {
      this.paramIdx = paramIdx;
      this.columnIdx = columnIdx;
      this.scale = scale;
      this.value = value;
      this.sink = sink;
    }
  }
}
