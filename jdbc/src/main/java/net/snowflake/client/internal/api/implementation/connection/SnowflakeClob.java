package net.snowflake.client.internal.api.implementation.connection;

import static java.lang.Integer.MAX_VALUE;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.io.Reader;
import java.io.StringReader;
import java.io.Writer;
import java.nio.charset.StandardCharsets;
import java.sql.Clob;
import java.sql.SQLException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.codegen.JdbcBoundary;

/**
 * A simple mutable {@link Clob} backed by a {@link StringBuilder}. {@link
 * java.sql.Connection#createClob()} returns an empty, growable Clob so callers can build content
 * with {@link #setString(long, String)} starting at position 1; the JDK's {@code SerialClob} cannot
 * be used here because it is fixed-length and rejects writes past its (zero) length. All positions
 * are 1-based per the JDBC contract.
 */
@JdbcBoundary
public class SnowflakeClob implements Clob {

  private StringBuilder buffer;

  public SnowflakeClob() {
    this.buffer = new StringBuilder();
  }

  public SnowflakeClob(String content) {
    this.buffer = new StringBuilder(content == null ? "" : content);
  }

  private void checkFreed() {
    if (buffer == null) {
      throw new SFSQLException("Clob has been freed and is no longer valid.");
    }
  }

  @Override
  public long length() {
    checkFreed();
    return buffer.length();
  }

  @Override
  public String getSubString(long pos, int length) {
    checkFreed();
    if (pos < 1 || length < 0 || pos - 1 > MAX_VALUE) {
      throw new SFSQLException(
          "Invalid arguments to getSubString: pos=" + pos + ", length=" + length);
    }
    int start = (int) (pos - 1);
    if (start > buffer.length()) {
      throw new SFSQLException("Invalid position in Clob: " + pos);
    }
    int end = (int) Math.min((long) start + length, buffer.length());
    return buffer.substring(start, end);
  }

  @Override
  public Reader getCharacterStream() {
    checkFreed();
    return new StringReader(buffer.toString());
  }

  @Override
  public InputStream getAsciiStream() {
    checkFreed();
    // ASCII streams are one byte per character; ISO-8859-1 maps each char to its low byte rather
    // than emitting multi-byte UTF-8 sequences for code points above U+007F.
    return new ByteArrayInputStream(buffer.toString().getBytes(StandardCharsets.ISO_8859_1));
  }

  @Override
  public long position(String searchstr, long start) {
    checkFreed();
    if (searchstr == null) {
      throw new SFSQLException("Search string is null.");
    }
    if (start < 1) {
      throw new SFSQLException("Invalid start position: " + start);
    }
    if (start - 1 > MAX_VALUE) {
      return -1;
    }
    int index = buffer.indexOf(searchstr, (int) (start - 1));
    return index < 0 ? -1 : index + 1;
  }

  @Override
  public long position(Clob searchstr, long start) {
    checkFreed();
    if (searchstr == null) {
      throw new SFSQLException("Search Clob is null.");
    }
    long searchLength = 0;
    try {
      searchLength = searchstr.length();
      if (searchLength > MAX_VALUE) {
        throw new SFSQLException("Search Clob length exceeds the maximum supported size.");
      }
      return position(searchstr.getSubString(1, (int) searchLength), start);
    } catch (SQLException e) {
      throw new RuntimeException(e);
    }
  }

  @Override
  public int setString(long pos, String str) {
    checkFreed();
    if (str == null) {
      throw new SFSQLException("Cannot set a null string on a Clob.");
    }
    return setString(pos, str, 0, str.length());
  }

  @Override
  public int setString(long pos, String str, int offset, int len) {
    checkFreed();
    if (str == null) {
      throw new SFSQLException("Cannot set a null string on a Clob.");
    }
    if (pos < 1
        || offset < 0
        || len < 0
        || (long) offset + len > str.length()
        || pos - 1 > MAX_VALUE) {
      throw new SFSQLException("Invalid arguments to setString.");
    }
    int start = (int) (pos - 1);
    if (start > buffer.length()) {
      throw new SFSQLException("Invalid position in Clob: " + pos);
    }
    String fragment = str.substring(offset, offset + len);
    int end = Math.min(start + fragment.length(), buffer.length());
    buffer.replace(start, end, fragment);
    return fragment.length();
  }

  @Override
  public OutputStream setAsciiStream(long pos) {
    checkFreed();
    if (pos < 1) {
      throw new SFSQLException("Invalid position in Clob: " + pos);
    }
    return new ClobAsciiOutputStream(buffer, (int) (pos - 1));
  }

  @Override
  public Writer setCharacterStream(long pos) {
    checkFreed();
    if (pos < 1) {
      throw new SFSQLException("Invalid position in Clob: " + pos);
    }
    // Legacy snowflake-jdbc appends flushed writer content; pos is accepted but not applied.
    return new ClobAppendWriter(buffer);
  }

  @Override
  public void truncate(long len) {
    checkFreed();
    if (len < 0 || len > buffer.length()) {
      throw new SFSQLException("Invalid truncation length: " + len);
    }
    buffer.setLength((int) len);
  }

  @Override
  public void free() {
    buffer = null;
  }

  @Override
  public Reader getCharacterStream(long pos, long length) {
    checkFreed();
    if (pos < 1 || pos > buffer.length()) {
      throw new SFSQLException("Invalid position in Clob: " + pos);
    }
    // Unlike getSubString(), this overload must reject (rather than silently truncate) a window
    // that
    // runs past the end of the Clob, matching the JDBC contract and the JDK SerialClob behavior. A
    // zero-length window at a valid position is allowed and yields an empty Reader.
    if (length < 0 || length > MAX_VALUE || pos - 1 + length > buffer.length()) {
      throw new SFSQLException("Invalid position and length: pos=" + pos + ", length=" + length);
    }
    return new StringReader(getSubString(pos, (int) length));
  }

  @Override
  public String toString() {
    return buffer == null ? "" : buffer.toString();
  }

  /**
   * Minimal port of legacy {@code SnowflakeClob.StringBufferOutputStream}: append when writing past
   * the end, otherwise replace one character at the current offset.
   */
  private static final class ClobAsciiOutputStream extends OutputStream {
    private final StringBuilder buffer;
    private int offset;

    private ClobAsciiOutputStream(StringBuilder buffer, int startIndex) {
      this.buffer = buffer;
      this.offset = startIndex;
    }

    @Override
    public void write(int b) {
      if (offset >= buffer.length()) {
        buffer.append((char) b);
      } else {
        buffer.replace(offset, offset + 1, String.valueOf((char) b));
      }
      offset++;
    }
  }

  /**
   * Minimal port of legacy {@code SnowflakeClob.StringBufferWriter}: buffer writes and append to
   * the Clob on {@link #flush()}/{@link #close()}.
   */
  private static final class ClobAppendWriter extends Writer {
    private final StringBuilder main;
    private final StringBuilder pending = new StringBuilder();
    private boolean closed;

    private ClobAppendWriter(StringBuilder main) {
      this.main = main;
    }

    @Override
    public void write(char[] cbuf, int off, int len) throws IOException {
      if (closed) {
        throw new IOException("Writer is closed.");
      }
      pending.append(cbuf, off, len);
    }

    @Override
    public void flush() {
      if (closed) {
        return;
      }
      main.append(pending);
      pending.setLength(0);
    }

    @Override
    public void close() throws IOException {
      if (closed) {
        return;
      }
      flush();
      closed = true;
    }
  }
}
