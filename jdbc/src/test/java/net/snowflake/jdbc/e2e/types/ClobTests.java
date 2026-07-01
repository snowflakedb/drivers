package net.snowflake.jdbc.e2e.types;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.OutputStream;
import java.io.Reader;
import java.io.Writer;
import java.sql.Clob;
import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import net.snowflake.jdbc.utils.SkipNewDriver;
import net.snowflake.jdbc.utils.SkipOldDriver;
import net.snowflake.jdbc.utils.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.Test;

/** Clob create/bind coverage for both universal and legacy JDBC reference runs. */
public class ClobTests extends SnowflakeIntegrationTestBase {

  @Test
  void shouldCreateEmptyClobAndBuildContentFromPositionOne() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When an empty Clob is created and content is written from position 1
    Clob clob = connection.createClob();
    assertEquals(0, clob.length());
    clob.setString(1, "pooling value");

    // Then the Clob round-trips the written value
    assertEquals("pooling value", readClobContent(clob));
  }

  private static String readClobContent(Clob clob) throws Exception {
    try (Reader reader = clob.getCharacterStream()) {
      StringBuilder content = new StringBuilder();
      char[] buffer = new char[4096];
      int read;
      while ((read = reader.read(buffer)) != -1) {
        content.append(buffer, 0, read);
      }
      return content.toString();
    }
  }

  @Test
  void shouldBindNullEmptyAndNonEmptyClobViaPreparedStatement() throws Exception {
    // Given Snowflake client is logged in and a temp table exists
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_clob_bind_", "txt STRING");

    // When empty, non-empty, and null Clobs are bound and inserted
    try (PreparedStatement insert =
        connection.prepareStatement("INSERT INTO " + tableName + " (txt) VALUES (?)")) {
      Clob empty = connection.createClob();
      insert.setClob(1, empty);
      insert.executeUpdate();

      Clob value = connection.createClob();
      value.setString(1, "bound clob");
      insert.setClob(1, value);
      insert.executeUpdate();

      insert.setClob(1, (Clob) null);
      insert.executeUpdate();
    }

    // Then the stored rows match the bound values in insert order
    try (PreparedStatement select =
            connection.prepareStatement(
                "SELECT txt FROM " + tableName + " ORDER BY txt NULLS LAST");
        ResultSet rs = select.executeQuery()) {
      assertTrue(rs.next());
      assertEquals("", rs.getString(1));
      assertTrue(rs.next());
      assertEquals("bound clob", rs.getString(1));
      assertTrue(rs.next());
      assertNull(rs.getString(1));
      assertTrue(rs.wasNull());
      assertFalse(rs.next());
    }
  }

  @Test
  @SkipOldDriver("BD#16")
  void shouldReturnOneBasedPositionIndexFromPosition() throws Exception {
    // Given Snowflake client is logged in and a Clob contains a search target
    Connection connection = getDefaultConnection();
    Clob clob = connection.createClob();
    clob.setString(1, "hello world");

    // When position is queried from the start
    long index = clob.position("world", 1);

    // Then the index is 1-based per JDBC
    assertEquals(7, index);
  }

  @Test
  @SkipNewDriver("BD#16")
  void shouldReturnZeroBasedPositionIndexOnLegacyDriver() throws Exception {
    // Given Snowflake client is logged in and a Clob contains a search target
    Connection connection = getDefaultConnection();
    Clob clob = connection.createClob();
    clob.setString(1, "hello world");

    // When position is queried from the start on the legacy driver
    long index = clob.position("world", 7);

    // Then the legacy driver returns a 0-based index when a match is found
    assertEquals(6, index);
  }

  @Test
  @SkipOldDriver("BD#15")
  void shouldRejectOperationsAfterFreeOnNewDriver() throws Exception {
    // Given Snowflake client is logged in and a Clob has been freed
    Connection connection = getDefaultConnection();
    Clob clob = connection.createClob();
    clob.setString(1, "x");
    clob.free();

    // When length is queried on the freed Clob
    SQLException error = assertThrows(SQLException.class, clob::length);

    // Then the driver rejects the call
    assertNotNull(error);
  }

  @Test
  @SkipNewDriver("BD#15")
  void shouldRemainUsableAfterFreeOnLegacyDriver() throws Exception {
    // Given Snowflake client is logged in and a legacy Clob has been freed
    Connection connection = getDefaultConnection();
    Clob clob = connection.createClob();
    clob.setString(1, "x");
    clob.free();

    // When content is written after free on the legacy driver
    clob.setString(1, "y");

    // Then the legacy Clob remains usable with cleared content
    assertEquals("y", readClobContent(clob));
  }

  @Test
  @SkipOldDriver("BD#13")
  void shouldOverwriteExistingPrefixViaSetString() throws Exception {
    // Given Snowflake client is logged in and a Clob contains existing text
    Connection connection = getDefaultConnection();
    Clob clob = connection.createClob();
    clob.setString(1, "hello world");

    // When a shorter prefix is written at position 1
    clob.setString(1, "HELLO");

    // Then the prefix is overwritten in place
    assertEquals("HELLO world", readClobContent(clob));
  }

  @Test
  @SkipNewDriver("BD#13")
  void shouldInsertAtPositionViaSetStringOnLegacyDriver() throws Exception {
    // Given Snowflake client is logged in and a legacy Clob contains existing text
    Connection connection = getDefaultConnection();
    Clob clob = connection.createClob();
    clob.setString(1, "hello world");

    // When a prefix is written at position 1 on the legacy driver
    clob.setString(1, "HELLO");

    // Then the legacy driver inserts rather than overwrites
    assertEquals("HELLOhello world", readClobContent(clob));
  }

  @Test
  void shouldAgreeOnSetStringWhenOffsetIsZero() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When setString uses offset zero
    Clob clob = connection.createClob();
    clob.setString(1, "abcdef", 0, 4);

    // Then both drivers copy the first len characters
    assertEquals("abcd", readClobContent(clob));
  }

  @Test
  @SkipOldDriver("BD#17")
  void shouldCopyInteriorFragmentWhenOffsetAndLengthAreNonZero() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When setString uses a non-zero offset and len as a character count
    Clob clob = connection.createClob();
    clob.setString(1, "XXhelloYY", 2, 5);

    // Then the interior fragment is copied per JDBC semantics
    assertEquals("hello", readClobContent(clob));
  }

  @Test
  @SkipOldDriver("BD#17")
  void shouldCopyFromMiddleOffsetWhenLengthIsCharacterCount() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When setString starts from a middle offset with a character count
    Clob clob = connection.createClob();
    clob.setString(1, "abcdef", 1, 4);

    // Then four characters are copied from the offset
    assertEquals("bcde", readClobContent(clob));
  }

  @Test
  @SkipNewDriver("BD#17")
  void shouldTreatLengthAsEndIndexForInteriorFragmentOnLegacyDriver() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When setString uses a non-zero offset on the legacy driver
    Clob clob = connection.createClob();
    clob.setString(1, "XXhelloYY", 2, 5);

    // Then len is treated as an exclusive end index
    assertEquals("hel", readClobContent(clob));
  }

  @Test
  @SkipNewDriver("BD#17")
  void shouldTreatLengthAsEndIndexFromMiddleOffsetOnLegacyDriver() throws Exception {
    // Given Snowflake client is logged in
    Connection connection = getDefaultConnection();

    // When setString starts from a middle offset on the legacy driver
    Clob clob = connection.createClob();
    clob.setString(1, "abcdef", 1, 4);

    // Then len is treated as an exclusive end index
    assertEquals("bcd", readClobContent(clob));
  }

  @Test
  void shouldAppendViaSetCharacterStreamOnConnectionClob() throws Exception {
    // Given Snowflake client is logged in and a Clob contains a prefix
    Connection connection = getDefaultConnection();
    Clob clob = connection.createClob();
    clob.setString(1, "hel");

    // When additional characters are flushed through setCharacterStream
    try (Writer writer = clob.setCharacterStream(1)) {
      writer.write("lo".toCharArray());
      writer.flush();
    }

    // Then the Clob contains the appended content
    assertEquals("hello", readClobContent(clob));
  }

  @Test
  @SkipOldDriver("BD#14")
  void shouldAppendViaSetAsciiStreamOnConnectionClob() throws Exception {
    // Given Snowflake client is logged in and an empty Clob exists
    Connection connection = getDefaultConnection();
    Clob clob = connection.createClob();

    // When ASCII bytes are written through setAsciiStream
    try (OutputStream stream = clob.setAsciiStream(1)) {
      stream.write('a');
      stream.write('b');
    }

    // Then the Clob contains the written characters
    assertEquals("ab", readClobContent(clob));
  }

  @Test
  @SkipNewDriver("BD#14")
  void shouldWriteAsciiBytesAsDecimalStringsOnLegacyDriver() throws Exception {
    // Given Snowflake client is logged in and an empty legacy Clob exists
    Connection connection = getDefaultConnection();
    Clob clob = connection.createClob();

    // When ASCII bytes are written through setAsciiStream on the legacy driver
    try (OutputStream stream = clob.setAsciiStream(1)) {
      stream.write('a');
      stream.write('b');
    }

    // Then the legacy driver stores Integer.toString(byte) for the last write only
    assertEquals("98", readClobContent(clob));
  }

  @Test
  void shouldBindSqlNullWhenSetClobReceivesNull() throws Exception {
    // Given Snowflake client is logged in and a temp table exists
    Connection connection = getDefaultConnection();
    String tableName = createTempTable(connection, "ud_clob_null_", "txt STRING");

    // When setClob binds SQL NULL
    try (PreparedStatement insert =
        connection.prepareStatement("INSERT INTO " + tableName + " (txt) VALUES (?)")) {
      insert.setClob(1, (Clob) null);
      insert.executeUpdate();
    }

    // Then the inserted column is NULL
    try (PreparedStatement select = connection.prepareStatement("SELECT txt FROM " + tableName);
        ResultSet rs = select.executeQuery()) {
      assertTrue(rs.next());
      assertNull(rs.getString(1));
      assertTrue(rs.wasNull());
    }
  }
}
