package net.snowflake.client.internal.api.implementation.connection;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.InputStream;
import java.io.OutputStream;
import java.io.Reader;
import java.io.Writer;
import java.sql.Clob;
import java.sql.SQLException;
import net.snowflake.client.internal.api.decorator.Telemetry;
import org.junit.jupiter.api.Test;

public class SnowflakeClobTest {

  // SnowflakeClob is a @JdbcBoundary: its generated decorator is the public contract, translating
  // the impl's runtime carriers (SFSQLException) into the checked SQLException JDBC promises. These
  // tests assert that contract, so they construct the Clob through the decorator.
  private static Clob decoratedClob() {
    return new DecoratedSnowflakeClob(new SnowflakeClob(), Telemetry.NOOP);
  }

  private static Clob decoratedClob(String initial) {
    return new DecoratedSnowflakeClob(new SnowflakeClob(initial), Telemetry.NOOP);
  }

  @Test
  public void shouldSetStringOnEmptyClobAndRoundTripViaGetSubString() throws SQLException {
    Clob clob = decoratedClob();

    // The createClob() usage pattern: build content on an initially-empty Clob starting at pos 1.
    // The JDK SerialClob throws "Invalid position in Clob object set" here, which is why a mutable
    // growable Clob is required.
    int written = clob.setString(1, "hello world");

    assertEquals(11, written);
    assertEquals(11, clob.length());
    // setClob() reads the whole value via getSubString(1, length); it must return the full content.
    assertEquals("hello world", clob.getSubString(1, (int) clob.length()));
  }

  @Test
  public void shouldReturnEmptyStringForZeroLengthGetSubStringOnEmptyClob() throws SQLException {
    Clob clob = decoratedClob();
    assertEquals(0, clob.length());
    assertEquals("", clob.getSubString(1, 0));
  }

  @Test
  public void shouldOverwriteAndExtendViaSetString() throws SQLException {
    // BD#13: overwrite prefix in-place; legacy driver would insert and yield "HELLOhello world".
    Clob clob = decoratedClob("hello");
    clob.setString(6, " world");
    assertEquals("hello world", clob.getSubString(1, (int) clob.length()));

    clob.setString(1, "HELLO");
    assertEquals("HELLO world", clob.getSubString(1, (int) clob.length()));
  }

  @Test
  public void shouldTruncateAndFindPosition() throws SQLException {
    Clob clob = decoratedClob("hello world");
    assertEquals(7, clob.position("world", 1));
    clob.truncate(5);
    assertEquals("hello", clob.getSubString(1, (int) clob.length()));
    assertEquals(-1, clob.position("world", 1));
  }

  @Test
  public void shouldRejectInvalidPositions() throws SQLException {
    Clob clob = decoratedClob("abc");
    assertThrows(SQLException.class, () -> clob.setString(0, "x"));
    assertThrows(SQLException.class, () -> clob.getSubString(0, 1));
    // Writing past the end (a gap) is invalid.
    assertThrows(SQLException.class, () -> clob.setString(10, "x"));
  }

  @Test
  public void shouldRejectOperationsAfterFree() throws SQLException {
    // BD#15: JDBC-invalid after free(); legacy driver resets buffer and keeps the Clob usable.
    Clob clob = decoratedClob("abc");
    clob.free();
    assertThrows(SQLException.class, clob::length);
    assertThrows(SQLException.class, () -> clob.getSubString(1, 1));
    assertThrows(SQLException.class, () -> clob.setString(1, "x"));
    assertThrows(SQLException.class, () -> clob.position("a", 1));
    assertThrows(SQLException.class, () -> clob.truncate(0));
  }

  @Test
  public void shouldSetStringWithOffsetAndLength() throws SQLException {
    // BD#17: len is a character count; legacy driver treats len as a substring end index.
    Clob clob = decoratedClob();
    int written = clob.setString(1, "XXhelloYY", 2, 5);
    assertEquals(5, written);
    assertEquals("hello", clob.getSubString(1, (int) clob.length()));
    assertThrows(SQLException.class, () -> clob.setString(1, "abc", 1, 5));
  }

  @Test
  public void shouldMatchLegacySliceWhenOffsetIsZero() throws SQLException {
    // BD#17: offset == 0 is the only case where substring(offset, len) matches
    // substring(offset, offset + len); both drivers copy the first len characters.
    Clob clob = decoratedClob();
    clob.setString(1, "abcdef", 0, 4);
    assertEquals("abcd", clob.getSubString(1, (int) clob.length()));
  }

  @Test
  public void shouldCopyLenCharactersFromNonZeroOffset() throws SQLException {
    // BD#17: new -> "bcde"; legacy substring(1, 4) -> "bcd".
    Clob clob = decoratedClob();
    clob.setString(1, "abcdef", 1, 4);
    assertEquals(4, clob.length());
    assertEquals("bcde", clob.getSubString(1, (int) clob.length()));
  }

  @Test
  public void shouldRejectOutOfBoundsCharacterStreamWindow() throws Exception {
    Clob clob = decoratedClob("abc");
    // getSubString truncates, but getCharacterStream(pos, length) must reject an over-long window.
    assertEquals("bc", clob.getSubString(2, 100));
    assertThrows(SQLException.class, () -> clob.getCharacterStream(2, 100));
    assertThrows(SQLException.class, () -> clob.getCharacterStream(0, 1));
    // A zero-length window at a valid position is allowed and yields an empty Reader (JDBC
    // contract).
    try (Reader reader = clob.getCharacterStream(2, 0)) {
      assertEquals(-1, reader.read());
    }
  }

  @Test
  public void shouldFindPositionWithClobAndRejectNull() throws SQLException {
    Clob clob = decoratedClob("hello world");
    Clob needle = decoratedClob("world");
    assertEquals(7, clob.position(needle, 1));
    assertThrows(SQLException.class, () -> clob.position((String) null, 1));
    assertThrows(SQLException.class, () -> clob.position((Clob) null, 1));
  }

  @Test
  public void shouldReturnOneBytePerCharFromAsciiStream() throws Exception {
    Clob clob = decoratedClob("ab");
    try (InputStream stream = clob.getAsciiStream()) {
      assertEquals('a', stream.read());
      assertEquals('b', stream.read());
      assertEquals(-1, stream.read());
    }
  }

  @Test
  public void shouldAppendViaSetCharacterStreamWriter() throws Exception {
    Clob clob = decoratedClob("hel");
    try (Writer writer = clob.setCharacterStream(1)) {
      writer.write("lo".toCharArray());
      writer.flush();
    }
    assertEquals("hello", clob.getSubString(1, (int) clob.length()));
  }

  @Test
  public void shouldAppendViaSetAsciiStream() throws Exception {
    Clob clob = decoratedClob();
    try (OutputStream stream = clob.setAsciiStream(1)) {
      stream.write('a');
      stream.write('b');
    }
    assertEquals("ab", clob.getSubString(1, (int) clob.length()));
  }
}
