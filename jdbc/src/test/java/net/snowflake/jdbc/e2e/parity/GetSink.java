package net.snowflake.jdbc.e2e.parity;

import java.sql.Date;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Time;
import java.sql.Timestamp;
import java.util.Calendar;
import java.util.Objects;
import java.util.TimeZone;

/** Read-side getter under test. Each variant runs once per (driver, column). */
public enum GetSink {
  GET_STRING {
    @Override
    Object read(ResultSet rs, int col) throws SQLException {
      return rs.getString(col);
    }
  },
  GET_DATE {
    @Override
    Object read(ResultSet rs, int col) throws SQLException {
      return rs.getDate(col);
    }
  },
  GET_DATE_CAL_UTC {
    @Override
    Object read(ResultSet rs, int col) throws SQLException {
      return rs.getDate(col, Calendar.getInstance(TimeZone.getTimeZone("UTC")));
    }
  },
  GET_TIME {
    @Override
    Object read(ResultSet rs, int col) throws SQLException {
      return rs.getTime(col);
    }
  },
  GET_TIME_CAL_UTC {
    @Override
    Object read(ResultSet rs, int col) throws SQLException {
      return rs.getTime(col, Calendar.getInstance(TimeZone.getTimeZone("UTC")));
    }
  },
  GET_TIMESTAMP {
    @Override
    Object read(ResultSet rs, int col) throws SQLException {
      return rs.getTimestamp(col);
    }
  },
  GET_TIMESTAMP_CAL_UTC {
    @Override
    Object read(ResultSet rs, int col) throws SQLException {
      return rs.getTimestamp(col, Calendar.getInstance(TimeZone.getTimeZone("UTC")));
    }
  },
  GET_OBJECT {
    @Override
    Object read(ResultSet rs, int col) throws SQLException {
      return rs.getObject(col);
    }
  };

  abstract Object read(ResultSet rs, int col) throws SQLException;

  /**
   * Render a sink output as a string that compares safely across classloaders. Driver-private
   * subclasses of {@link Timestamp} (e.g. SnowflakeTimestampWithTimezone) live in their respective
   * classloaders; their {@code toString()} crosses the boundary fine, but {@code equals} would
   * reject the cross-loader compare. Reduce to bootstrap-loaded primitives + the implementation's
   * own toString.
   */
  static String describe(Object v) {
    if (v == null) {
      return "null";
    }
    if (v instanceof Timestamp) {
      Timestamp t = (Timestamp) v;
      return "Timestamp{millis=" + t.getTime() + ",nanos=" + t.getNanos() + ",str=" + t + "}";
    }
    if (v instanceof Time) {
      Time t = (Time) v;
      return "Time{millis=" + t.getTime() + ",str=" + t + "}";
    }
    if (v instanceof Date) {
      Date d = (Date) v;
      return "Date{millis=" + d.getTime() + ",str=" + d + "}";
    }
    if (v instanceof byte[]) {
      byte[] b = (byte[]) v;
      StringBuilder sb = new StringBuilder("bytes{");
      for (int i = 0; i < b.length; i++) {
        if (i > 0) {
          sb.append(',');
        }
        sb.append(Integer.toHexString(b[i] & 0xFF));
      }
      return sb.append('}').toString();
    }
    return v.getClass().getName() + "{" + Objects.toString(v) + "}";
  }
}
