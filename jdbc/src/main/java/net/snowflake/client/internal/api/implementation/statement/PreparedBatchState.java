package net.snowflake.client.internal.api.implementation.statement;

import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;
import net.snowflake.client.internal.log.SFLogger;
import net.snowflake.client.internal.log.SFLoggerFactory;

final class PreparedBatchState {
  private static final SFLogger logger = SFLoggerFactory.getLogger(PreparedBatchState.class);
  private final List<PreparedStatementBinding[]> rows = new ArrayList<PreparedStatementBinding[]>();
  private final ColumnBatchState[] columnStates;

  PreparedBatchState(int parameterCount) {
    this.columnStates = new ColumnBatchState[parameterCount];
    for (int i = 0; i < parameterCount; i++) {
      columnStates[i] = new ColumnBatchState();
    }
  }

  void appendSnapshot(PreparedStatementBinding[] nextSnapshot) throws SQLException {
    int nextRowNumber = rows.size() + 1;
    for (int col = 0; col < nextSnapshot.length; col++) {
      columnStates[col].observe(nextSnapshot[col], col + 1, nextRowNumber);
    }
    rows.add(nextSnapshot);
    logger.debug(
        "Prepared batch snapshot appended: rowNumber={}, batchSize={}, parameterCount={}",
        nextRowNumber,
        rows.size(),
        nextSnapshot.length);
  }

  PreparedStatementBinding[] toArrayBoundColumns() throws SQLException {
    PreparedStatementBinding[] arrayBoundValues = new PreparedStatementBinding[columnStates.length];
    @SuppressWarnings("unchecked")
    List<String>[] valueColumns = new ArrayList[columnStates.length];
    for (int i = 0; i < columnStates.length; i++) {
      valueColumns[i] = new ArrayList<String>(rows.size());
    }

    for (int rowIndex = 0; rowIndex < rows.size(); rowIndex++) {
      PreparedStatementBinding[] rowValues = rows.get(rowIndex);
      for (int col = 0; col < rowValues.length; col++) {
        PreparedStatementBinding rowValue = rowValues[col];
        if (rowValue == null) {
          throw missingBatchParameterException(col + 1, rowIndex + 1);
        }

        ColumnBatchState columnState = columnStates[col];
        if (!rowValue.isNull() && !columnState.matches(rowValue.bindType())) {
          throw mixedTypeException(
              col + 1,
              columnState.resolvedType(),
              columnState.resolvedTypeFirstRow(),
              rowIndex + 1,
              rowValue.bindType());
        }

        valueColumns[col].add(rowValue.scalarValue());
      }
    }

    for (int col = 0; col < columnStates.length; col++) {
      arrayBoundValues[col] =
          PreparedStatementBinding.arrayColumn(
              columnStates[col].resolvedTypeOrAny(), valueColumns[col]);
    }
    return arrayBoundValues;
  }

  PreparedStatementBinding[] rowAt(int rowIndex) {
    return rows.get(rowIndex);
  }

  int batchSize() {
    return rows.size();
  }

  boolean isEmpty() {
    return rows.isEmpty();
  }

  void reset() {
    int clearedRows = rows.size();
    rows.clear();
    for (ColumnBatchState columnState : columnStates) {
      columnState.reset();
    }
    logger.debug(
        "Prepared batch state reset: clearedRows={}, parameterCount={}",
        clearedRows,
        columnStates.length);
  }

  private static boolean isAnyType(String bindType) {
    return bindType == null || PreparedStatementBinding.ANY_BIND_TYPE.equalsIgnoreCase(bindType);
  }

  private static SQLException mixedTypeException(
      int parameterIndex,
      String existingType,
      int existingRow,
      int incomingRow,
      String incomingType) {
    return new SQLException(
        "Mixed parameter types for index "
            + parameterIndex
            + " between row "
            + existingRow
            + " type "
            + existingType
            + " and row "
            + incomingRow
            + " type "
            + incomingType);
  }

  private static SQLException missingBatchParameterException(int parameterIndex, int rowNumber) {
    return new SQLException(
        "Missing value for parameter index " + parameterIndex + " in batch row " + rowNumber);
  }

  private static final class ColumnBatchState {
    private String resolvedType;
    private int resolvedTypeFirstRow = -1;

    private void observe(PreparedStatementBinding value, int parameterIndex, int rowNumber)
        throws SQLException {
      if (value == null || value.isNull() || isAnyType(value.bindType())) {
        return;
      }
      if (resolvedType == null || isAnyType(resolvedType)) {
        resolvedType = value.bindType();
        resolvedTypeFirstRow = rowNumber;
        return;
      }
      if (!resolvedType.equalsIgnoreCase(value.bindType())) {
        throw mixedTypeException(
            parameterIndex, resolvedType, resolvedTypeFirstRow, rowNumber, value.bindType());
      }
    }

    private boolean matches(String incomingType) {
      return resolvedType == null
          || isAnyType(incomingType)
          || resolvedType.equalsIgnoreCase(incomingType);
    }

    private String resolvedType() {
      return resolvedType;
    }

    private int resolvedTypeFirstRow() {
      return resolvedTypeFirstRow;
    }

    private String resolvedTypeOrAny() {
      return resolvedType == null ? PreparedStatementBinding.ANY_BIND_TYPE : resolvedType;
    }

    private void reset() {
      resolvedType = null;
      resolvedTypeFirstRow = -1;
    }
  }
}
