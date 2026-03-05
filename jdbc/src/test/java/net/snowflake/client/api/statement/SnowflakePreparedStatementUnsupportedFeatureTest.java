package net.snowflake.client.api.statement;

import static org.junit.jupiter.api.Assertions.assertThrows;

import java.io.ByteArrayInputStream;
import java.io.StringReader;
import java.sql.PreparedStatement;
import java.sql.SQLFeatureNotSupportedException;
import net.snowflake.client.SnowflakeIntegrationTestBase;
import org.junit.jupiter.api.AfterAll;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;

@SuppressWarnings("deprecation")
@TestInstance(TestInstance.Lifecycle.PER_CLASS)
public class SnowflakePreparedStatementUnsupportedFeatureTest extends SnowflakeIntegrationTestBase {

  private PreparedStatement preparedStatement;

  @BeforeAll
  public void setUp() throws Exception {
    preparedStatement = getDefaultConnection().prepareStatement("select ? ? ?");
  }

  @AfterAll
  public void tearDown() throws Exception {
    if (preparedStatement != null) {
      preparedStatement.close();
    }
  }

  @Test
  public void testSetAsciiStreamWithIntLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setAsciiStream(1, new ByteArrayInputStream(new byte[] {1}), 1));
  }

  @Test
  public void testSetUnicodeStreamWithIntLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setUnicodeStream(1, new ByteArrayInputStream(new byte[] {1}), 1));
  }

  @Test
  public void testSetBinaryStreamWithIntLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setBinaryStream(1, new ByteArrayInputStream(new byte[] {1}), 1));
  }

  @Test
  public void testSetCharacterStreamWithIntLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setCharacterStream(1, new StringReader("x"), 1));
  }

  @Test
  public void testSetAsciiStreamWithLongLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setAsciiStream(1, new ByteArrayInputStream(new byte[] {1}), 1L));
  }

  @Test
  public void testSetBinaryStreamWithLongLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setBinaryStream(1, new ByteArrayInputStream(new byte[] {1}), 1L));
  }

  @Test
  public void testSetCharacterStreamWithLongLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setCharacterStream(1, new StringReader("x"), 1L));
  }

  @Test
  public void testSetAsciiStreamWithoutLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setAsciiStream(1, new ByteArrayInputStream(new byte[] {1})));
  }

  @Test
  public void testSetBinaryStreamWithoutLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setBinaryStream(1, new ByteArrayInputStream(new byte[] {1})));
  }

  @Test
  public void testSetCharacterStreamWithoutLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setCharacterStream(1, new StringReader("x")));
  }

  @Test
  public void testSetNCharacterStreamWithLongLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setNCharacterStream(1, new StringReader("x"), 1L));
  }

  @Test
  public void testSetNCharacterStreamWithoutLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setNCharacterStream(1, new StringReader("x")));
  }

  @Test
  public void testSetClobReaderWithLongLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setClob(1, new StringReader("x"), 1L));
  }

  @Test
  public void testSetClobReaderWithoutLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setClob(1, new StringReader("x")));
  }

  @Test
  public void testSetBlobInputStreamWithLongLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setBlob(1, new ByteArrayInputStream(new byte[] {1}), 1L));
  }

  @Test
  public void testSetBlobInputStreamWithoutLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setBlob(1, new ByteArrayInputStream(new byte[] {1})));
  }

  @Test
  public void testSetNClobObjectThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setNClob(1, (java.sql.NClob) null));
  }

  @Test
  public void testSetNClobReaderWithLongLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setNClob(1, new StringReader("x"), 1L));
  }

  @Test
  public void testSetNClobReaderWithoutLengthThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setNClob(1, new StringReader("x")));
  }

  @Test
  public void testSetRefThrowsFeatureNotSupported() {
    assertThrows(SQLFeatureNotSupportedException.class, () -> preparedStatement.setRef(1, null));
  }

  @Test
  public void testSetBlobObjectThrowsFeatureNotSupported() {
    assertThrows(
        SQLFeatureNotSupportedException.class,
        () -> preparedStatement.setBlob(1, (java.sql.Blob) null));
  }

  @Test
  public void testSetRowIdThrowsFeatureNotSupported() {
    assertThrows(SQLFeatureNotSupportedException.class, () -> preparedStatement.setRowId(1, null));
  }

  @Test
  public void testSetUrlThrowsFeatureNotSupported() {
    assertThrows(SQLFeatureNotSupportedException.class, () -> preparedStatement.setURL(1, null));
  }

  @Test
  public void testSetSqlXmlThrowsFeatureNotSupported() {
    assertThrows(SQLFeatureNotSupportedException.class, () -> preparedStatement.setSQLXML(1, null));
  }
}
