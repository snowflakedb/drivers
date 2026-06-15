package net.snowflake.jdbc.e2e.parity;

import java.sql.PreparedStatement;
import java.sql.SQLException;
import java.sql.Types;
import java.util.Calendar;
import java.util.TimeZone;

/**
 * Write-side setter under test. Each variant runs once per (driver, parameter). Every variant is
 * applicable to every {@link SfType}: {@link JavaValues} produces a typed value for any (type,
 * literal) pair, so e.g. SET_DATE can bind a value derived from a TIME literal and we test how each
 * driver handles the cross-type bind.
 */
public enum SetSink {
  SET_DATE {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setDate(idx, JavaValues.asDate(type, literal));
    }
  },
  SET_DATE_CAL_UTC {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setDate(idx, JavaValues.asDate(type, literal), utcCalendar());
    }
  },
  SET_TIME {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setTime(idx, JavaValues.asTime(type, literal));
    }
  },
  SET_TIME_CAL_UTC {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setTime(idx, JavaValues.asTime(type, literal), utcCalendar());
    }
  },
  SET_TIMESTAMP {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setTimestamp(idx, JavaValues.asTimestamp(type, literal));
    }
  },
  SET_TIMESTAMP_CAL_UTC {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setTimestamp(idx, JavaValues.asTimestamp(type, literal), utcCalendar());
    }
  },
  SET_OBJECT {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setObject(idx, JavaValues.asNatural(type, literal));
    }
  },
  SET_OBJECT_TYPED_DATE {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setObject(idx, JavaValues.asDate(type, literal), Types.DATE);
    }
  },
  SET_OBJECT_TYPED_TIME {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setObject(idx, JavaValues.asTime(type, literal), Types.TIME);
    }
  },
  SET_OBJECT_TYPED_TIMESTAMP {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setObject(idx, JavaValues.asTimestamp(type, literal), Types.TIMESTAMP);
    }
  },
  SET_STRING {
    @Override
    void bind(PreparedStatement ps, int idx, SfType type, String literal) throws SQLException {
      ps.setString(idx, JavaValues.asString(literal));
    }
  };

  abstract void bind(PreparedStatement ps, int idx, SfType type, String literal)
      throws SQLException;

  private static Calendar utcCalendar() {
    return Calendar.getInstance(TimeZone.getTimeZone("UTC"));
  }
}
