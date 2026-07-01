package net.snowflake.client.internal.api.implementation.metadata.objects;

/**
 * Tracks {@code ORDINAL_POSITION} while converting {@code SHOW COLUMNS} rows into {@code
 * getColumns} result rows. Resets to 1 when the table name changes.
 */
class ColumnOrdinalTracker {

  private String currentTableName;
  private int ordinalPosition;

  int nextOrdinalFor(String tableName) {
    if (!tableName.equals(currentTableName)) {
      currentTableName = tableName;
      ordinalPosition = 1;
    } else {
      ordinalPosition++;
    }
    return ordinalPosition;
  }
}
