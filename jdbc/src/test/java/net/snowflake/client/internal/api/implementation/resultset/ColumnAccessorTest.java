package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.Mockito.CALLS_REAL_METHODS;
import static org.mockito.Mockito.doReturn;
import static org.mockito.Mockito.mock;

import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import org.junit.jupiter.api.Test;

class ColumnAccessorTest {

  @Test
  void shouldFindColumnByLabelIgnoringCase() {
    ColumnAccessor accessor = twoColumnAccessor();
    doReturn("second-value").when(accessor).getString(2);

    assertEquals(1, accessor.findColumn("first_col"));
    assertEquals(2, accessor.findColumn("SECOND_COL"));
    assertEquals("second-value", accessor.getString("second_col"));
  }

  @Test
  void shouldRejectUnknownColumnLabel() {
    IllegalArgumentException thrown =
        assertThrows(
            IllegalArgumentException.class, () -> twoColumnAccessor().findColumn("missing_col"));

    assertEquals("Column not found: missing_col", thrown.getMessage());
  }

  @Test
  void shouldReportIntervalAndListAccessorsAsUnsupportedByDefault() {
    ColumnAccessor accessor = twoColumnAccessor();

    assertEquals(
        "getPeriod is not supported for this result set",
        assertThrows(SFSQLFeatureNotSupportedException.class, () -> accessor.getPeriod(1))
            .getMessage());
    assertEquals(
        "getDuration is not supported for this result set",
        assertThrows(SFSQLFeatureNotSupportedException.class, () -> accessor.getDuration(1))
            .getMessage());
    assertEquals(
        "getList is not supported for this result set",
        assertThrows(SFSQLFeatureNotSupportedException.class, () -> accessor.getList(1))
            .getMessage());
  }

  private static ColumnAccessor twoColumnAccessor() {
    ColumnAccessor accessor = mock(ColumnAccessor.class, CALLS_REAL_METHODS);
    doReturn(2).when(accessor).getColumnCount();
    doReturn("FIRST_COL").when(accessor).getColumnName(1);
    doReturn("second_col").when(accessor).getColumnName(2);
    return accessor;
  }
}
