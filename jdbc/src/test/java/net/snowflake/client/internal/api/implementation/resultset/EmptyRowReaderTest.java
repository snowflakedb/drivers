package net.snowflake.client.internal.api.implementation.resultset;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.sql.SQLException;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class EmptyRowReaderTest {

  private EmptyRowReader reader;

  @BeforeEach
  void setUp() {
    reader = new EmptyRowReader(new String[] {"TABLE_SCHEM", "TABLE_CATALOG"});
  }

  @Test
  void shouldExposeColumnMetadataWithoutAdvancingCursor() throws SQLException {
    assertEquals(2, reader.getColumnCount());
    assertEquals("TABLE_SCHEM", reader.getColumnName(1));
    assertEquals("TABLE_CATALOG", reader.getColumnName(2));
  }

  @Test
  void shouldStartBeforeFirstAndEndAfterLast() throws SQLException {
    assertTrue(reader.isBeforeFirst());
    assertFalse(reader.isAfterLast());
    assertFalse(reader.isFirst());
    assertEquals(-1, reader.getCurrentRow());

    assertFalse(reader.next());
    assertFalse(reader.isBeforeFirst());
    assertTrue(reader.isAfterLast());
    assertFalse(reader.isFirst());
    assertEquals(-1, reader.getCurrentRow());
  }

  @Test
  void shouldRejectColumnAccessWithoutCurrentRow() {
    assertThrows(SQLException.class, () -> reader.getString(1));
  }

  @Test
  void shouldMarkClosedAfterClose() throws SQLException {
    reader.close();
    assertTrue(reader.isClosed());
  }
}
