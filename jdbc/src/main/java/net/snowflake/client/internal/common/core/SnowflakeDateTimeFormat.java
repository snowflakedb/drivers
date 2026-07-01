package net.snowflake.client.internal.common.core;

import java.sql.Time;
import java.sql.Timestamp;
import java.text.SimpleDateFormat;
import java.time.LocalTime;
import java.util.ArrayList;
import java.util.GregorianCalendar;
import java.util.List;
import java.util.TimeZone;

/** Copied directly from snowflake-common to maintain behavioral parity */
public class SnowflakeDateTimeFormat {
  private static final TimeZone GMT = TimeZone.getTimeZone("GMT");

  public enum ElementType {
    Year2digit_ElementType("YY", "yy"),
    Year_ElementType("YYYY", "yyyy"),
    Month_ElementType("MM", "MM"),
    MonthAbbrev_ElementType("MON", "MMM"),
    MonthFullName_ElementType("MMMM", "MMMM"),
    DayOfMonth_ElementType("DD", "dd"),
    DayOfWeekAbbrev_ElementType("DY", "EEE"),
    Hour24_ElementType("HH24", "HH"),
    Hour12_ElementType("HH12", "hh"),
    Hour_ElementType("HH", "HH"),
    Ante_Meridiem_ElementType("AM", "a"),
    Post_Meridiem_ElementType("PM", "a"),
    Minute_ElementType("MI", "mm"),
    Second_ElementType("SS", "ss"),
    MilliSecond_ElementType("FF", ""), // special code for parsing fractions
    TZOffsetHourColonMin_ElementType("TZH:TZM", "XXX"),
    TZOffsetHourMin_ElementType("TZHTZM", "XX"),
    TZOffsetHourOnly_ElementType("TZH", "X"),
    TZAbbr_ElementType("TZD", "z");

    private final String sqlFormat;
    private final String javaFormat; // java SimpleDateFormat

    /**
     * Constructor for ElementType
     *
     * @param sqlFormat
     * @param javaFormat
     */
    ElementType(String sqlFormat, String javaFormat) {
      this.sqlFormat = sqlFormat;
      this.javaFormat = javaFormat;
    }

    public String getSqlFormat() {
      return sqlFormat;
    }

    public String getJavaFormat() {
      return javaFormat;
    }
  }

  private static class Fragment {
    private final String javaFormat;

    Fragment(String javaFormat) {
      this.javaFormat = javaFormat;
    }
  }

  /** Our SQL format */
  private final String sqlFormat;

  private final boolean isDeprecated;

  /** If set, we'll use auto-logic during parsing */
  private boolean automaticParsing;

  /** Enables auto-scaling of Epoch time */
  public boolean epochAutoScale = true;

  private List<Fragment> fragments;
  private SimpleDateFormat simpleDateFormat;

  // Precision of the fractions. If -1, type-based.
  private int fractionsLen = -1;
  // Position of the fractions in the format. If -1, fractions are absent.
  private int fractionsPos = -1;
  // Defines if fractions are prefixed with a dot
  private boolean fractionsWithDot = false;
  // Formatter used to parse everything before fractions to find their place
  private SimpleDateFormat fractionsPreFormatter;

  // Formatter used to parse everything before the timezone to find its place.
  // If null, we know it's absent.
  private SimpleDateFormat timezonePreFormatter;
  // If we have timezones, what is their format.
  private ElementType timezoneElementType;

  // If we have a 2-year year
  private boolean has2digitYear = false;

  // Bitmasks to use in type definition
  public static final int DATE = 1;
  public static final int TIME = 2;
  public static final int TIMESTAMP = 4;
  public static final int ANY_TYPE = DATE | TIME | TIMESTAMP;

  private final int type;

  // Array of all supported formats, to be used in AUTO parsing.
  // NOTE: generated with gen_date_formats.py, do not edit manually.
  // Should to be in sync with the CPP version
  private static final SnowflakeDateTimeFormat[] acceptedFormats = {
    // ISO_DATE_T_HOUR24_MINUTE_SECOND_FRAC_TZHM
    // Ex: "2013-04-28T20:57:01.123456789+07:00"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD\"T\"HH24:MI:SS.FFTZH:TZM", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_SECOND_FRAC_TZHCM
    // Ex: "2013-04-28 20:57:01.123456789+07:00"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SS.FFTZH:TZM", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_SECOND_FRAC_TZH
    // Ex: "2013-04-28 20:57:01.123456789+07"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SS.FFTZH", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_SECOND_FRAC_TZSHCM
    // Ex: "2013-04-28 20:57:01.123456789 +07:00"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SS.FF TZH:TZM", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_SECOND_FRAC_TZSHM
    // Ex: "2013-04-28 20:57:01.123456789 +0700"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SS.FF TZHTZM", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_SECOND_TZSHCM
    // Ex: "2013-04-28 20:57:01 +07:00"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SS TZH:TZM", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_SECOND_TZSHM
    // Ex: "2013-04-28 20:57:01 +0700"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SS TZHTZM", TIMESTAMP | DATE),

    // ISO_DATE_T_HOUR24_MINUTE_SECOND_FRAC
    // Ex: "2013-04-28T20:57:01.123456"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD\"T\"HH24:MI:SS.FF", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_SECOND_FRAC2
    // Ex: "2013-04-28 20:57:01.123456"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SS.FF", TIMESTAMP | DATE),

    // ISO_DATE_T_HOUR24_MINUTE_SECOND
    // Ex: "2013-04-28T20:57:01"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD\"T\"HH24:MI:SS", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_SECOND2
    // Ex: "2013-04-28 20:57:01"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SS", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE
    // Ex: "2013-04-28T20:57"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD\"T\"HH24:MI", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE2
    // Ex: "2013-04-28 20:57"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI", TIMESTAMP | DATE),

    // ISO_DATE_T_HOUR24
    // Ex: "2013-04-28T20"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD\"T\"HH24", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_2
    // Ex: "2013-04-28 20"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24", TIMESTAMP | DATE),

    // ISO_DATE
    // Ex: "2013-04-28"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD", TIMESTAMP | DATE),

    // ISO_US_SIMPLE_DATE
    // Ex: "17-DEC-1980"
    SnowflakeDateTimeFormat.fromSqlFormat("DD-MON-YYYY", TIMESTAMP | DATE),

    // ISO_DATE_T_HOUR24_MINUTE_SECOND_TZHCM
    // Ex: "2013-04-28T20:57:01-07:00"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD\"T\"HH24:MI:SSTZH:TZM", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_SECOND_TZHCM
    // Ex: "2013-04-28 20:57:01-07:00"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SSTZH:TZM", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_SECOND_TZH
    // Ex: "2013-04-28 20:57:01-07"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MI:SSTZH", TIMESTAMP | DATE),

    // ISO_DATE_T_HOUR24_MINUTE_TZHCM
    // Ex: "2013-04-28T20:57+07:00"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD\"T\"HH24:MITZH:TZM", TIMESTAMP | DATE),

    // ISO_DATE_HOUR24_MINUTE_TZHCM
    // Ex: "2013-04-28 20:57+07:00"
    SnowflakeDateTimeFormat.fromSqlFormat("YYYY-MM-DD HH24:MITZH:TZM", TIMESTAMP | DATE),

    // RFC_DATE_HOUR24_MINUTE_SECOND_TZ
    // Ex: "Thu, 21 Dec 2000 16:01:07 +0200"
    SnowflakeDateTimeFormat.fromSqlFormat("DY, DD MON YYYY HH24:MI:SS TZHTZM", TIMESTAMP | DATE),

    // RFC_DATE_HOUR24_MINUTE_SECOND_FRAC_TZ
    // Ex: "Thu, 21 Dec 2000 16:01:07.123456 +0200"
    SnowflakeDateTimeFormat.fromSqlFormat("DY, DD MON YYYY HH24:MI:SS.FF TZHTZM", TIMESTAMP | DATE),

    // RFC_DATE_HOUR12_MINUTE_SECOND_MERIDIEM_TZ
    // Ex: "Thu, 21 Dec 2000 04:01:07 PM +0200"
    SnowflakeDateTimeFormat.fromSqlFormat("DY, DD MON YYYY HH12:MI:SS AM TZHTZM", TIMESTAMP | DATE),

    // RFC_DATE_HOUR12_MINUTE_SECOND_FRAC_MERIDIEM_TZ
    // Ex: "Thu, 21 Dec 2000 04:01:07.123456 PM +0200"
    SnowflakeDateTimeFormat.fromSqlFormat(
        "DY, DD MON YYYY HH12:MI:SS.FF AM TZHTZM", TIMESTAMP | DATE),

    // RFC_DATE_HOUR24_MINUTE_SECOND
    // Ex: "Thu, 21 Dec 2000 16:01:07"
    SnowflakeDateTimeFormat.fromSqlFormat("DY, DD MON YYYY HH24:MI:SS", TIMESTAMP | DATE),

    // RFC_DATE_HOUR24_MINUTE_SECOND_FRAC
    // Ex: "Thu, 21 Dec 2000 16:01:07.123456"
    SnowflakeDateTimeFormat.fromSqlFormat("DY, DD MON YYYY HH24:MI:SS.FF", TIMESTAMP | DATE),

    // RFC_DATE_HOUR12_MINUTE_SECOND_MERIDIEM
    // Ex: "Thu, 21 Dec 2000 04:01:07 PM"
    SnowflakeDateTimeFormat.fromSqlFormat("DY, DD MON YYYY HH12:MI:SS AM", TIMESTAMP | DATE),

    // RFC_DATE_HOUR12_MINUTE_SECOND_FRAC_MERIDIEM
    // Ex: "Thu, 21 Dec 2000 04:01:07.123456 PM"
    SnowflakeDateTimeFormat.fromSqlFormat("DY, DD MON YYYY HH12:MI:SS.FF AM", TIMESTAMP | DATE),

    // TWITTER_DATE_HOUR24_MIN_SEC_TZ_YEAR
    // Twitter timestamp format. Ex: Mon Jul 08 18:09:51 +0000 2013
    SnowflakeDateTimeFormat.fromSqlFormat("DY MON DD HH24:MI:SS TZHTZM YYYY", TIMESTAMP | DATE),

    // ISO_HOUR24_MINUTE_SECOND_FRAC_TZ
    // Ex: "20:57:01.123456789+07:00"
    SnowflakeDateTimeFormat.fromSqlFormat("HH24:MI:SS.FFTZH:TZM", TIME),

    // ISO_HOUR24_MINUTE_SECOND_FRAC
    // Ex: "20:57:01.123456789"
    SnowflakeDateTimeFormat.fromSqlFormat("HH24:MI:SS.FF", TIME),

    // RFC_HOUR12_MINUTE_SECOND_FRAC_MERIDIEM
    // Ex: "07:57:01.123456789 PM"
    SnowflakeDateTimeFormat.fromSqlFormat("HH12:MI:SS.FF AM", TIME),

    // ISO_HOUR24_MINUTE_SECOND
    // Ex: "20:57:01"
    SnowflakeDateTimeFormat.fromSqlFormat("HH24:MI:SS", TIME),

    // RFC_HOUR12_MINUTE_SECOND_MERIDIEM
    // Ex: "04:01:07 PM"
    SnowflakeDateTimeFormat.fromSqlFormat("HH12:MI:SS AM", TIME),

    // ISO_HOUR24_MINUTE
    // Ex: "20:57"
    SnowflakeDateTimeFormat.fromSqlFormat("HH24:MI", TIME),

    // RFC_HOUR12_MINUTE_MERIDIEM
    // Ex: "04:01 PM"
    SnowflakeDateTimeFormat.fromSqlFormat("HH12:MI AM", TIME),

    // ISO_US_DATE_ALT1
    // Ex: "12/17/1980"
    SnowflakeDateTimeFormat.deprecatedFormat("MM/DD/YYYY", TIMESTAMP | DATE),

    // ALT_DATE_HOUR24_MIN_SEC
    // Ex: "2/18/2008 02:36:48"
    SnowflakeDateTimeFormat.deprecatedFormat("MM/DD/YYYY HH24:MI:SS", TIMESTAMP | DATE),
  };

  public static SnowflakeDateTimeFormat fromSqlFormat(String sqlFormat) {
    return new SnowflakeDateTimeFormat(sqlFormat, ANY_TYPE, false);
  }

  private static SnowflakeDateTimeFormat fromSqlFormat(String sqlFormat, int type) {
    return new SnowflakeDateTimeFormat(sqlFormat, type, false);
  }

  private static SnowflakeDateTimeFormat deprecatedFormat(String sqlFormat, int type) {
    return new SnowflakeDateTimeFormat(sqlFormat, type, true);
  }

  private SnowflakeDateTimeFormat(String sqlFormat, int type, boolean isDeprecated) {
    // limit sql format length
    if (sqlFormat.length() > 1024) {
      throw new IllegalArgumentException("timestamp format too long");
    }

    this.sqlFormat = sqlFormat;
    this.type = type;
    this.isDeprecated = isDeprecated;
    fragments = new ArrayList<>();
    if (sqlFormat.compareToIgnoreCase("auto") == 0) {
      automaticParsing = true;
    } else {
      automaticParsing = false;
      compile(sqlFormat);
      assert fragments.size() <= 1;
      simpleDateFormat = new SimpleDateFormat(toSimpleDateTimePattern());
    }
  }

  public String getSqlFormat() {
    return sqlFormat;
  }

  private void createNewFragment(String javaTimestampFormat) {
    fragments.add(new Fragment(javaTimestampFormat));
  }

  /**
   * Add the java format for the element to the java format string builder Add the element to the
   * list of elements.
   *
   * @param element
   * @param javaTimestampFormat
   * @param elementTypes
   * @return Return the length of the sql format corresponding to the element.
   */
  private int addElement(
      ElementType element, StringBuilder javaTimestampFormat, List<ElementType> elementTypes) {
    javaTimestampFormat.append(element.getJavaFormat());
    elementTypes.add(element);
    return element.getSqlFormat().length();
  }

  /**
   * Adds a raw character to a string format by quoting it
   *
   * @param stringFormat
   * @param charToAdd
   */
  private void addRawChar(StringBuilder stringFormat, char charToAdd) {
    int curSize = stringFormat.length();
    if (charToAdd == '\'') {
      // Special code for "'"
      stringFormat.append("''");
    } else if (curSize > 2
        && stringFormat.charAt(curSize - 1) == '\''
        && stringFormat.charAt(curSize - 2) != '\'') {
      // Previous character was "raw", combine them
      if (fractionsPos == curSize) {
        // We're deleting a character before fractions, need to adjust for that
        fractionsPos--;
      }
      stringFormat.deleteCharAt(curSize - 1);
      stringFormat.append(charToAdd).append('\'');
    } else {
      stringFormat.append('\'').append(charToAdd).append('\'');
    }
  }

  /**
   * A function to parse SQL timestamp format and generate timestamp fragments.
   *
   * @param sqlTimestampFormat
   */
  private void compile(String sqlTimestampFormat) {
    StringBuilder javaTimestampFormat = new StringBuilder();
    List<ElementType> elementTypes = new ArrayList<>();

    int idx = 0;

    String formatUpperCase = sqlTimestampFormat.toUpperCase();

    while (idx < formatUpperCase.length()) {
      switch (formatUpperCase.charAt(idx)) {
        case 'A':
          if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.Ante_Meridiem_ElementType.getSqlFormat())) {
            idx +=
                addElement(
                    ElementType.Ante_Meridiem_ElementType, javaTimestampFormat, elementTypes);
          } else {
            addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
          }
          break;

        case 'D':
          if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.DayOfMonth_ElementType.getSqlFormat())) {
            idx +=
                addElement(ElementType.DayOfMonth_ElementType, javaTimestampFormat, elementTypes);
          } else if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.DayOfWeekAbbrev_ElementType.getSqlFormat())) {
            idx +=
                addElement(
                    ElementType.DayOfWeekAbbrev_ElementType, javaTimestampFormat, elementTypes);
          } else {
            addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
          }
          break;

        case 'H':
          if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.Hour24_ElementType.getSqlFormat())) {
            idx += addElement(ElementType.Hour24_ElementType, javaTimestampFormat, elementTypes);
          } else if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.Hour12_ElementType.getSqlFormat())) {
            idx += addElement(ElementType.Hour12_ElementType, javaTimestampFormat, elementTypes);
          } else if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.Hour_ElementType.getSqlFormat())) {
            idx += addElement(ElementType.Hour_ElementType, javaTimestampFormat, elementTypes);
          } else {
            addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
          }
          break;

        case 'M':
          if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.MonthFullName_ElementType.getSqlFormat())) {
            idx +=
                addElement(
                    ElementType.MonthFullName_ElementType, javaTimestampFormat, elementTypes);
          } else if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.Month_ElementType.getSqlFormat())) {
            idx += addElement(ElementType.Month_ElementType, javaTimestampFormat, elementTypes);
          } else if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.Minute_ElementType.getSqlFormat())) {
            idx += addElement(ElementType.Minute_ElementType, javaTimestampFormat, elementTypes);
          } else if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.MonthAbbrev_ElementType.getSqlFormat())) {
            idx +=
                addElement(ElementType.MonthAbbrev_ElementType, javaTimestampFormat, elementTypes);
          } else {
            addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
          }
          break;

        case 'P':
          if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.Post_Meridiem_ElementType.getSqlFormat())) {
            idx +=
                addElement(
                    ElementType.Post_Meridiem_ElementType, javaTimestampFormat, elementTypes);
          } else {
            addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
          }
          break;

        case 'S':
          if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.Second_ElementType.getSqlFormat())) {
            idx += addElement(ElementType.Second_ElementType, javaTimestampFormat, elementTypes);
          } else {
            addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
          }
          break;

        case 'T':
          {
            if (formatUpperCase
                .substring(idx)
                .startsWith(ElementType.TZOffsetHourColonMin_ElementType.getSqlFormat())) {
              timezonePreFormatter = new SimpleDateFormat(javaTimestampFormat.toString());
              timezoneElementType = ElementType.TZOffsetHourColonMin_ElementType;
              idx +=
                  addElement(
                      ElementType.TZOffsetHourColonMin_ElementType,
                      javaTimestampFormat,
                      elementTypes);
            } else if (formatUpperCase
                .substring(idx)
                .startsWith(ElementType.TZOffsetHourMin_ElementType.getSqlFormat())) {
              timezonePreFormatter = new SimpleDateFormat(javaTimestampFormat.toString());
              timezoneElementType = ElementType.TZOffsetHourMin_ElementType;
              idx +=
                  addElement(
                      ElementType.TZOffsetHourMin_ElementType, javaTimestampFormat, elementTypes);
            } else if (formatUpperCase
                .substring(idx)
                .startsWith(ElementType.TZOffsetHourOnly_ElementType.getSqlFormat())) {
              timezonePreFormatter = new SimpleDateFormat(javaTimestampFormat.toString());
              timezoneElementType = ElementType.TZOffsetHourOnly_ElementType;
              idx +=
                  addElement(
                      ElementType.TZOffsetHourOnly_ElementType, javaTimestampFormat, elementTypes);
            } else if (formatUpperCase
                .substring(idx)
                .startsWith(ElementType.TZAbbr_ElementType.getSqlFormat())) {
              timezonePreFormatter = new SimpleDateFormat(javaTimestampFormat.toString());
              timezoneElementType = ElementType.TZAbbr_ElementType;
              idx += addElement(ElementType.TZAbbr_ElementType, javaTimestampFormat, elementTypes);
            } else {
              addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
            }
          }
          break;

        case 'Y':
          // It's important 4-digit year goes before 2-digit year
          if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.Year_ElementType.getSqlFormat())) {
            idx += addElement(ElementType.Year_ElementType, javaTimestampFormat, elementTypes);
          } else if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.Year2digit_ElementType.getSqlFormat())) {
            has2digitYear = true;
            idx +=
                addElement(ElementType.Year2digit_ElementType, javaTimestampFormat, elementTypes);
          } else {
            addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
          }
          break;

        case '.':
          if (idx + 1 < formatUpperCase.length()
              && formatUpperCase
                  .substring(idx + 1)
                  .startsWith(ElementType.MilliSecond_ElementType.getSqlFormat())) {
            // Will be FF, just mark that there's a dot before FF
            fractionsWithDot = true;
            idx++;
          } else {
            addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
          }
          break;

        case 'F':
          if (formatUpperCase
              .substring(idx)
              .startsWith(ElementType.MilliSecond_ElementType.getSqlFormat())) {
            idx += ElementType.MilliSecond_ElementType.getSqlFormat().length();
            // @todo Handle multiple occurences?
            // Construct formatter to find fractions position.
            fractionsPreFormatter = new SimpleDateFormat(javaTimestampFormat.toString());
            // Save fractions information
            fractionsPos = javaTimestampFormat.toString().length();
            fractionsLen = -1;
            // Check if FF is followed by the length specification (e.g. FF3)
            if (idx < formatUpperCase.length() && Character.isDigit(formatUpperCase.charAt(idx))) {
              fractionsLen = Character.digit(formatUpperCase.charAt(idx), 10);
              idx++;
            }
          } else {
            addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
          }
          break;

        case '\"':
          // two double quotes become a single double quote;
          // in all other cases, replace double quotes with single quotes;
          // single quotes are Java's way of quoting things in a datetime format string

          int endIdx = idx + 1;

          while (endIdx < sqlTimestampFormat.length()
              && sqlTimestampFormat.charAt(endIdx) != '\"') {
            endIdx++;
          }

          if (endIdx == sqlTimestampFormat.length()) {
            throw new IllegalArgumentException("Unterminated '\"'");
          }

          if (endIdx == idx + 1) {
            // two double quotes = a single double quote
            javaTimestampFormat.append("\"");
          } else {
            // replace double quote with a single quote
            javaTimestampFormat.append("'");
            javaTimestampFormat.append(sqlTimestampFormat, idx + 1, endIdx);
            javaTimestampFormat.append("'");
          }

          idx = endIdx + 1;

          break;

        default:
          addRawChar(javaTimestampFormat, sqlTimestampFormat.charAt(idx++));
          break;
      }
    }

    if (!elementTypes.isEmpty() || javaTimestampFormat.length() > 0 || fractionsLen > 0) {
      createNewFragment(javaTimestampFormat.toString());
    }
  }

  public final String toSimpleDateTimePattern() {
    if (automaticParsing) {
      // This is only supposed to happen in logging.
      return "AUTO";
    }

    if (fragments.size() == 0) {
      return "";
    } else if (fragments.size() == 1) {
      return fragments.get(0).javaFormat;
    } else {
      return fragments.get(0).javaFormat + "FFF" + fragments.get(1).javaFormat;
    }
  }

  public String format(Timestamp timestamp, String timeZoneId, int scale) {
    return format(timestamp, (timeZoneId == null) ? GMT : TimeZone.getTimeZone(timeZoneId), scale);
  }

  public String format(Timestamp timestamp, TimeZone timeZone, int scale) {
    return format(timestamp, timeZone, timestamp.getNanos(), scale);
  }

  public String format(java.util.Date date, String timeZoneId) {
    return format(date, (timeZoneId == null) ? GMT : TimeZone.getTimeZone(timeZoneId));
  }

  public String format(java.util.Date date, TimeZone timeZone) {
    return format(date, timeZone, 0, 0);
  }

  public String format(LocalTime localTime, int scale) {
    return format(new Time(localTime.toNanoOfDay() / 1_000_000L), GMT, localTime.getNano(), scale);
  }

  // Private function performing actual formatting
  private String format(java.util.Date timestampOrDate, TimeZone timeZone, int nanos, int scale) {
    SimpleDateFormat formatter;

    if (fractionsPos >= 0) { // Construct a special formatter, with nanos embedded
      assert fragments.size() <= 1;
      if (fractionsLen >= 0) {
        scale = fractionsLen;
      }
      String nanoStr = String.format("%1$09d", nanos).substring(0, scale);
      if (fractionsWithDot) {
        nanoStr = "." + nanoStr;
      }
      String newDateFormat;
      if (fragments.size() > 0) {
        String oldFormat = this.fragments.get(0).javaFormat;
        newDateFormat =
            oldFormat.substring(0, fractionsPos) + nanoStr + oldFormat.substring(fractionsPos);
      } else {
        newDateFormat = nanoStr;
      }
      formatter = new SimpleDateFormat(newDateFormat);
    } else {
      if (simpleDateFormat == null) {
        throw new IllegalArgumentException(
            "formatter is null. automaticParsing: " + automaticParsing);
      }
      formatter = simpleDateFormat;
    }
    formatter.setCalendar(new GregorianCalendar(timeZone));
    return formatter.format(timestampOrDate);
  }
}
