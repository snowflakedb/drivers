package net.snowflake.client.internal.api.implementation.connection;

import java.io.ByteArrayInputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.Reader;
import java.io.StringReader;
import java.io.Writer;
import java.sql.Clob;
import java.sql.SQLException;

class SnowflakeClob implements Clob {
  private StringBuffer buffer;

  SnowflakeClob() {
    buffer = new StringBuffer();
  }

  @Override
  public long length() throws SQLException {
    return buffer.length();
  }

  @Override
  public String getSubString(long pos, int length) throws SQLException {
    if (pos < 1 || length < 0) {
      throw new SQLException();
    }
    return buffer.substring((int) pos - 1, (int) pos - 1 + length);
  }

  @Override
  public Reader getCharacterStream() throws SQLException {
    return new StringReader(buffer.toString());
  }

  @Override
  public InputStream getAsciiStream() throws SQLException {
    return new ByteArrayInputStream(buffer.toString().getBytes());
  }

  @Override
  public long position(String searchstr, long start) throws SQLException {
    if (start < 1) {
      throw new SQLException();
    }
    return (long) buffer.lastIndexOf(searchstr, (int) start - 1);
  }

  @Override
  public long position(Clob searchstr, long start) throws SQLException {
    if (start < 1) {
      throw new SQLException();
    }
    return (long) buffer.lastIndexOf(searchstr.toString(), (int) start - 1);
  }

  @Override
  public int setString(long pos, String str) throws SQLException {
    if (pos < 1) {
      throw new SQLException();
    }
    buffer.insert((int) pos - 1, str);
    return str.length();
  }

  @Override
  public int setString(long pos, String str, int offset, int len) throws SQLException {
    if (pos < 1) {
      throw new SQLException();
    }
    String substring = str.substring(offset, len);
    buffer.insert((int) pos - 1, substring);
    return substring.length();
  }

  @Override
  public OutputStream setAsciiStream(long pos) throws SQLException {
    throw new SQLException("setAsciiStream not supported");
  }

  @Override
  public Writer setCharacterStream(long pos) throws SQLException {
    throw new SQLException("setCharacterStream not supported");
  }

  @Override
  public void truncate(long len) throws SQLException {
    if (buffer.length() > len) {
      buffer.delete((int) len, buffer.length());
    }
  }

  @Override
  public void free() throws SQLException {
    buffer = new StringBuffer();
  }

  @Override
  public Reader getCharacterStream(long pos, long length) throws SQLException {
    return new StringReader(buffer.substring((int) pos - 1, (int) pos - 1 + (int) length));
  }

  @Override
  public String toString() {
    return buffer.toString();
  }
}
