package net.snowflake.client.internal.util;

import static net.snowflake.client.api.resultset.SnowflakeType.GEOGRAPHY;

import java.sql.Types;
import java.util.Locale;
import java.util.Optional;
import javax.annotation.Nullable;
import lombok.Value;
import lombok.experimental.UtilityClass;
import net.snowflake.client.api.exception.ErrorCode;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;

/**
 * Single source of truth for mapping an internal Snowflake type name to the JDBC type, external
 * type name and {@link SnowflakeType} base.
 */
@UtilityClass
public class SnowflakeColumnTypes {

  /** Resolved JDBC type, external type name and {@link SnowflakeType} base for a column. */
  @Value
  public static class ColumnTypeInfo {
    int columnType;
    String extColTypeName;
    SnowflakeType snowflakeType;
  }

  /**
   * Parses an internal Snowflake type name into a {@link SnowflakeType}, returning {@code null} if
   * unknown.
   *
   * @param name the internal Snowflake type name (case-insensitive)
   * @return the matching {@link SnowflakeType}, or {@code null} if unrecognized or {@code null}
   */
  public static SnowflakeType fromStringOrNull(String name) {
    if (name == null) {
      return null;
    }
    try {
      return SnowflakeType.valueOf(name.toUpperCase(Locale.ROOT));
    } catch (IllegalArgumentException e) {
      return null;
    }
  }

  // ported from snowflake-jdbc
  public static ColumnTypeInfo getSnowflakeType(
      String internalColTypeName,
      String extColTypeName,
      @Nullable String udtOutputType,
      int fixedColType,
      boolean isStructuredType,
      boolean isVectorType) {
    SnowflakeType baseType = fromStringOrNull(internalColTypeName);
    if (baseType == null) {
      // Unknown Snowflake type (e.g. UUID) — report as OTHER with the actual type name
      return new ColumnTypeInfo(
          Types.OTHER,
          defaultIfNull(extColTypeName, internalColTypeName.toUpperCase(Locale.ROOT)),
          SnowflakeType.ANY);
    }
    ColumnTypeInfo columnTypeInfo;

    switch (baseType) {
      case TEXT:
        columnTypeInfo =
            new ColumnTypeInfo(Types.VARCHAR, defaultIfNull(extColTypeName, "VARCHAR"), baseType);
        break;
      case CHAR:
        columnTypeInfo =
            new ColumnTypeInfo(Types.CHAR, defaultIfNull(extColTypeName, "CHAR"), baseType);
        break;
      case INTEGER:
        columnTypeInfo =
            new ColumnTypeInfo(Types.INTEGER, defaultIfNull(extColTypeName, "INTEGER"), baseType);
        break;
      case DECFLOAT:
        columnTypeInfo = new ColumnTypeInfo(Types.DECIMAL, "DECFLOAT", baseType);
        break;
      case FIXED:
        if (isVectorType) {
          columnTypeInfo =
              new ColumnTypeInfo(Types.INTEGER, defaultIfNull(extColTypeName, "INTEGER"), baseType);
        } else {
          columnTypeInfo =
              new ColumnTypeInfo(fixedColType, defaultIfNull(extColTypeName, "NUMBER"), baseType);
        }
        break;

      case REAL:
        if (isVectorType) {
          columnTypeInfo =
              new ColumnTypeInfo(Types.FLOAT, defaultIfNull(extColTypeName, "FLOAT"), baseType);
        } else {
          columnTypeInfo =
              new ColumnTypeInfo(Types.DOUBLE, defaultIfNull(extColTypeName, "DOUBLE"), baseType);
        }
        break;

      case TIMESTAMP:
      case TIMESTAMP_LTZ:
        columnTypeInfo =
            new ColumnTypeInfo(
                SnowflakeType.EXTRA_TYPES_TIMESTAMP_LTZ,
                defaultIfNull(extColTypeName, "TIMESTAMPLTZ"),
                baseType);
        break;

      case INTERVAL_YEAR_MONTH:
        columnTypeInfo =
            new ColumnTypeInfo(
                SnowflakeType.EXTRA_TYPES_YEAR_MONTH_INTERVAL,
                defaultIfNull(extColTypeName, "INTERVAL_YEAR_MONTH"),
                baseType);
        break;

      case INTERVAL_DAY_TIME:
        columnTypeInfo =
            new ColumnTypeInfo(
                SnowflakeType.EXTRA_TYPES_DAY_TIME_INTERVAL,
                defaultIfNull(extColTypeName, "INTERVAL_DAY_TIME"),
                baseType);
        break;

      case TIMESTAMP_NTZ:
        // if the column type is changed to EXTRA_TYPES_TIMESTAMP_NTZ, update also JsonSqlInput
        columnTypeInfo =
            new ColumnTypeInfo(
                Types.TIMESTAMP, defaultIfNull(extColTypeName, "TIMESTAMPNTZ"), baseType);
        break;

      case TIMESTAMP_TZ:
        columnTypeInfo =
            new ColumnTypeInfo(
                SnowflakeType.EXTRA_TYPES_TIMESTAMP_TZ,
                defaultIfNull(extColTypeName, "TIMESTAMPTZ"),
                baseType);
        break;

      case DATE:
        columnTypeInfo =
            new ColumnTypeInfo(Types.DATE, defaultIfNull(extColTypeName, "DATE"), baseType);
        break;

      case TIME:
        columnTypeInfo =
            new ColumnTypeInfo(Types.TIME, defaultIfNull(extColTypeName, "TIME"), baseType);
        break;

      case BOOLEAN:
        columnTypeInfo =
            new ColumnTypeInfo(Types.BOOLEAN, defaultIfNull(extColTypeName, "BOOLEAN"), baseType);
        break;

      case VECTOR:
        columnTypeInfo =
            new ColumnTypeInfo(
                SnowflakeType.EXTRA_TYPES_VECTOR,
                defaultIfNull(extColTypeName, "VECTOR"),
                baseType);
        break;

      case ARRAY:
        int columnType = isStructuredType ? Types.ARRAY : Types.VARCHAR;
        columnTypeInfo =
            new ColumnTypeInfo(columnType, defaultIfNull(extColTypeName, "ARRAY"), baseType);
        break;

      case MAP:
        columnTypeInfo =
            new ColumnTypeInfo(Types.STRUCT, defaultIfNull(extColTypeName, "OBJECT"), baseType);
        break;

      case OBJECT:
        if (isStructuredType) {
          boolean isGeoType =
              "GEOMETRY".equals(extColTypeName) || "GEOGRAPHY".equals(extColTypeName);
          int type = isGeoType ? Types.VARCHAR : Types.STRUCT;
          columnTypeInfo =
              new ColumnTypeInfo(type, defaultIfNull(extColTypeName, "OBJECT"), baseType);
        } else {
          columnTypeInfo =
              new ColumnTypeInfo(Types.VARCHAR, defaultIfNull(extColTypeName, "OBJECT"), baseType);
        }
        break;

      case VARIANT:
        columnTypeInfo =
            new ColumnTypeInfo(Types.VARCHAR, defaultIfNull(extColTypeName, "VARIANT"), baseType);
        break;

      case BINARY:
        columnTypeInfo =
            new ColumnTypeInfo(Types.BINARY, defaultIfNull(extColTypeName, "BINARY"), baseType);
        break;

      case GEOGRAPHY:
      case GEOMETRY:
        int colType = Types.VARCHAR;
        extColTypeName = (baseType == GEOGRAPHY) ? "GEOGRAPHY" : "GEOMETRY";

        if (udtOutputType != null) {
          SnowflakeType outputType = fromStringOrNull(udtOutputType);
          if (outputType != null) {
            switch (outputType) {
              case OBJECT:
              case TEXT:
                colType = Types.VARCHAR;
                break;
              case BINARY:
                colType = Types.BINARY;
            }
          }
        }
        columnTypeInfo = new ColumnTypeInfo(colType, extColTypeName, baseType);
        break;

      default:
        // INTERNAL_ERROR carries the verbatim message (null template) — see SFSQLException.
        throw new SFSQLException(
            ErrorCode.INTERNAL_ERROR, "Unknown column type: " + internalColTypeName);
    }

    return columnTypeInfo;
  }

  private static String defaultIfNull(String extColTypeName, String defaultValue) {
    return Optional.ofNullable(extColTypeName).orElse(defaultValue);
  }

  public static boolean isVectorType(String internalColumnTypeName) {
    return internalColumnTypeName.equalsIgnoreCase("vector");
  }
}
