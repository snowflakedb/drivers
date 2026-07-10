package net.snowflake.client.internal.api.implementation.metadata.capabilities;

import java.sql.SQLException;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.api.driver.SnowflakeDriver;
import net.snowflake.client.internal.api.implementation.connection.InternalSnowflakeConnection;
import net.snowflake.client.internal.api.implementation.connection.SnowflakeConnectionImpl;
import net.snowflake.client.internal.api.implementation.parameters.SessionProperty;

@RequiredArgsConstructor
public final class MetaDataIdentity {
  private static final String DATABASE_PRODUCT_NAME = "Snowflake";
  private static final char SEARCH_STRING_ESCAPE = '\\';

  // Open Group CLI Functions; LOG10 is not supported.
  static final String NUMERIC_FUNCTIONS_SUPPORTED =
      "ABS,ACOS,ASIN,ATAN,ATAN2,CBRT,CEILING,COS,COT,DEGREES,EXP,FACTORIAL,"
          + "FLOOR,HAVERSINE,LN,LOG,MOD,PI,POWER,RADIANS,RAND,"
          + "ROUND,SIGN,SIN,SQRT,SQUARE,TAN,TRUNCATE";

  // DIFFERENCE and SOUNDEX are not supported.
  static final String STRING_FUNCTIONS_SUPPORTED =
      "ASCII,BIT_LENGTH,CHAR,CONCAT,INSERT,LCASE,LEFT,LENGTH,LPAD,"
          + "LOCATE,LTRIM,OCTET_LENGTH,PARSE_IP,PARSE_URL,REPEAT,REVERSE,"
          + "REPLACE,RPAD,RTRIMMED_LENGTH,SPACE,SPLIT,SPLIT_PART,"
          + "SPLIT_TO_TABLE,STRTOK,STRTOK_TO_ARRAY,STRTOK_SPLIT_TO_TABLE,"
          + "TRANSLATE,TRIM,UNICODE,UUID_STRING,INITCAP,LOWER,UPPER,REGEXP,"
          + "REGEXP_COUNT,REGEXP_INSTR,REGEXP_LIKE,REGEXP_REPLACE,"
          + "REGEXP_SUBSTR,RLIKE,CHARINDEX,CONTAINS,EDITDISTANCE,ENDSWITH,"
          + "ILIKE,ILIKE ANY,LIKE,LIKE ALL,LIKE ANY,POSITION,REPLACE,RIGHT,"
          + "STARTSWITH,SUBSTRING,COMPRESS,DECOMPRESS_BINARY,DECOMPRESS_STRING,"
          + "BASE64_DECODE_BINARY,BASE64_DECODE_STRING,BASE64_ENCODE,"
          + "HEX_DECODE_BINARY,HEX_DECODE_STRING,HEX_ENCODE,"
          + "TRY_BASE64_DECODE_BINARY,TRY_BASE64_DECODE_STRING,"
          + "TRY_HEX_DECODE_BINARY,TRY_HEX_DECODE_STRING,MD_5,MD5_HEX,"
          + "MD5_BINARY,SHA1,SHA1_HEX,SHA2,SHA1_BINARY,SHA2_HEX,SHA2_BINARY,"
          + " HASH,HASH_AGG,COLLATE,COLLATION";

  static final String DATE_AND_TIME_FUNCTIONS_SUPPORTED =
      "CURDATE,"
          + "CURTIME,DAYNAME,DAYOFMONTH,DAYOFWEEK,DAYOFYEAR,HOUR,MINUTE,MONTH,"
          + "MONTHNAME,NOW,QUARTER,SECOND,TIMESTAMPADD,TIMESTAMPDIFF,WEEK,YEAR";

  static final String SYSTEM_FUNCTIONS_SUPPORTED = "DATABASE,IFNULL,USER";

  // Keywords supported by Snowflake but not in the SQL:2003 standard.
  static final String NOT_SQL2003_KEYWORDS =
      String.join(
          ",",
          "ACCOUNT",
          "ASOF",
          "BIT",
          "BYTEINT",
          "CONNECTION",
          "DATABASE",
          "DATETIME",
          "DATE_PART",
          "FIXED",
          "FOLLOWING",
          "GSCLUSTER",
          "GSPACKAGE",
          "IDENTIFIER",
          "ILIKE",
          "INCREMENT",
          "ISSUE",
          "LONG",
          "MAP",
          "MATCH_CONDITION",
          "MINUS",
          "NUMBER",
          "OBJECT",
          "ORGANIZATION",
          "QUALIFY",
          "REFERENCE",
          "REGEXP",
          "RLIKE",
          "SAMPLE",
          "SCHEMA",
          "STRING",
          "TEXT",
          "TIMESTAMPLTZ",
          "TIMESTAMPNTZ",
          "TIMESTAMPTZ",
          "TIMESTAMP_LTZ",
          "TIMESTAMP_NTZ",
          "TIMESTAMP_TZ",
          "TINYINT",
          "TRANSIT",
          "TRY_CAST",
          "VARIANT",
          "VECTOR",
          "VIEW");

  private final InternalSnowflakeConnection connection;

  public String getURL() throws SQLException {
    connection.checkClosed();
    return connection.getParameters().getOrThrow(SessionProperty.URL);
  }

  public String getUserName() throws SQLException {
    connection.checkClosed();
    // USER is probably always populated during connection setup, but the spec allows null in theory
    return connection.getParameters().get(SessionProperty.USER, null);
  }

  public String getDatabaseProductName() throws SQLException {
    connection.checkClosed();
    return DATABASE_PRODUCT_NAME;
  }

  public String getDatabaseProductVersion() throws SQLException {
    connection.checkClosed();
    return connection.getDatabaseVersion();
  }

  public String getDriverName() throws SQLException {
    connection.checkClosed();
    return SnowflakeDriver.DRIVER_NAME;
  }

  public String getDriverVersion() throws SQLException {
    connection.checkClosed();
    return SnowflakeDriver.DRIVER_VERSION;
  }

  public int getDriverMajorVersion() {
    return SnowflakeDriver.MAJOR_VERSION;
  }

  public int getDriverMinorVersion() {
    return SnowflakeDriver.MINOR_VERSION;
  }

  public String getIdentifierQuoteString() throws SQLException {
    connection.checkClosed();
    return "\"";
  }

  public String getSQLKeywords() throws SQLException {
    connection.checkClosed();
    return NOT_SQL2003_KEYWORDS;
  }

  public String getNumericFunctions() throws SQLException {
    connection.checkClosed();
    return NUMERIC_FUNCTIONS_SUPPORTED;
  }

  public String getStringFunctions() throws SQLException {
    connection.checkClosed();
    return STRING_FUNCTIONS_SUPPORTED;
  }

  public String getSystemFunctions() throws SQLException {
    connection.checkClosed();
    return SYSTEM_FUNCTIONS_SUPPORTED;
  }

  public String getTimeDateFunctions() throws SQLException {
    connection.checkClosed();
    return DATE_AND_TIME_FUNCTIONS_SUPPORTED;
  }

  public String getSearchStringEscape() throws SQLException {
    connection.checkClosed();
    return Character.toString(SEARCH_STRING_ESCAPE);
  }

  public String getExtraNameCharacters() throws SQLException {
    connection.checkClosed();
    return "$";
  }

  public String getSchemaTerm() throws SQLException {
    connection.checkClosed();
    return "schema";
  }

  public String getProcedureTerm() throws SQLException {
    connection.checkClosed();
    return "procedure";
  }

  public String getCatalogTerm() throws SQLException {
    connection.checkClosed();
    return "database";
  }

  public String getCatalogSeparator() throws SQLException {
    connection.checkClosed();
    return ".";
  }

  public int getDatabaseMajorVersion() throws SQLException {
    connection.checkClosed();
    return connection.unwrap(SnowflakeConnectionImpl.class).getDatabaseMajorVersion();
  }

  public int getDatabaseMinorVersion() throws SQLException {
    connection.checkClosed();
    return connection.unwrap(SnowflakeConnectionImpl.class).getDatabaseMinorVersion();
  }

  public int getJDBCMajorVersion() throws SQLException {
    connection.checkClosed();
    return SnowflakeDriver.JDBC_SPEC_MAJOR;
  }

  public int getJDBCMinorVersion() throws SQLException {
    connection.checkClosed();
    return SnowflakeDriver.JDBC_SPEC_MINOR;
  }
}
