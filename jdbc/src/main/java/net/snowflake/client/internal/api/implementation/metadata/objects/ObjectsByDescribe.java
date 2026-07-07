package net.snowflake.client.internal.api.implementation.metadata.objects;

import static net.snowflake.client.internal.api.implementation.metadata.objects.ErrorUtils.isMissingMetadataObject;
import static net.snowflake.client.internal.api.implementation.metadata.objects.ErrorUtils.isSyntaxError;
import static net.snowflake.client.internal.api.implementation.metadata.objects.MatchingUtils.isPatternMatchingAll;
import static net.snowflake.client.internal.util.SnowflakeTypeHelper.convertStringToType;
import static net.snowflake.client.internal.util.StringUtil.isNullOrEmpty;

import java.sql.DatabaseMetaData;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.sql.Types;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Objects;
import java.util.regex.Pattern;
import lombok.RequiredArgsConstructor;
import net.snowflake.client.internal.api.implementation.metadata.capabilities.MetaDataLimits;
import net.snowflake.common.util.Wildcard;

@RequiredArgsConstructor
class ObjectsByDescribe {

  private final MetaDataLimits limits;
  private final Statement stmt;
  private final String sqlQuery;

  // TODO(SNOW-3695645): consider replacing that with a streaming approach
  //  getProcedureColumns is intrinsically 1:N: each SHOW PROCEDURES row expands into one output
  //  row per parameter/result column. The streaming RowConverter path is 1:1, so we materialize
  //  eagerly here (like in the snowflake--jdbc).
  Object[][] showAndDescribeProcedures(
      String catalog, String schemaPattern, String procedureNamePattern, String columnNamePattern)
      throws SQLException {
    Pattern compiledSchemaPattern = Wildcard.toRegexPattern(schemaPattern, true);
    Pattern compiledProcedurePattern = Wildcard.toRegexPattern(procedureNamePattern, true);
    List<Object[]> rows = new ArrayList<>();
    ResultSet rs = executeShowQuery();
    if (rs == null) {
      return new Object[0][];
    }
    while (rs.next()) {
      // Divergence from snowflake-jdbc: null-guard getString() before .trim() to avoid NPE
      String procedureNameUnparsed = Objects.toString(rs.getString("arguments"), "").trim();
      String procedureNameNoArgs = rs.getString("name");
      String schemaName = rs.getString("schema_name");
      String catalogName = rs.getString("catalog_name");
      String remarks = Objects.toString(rs.getString("description"), "").trim();

      if (!MatchingUtils.matches(compiledProcedurePattern, procedureNameNoArgs)
          || (!MatchingUtils.matches(compiledSchemaPattern, schemaName)
              // TODO(SNOW-3695645): why this is the only place where we do this un-escaping
              && !(schemaName.startsWith("\"")
                  && schemaName.endsWith("\"")
                  && compiledSchemaPattern
                      .matcher(schemaName)
                      .region(1, schemaName.length() - 1)
                      .matches()))) {
        continue;
      }

      ParsedParams params =
          describeAndParseParams(catalogName, schemaName, procedureNameUnparsed, "procedure");
      if (params == null) {
        continue;
      }

      for (int i = 0; i < params.names.length; i++) {
        // if it's the 1st in for loop, it's the result
        if (i == 0
            || params.names[i].equalsIgnoreCase(columnNamePattern)
            || isPatternMatchingAll(columnNamePattern)) {
          Object[] nextRow = new Object[20];
          nextRow[0] = catalog;
          nextRow[1] = schemaName;
          nextRow[2] = procedureNameNoArgs;
          nextRow[3] = params.names[i];
          if (i == 0 && params.resultsetColumnNum < 0) {
            nextRow[4] = DatabaseMetaData.procedureColumnReturn;
          } else if (params.resultsetColumnNum >= 0 && i < params.resultsetColumnNum) {
            nextRow[4] = DatabaseMetaData.procedureColumnResult;
          } else {
            nextRow[4] = DatabaseMetaData.procedureColumnIn;
          }
          String typeName = params.types[i];
          String typeNameTrimmed = typeName;
          if (typeName.contains(" NOT NULL")) {
            typeNameTrimmed = typeName.substring(0, typeName.indexOf(' '));
          }
          if (typeNameTrimmed.contains("(") && typeNameTrimmed.contains(")")) {
            typeNameTrimmed = typeNameTrimmed.substring(0, typeNameTrimmed.indexOf('('));
          }
          int type = convertStringToType(typeName);
          nextRow[5] = type;
          nextRow[6] = typeNameTrimmed;
          if (type < 10) {
            nextRow[7] = 38;
            nextRow[9] = (short) 0;
            // Divergence from snowflake-jdbc: handle comma-less types like CHAR(n) that
            // have precision but no scale, instead of unconditionally splitting on ','
            if (typeName.contains("(") && typeName.contains(")")) {
              int commaIdx = typeName.indexOf(',');
              if (commaIdx >= 0) {
                nextRow[7] =
                    Integer.parseInt(typeName.substring(typeName.indexOf('(') + 1, commaIdx));
                nextRow[9] =
                    Short.parseShort(typeName.substring(commaIdx + 1, typeName.indexOf(')')));
              } else {
                nextRow[7] =
                    Integer.parseInt(
                        typeName.substring(typeName.indexOf('(') + 1, typeName.indexOf(')')));
              }
            }
          } else {
            nextRow[7] = 0;
            nextRow[9] = null;
          }
          nextRow[8] = 0;
          nextRow[10] = 10;
          if (typeName.toLowerCase().contains("not null")) {
            nextRow[11] = DatabaseMetaData.procedureNoNulls;
            nextRow[18] = "NO";
          } else if (i == 0) {
            nextRow[11] = DatabaseMetaData.procedureNullable;
            nextRow[18] = "YES";
          } else {
            nextRow[11] = DatabaseMetaData.procedureNullableUnknown;
            nextRow[18] = "";
          }
          nextRow[12] = remarks;
          nextRow[13] = null;
          nextRow[14] = 0;
          nextRow[15] = 0;
          if (type == Types.BINARY
              || type == Types.VARBINARY
              || type == Types.CHAR
              || type == Types.VARCHAR) {
            if (typeName.contains("(") && typeName.contains(")")) {
              int charOctetLen =
                  Integer.parseInt(
                      typeName.substring(typeName.indexOf('(') + 1, typeName.indexOf(')')));
              nextRow[16] = charOctetLen;
            } else if (type == Types.CHAR || type == Types.VARCHAR) {
              nextRow[16] = limits.getMaxCharLiteralLength();
            } else if (type == Types.BINARY || type == Types.VARBINARY) {
              nextRow[16] = limits.getMaxBinaryLiteralLength();
            }
          } else {
            nextRow[16] = null;
          }
          if (params.resultsetColumnNum >= 0) {
            if (i < params.resultsetColumnNum) {
              nextRow[17] = i + 1;
            } else {
              nextRow[17] = i - params.resultsetColumnNum + 1;
            }
          } else {
            nextRow[17] = i;
          }
          nextRow[19] = procedureNameUnparsed;
          rows.add(nextRow);
        }
      }
    }
    return rows.toArray(new Object[0][]);
  }

  // TODO(SNOW-3695645): consider replacing that with a streaming approach
  //  getFunctionColumns is intrinsically 1:N: each SHOW FUNCTIONS row expands into one output row
  //  per parameter/result column. The streaming RowConverter path is 1:1, so we materialize
  //  eagerly here (like in the snowflake--jdbc).
  Object[][] showAndDescribeFunctions(
      String catalog, String schemaPattern, String functionNamePattern, String columnNamePattern)
      throws SQLException {
    List<Object[]> rows = new ArrayList<>();
    ResultSet rs = executeShowQuery();
    if (rs == null) {
      return new Object[0][];
    }
    while (rs.next()) {
      // Divergence from snowflake-jdbc: null-guard getString() before .trim() to avoid NPE
      String functionNameUnparsed = Objects.toString(rs.getString("arguments"), "").trim();
      String functionNameNoArgs = rs.getString("name");
      String schemaName = rs.getString("schema_name");
      String catalogName = rs.getString("catalog_name");
      String remarks = Objects.toString(rs.getString("description"), "").trim();

      // TODO(SNOW-3695645): why don't we filter by functionNamePattern like in getProcedureColumns

      ParsedParams params =
          describeAndParseParams(catalogName, schemaName, functionNameUnparsed, "function");
      if (params == null) {
        continue;
      }

      for (int i = 0; i < params.names.length; i++) {
        if (i == 0
            || params.names[i].equalsIgnoreCase(columnNamePattern)
            || isPatternMatchingAll(columnNamePattern)) {
          Object[] nextRow = new Object[17];
          nextRow[0] = catalog;
          // Divergence from snowflake-jdbc: legacy used schemaPattern here (pre-existing bug),
          // we use the actual schema from the SHOW result row
          nextRow[1] = schemaName;
          nextRow[2] = functionNameNoArgs;
          nextRow[3] = params.names[i];
          if (i == 0 && params.resultsetColumnNum < 0) {
            nextRow[4] = DatabaseMetaData.functionReturn;
          } else if (params.resultsetColumnNum >= 0 && i < params.resultsetColumnNum) {
            nextRow[4] = DatabaseMetaData.functionColumnResult;
          } else {
            nextRow[4] = DatabaseMetaData.functionColumnIn;
          }
          String typeName = params.types[i];
          int type = convertStringToType(typeName);
          nextRow[5] = type;
          // TODO(SNOW-3695645): why isn't typeName trimmed like for getProcedureColumns?
          nextRow[6] = typeName;
          if (type < 10) {
            nextRow[7] = 38;
            nextRow[9] = (short) 0;
            if (typeName.contains("(") && typeName.contains(")")) {
              // Divergence from snowflake-jdbc: handle comma-less types like CHAR(n) that
              // have precision but no scale, instead of unconditionally splitting on ','
              int commaIdx = typeName.indexOf(',');
              if (commaIdx >= 0) {
                nextRow[7] =
                    Integer.parseInt(typeName.substring(typeName.indexOf('(') + 1, commaIdx));
                nextRow[9] =
                    Short.parseShort(typeName.substring(commaIdx + 1, typeName.indexOf(')')));
              } else {
                nextRow[7] =
                    Integer.parseInt(
                        typeName.substring(typeName.indexOf('(') + 1, typeName.indexOf(')')));
              }
            } else if (type == Types.FLOAT) {
              // TODO(SNOW-3695645): this branch is not present in getProcedureColumns
              nextRow[7] = 0;
              nextRow[9] = null;
            }
          } else {
            nextRow[7] = 0;
            nextRow[9] = null;
          }
          nextRow[8] = 0;
          nextRow[10] = 10;
          nextRow[11] = DatabaseMetaData.functionNullableUnknown;
          nextRow[12] = remarks;
          if (type == Types.BINARY
              || type == Types.VARBINARY
              || type == Types.CHAR
              || type == Types.VARCHAR) {
            if (typeName.contains("(") && typeName.contains(")")) {
              int charOctetLen =
                  Integer.parseInt(
                      typeName.substring(typeName.indexOf('(') + 1, typeName.indexOf(')')));
              nextRow[13] = charOctetLen;
            } else if (type == Types.CHAR || type == Types.VARCHAR) {
              nextRow[13] = limits.getMaxCharLiteralLength();
            } else if (type == Types.BINARY || type == Types.VARBINARY) {
              nextRow[13] = limits.getMaxBinaryLiteralLength();
            }
          } else {
            nextRow[13] = null;
          }
          if (params.resultsetColumnNum >= 0) {
            if (i < params.resultsetColumnNum) {
              nextRow[14] = i + 1;
            } else {
              nextRow[14] = i - params.resultsetColumnNum + 1;
            }
          } else {
            nextRow[14] = i;
          }
          nextRow[15] = "";
          nextRow[16] = functionNameUnparsed;
          rows.add(nextRow);
        }
      }
    }
    return rows.toArray(new Object[0][]);
  }

  private ParsedParams describeAndParseParams(
      String catalog, String schema, String nameUnparsed, String type) throws SQLException {
    String sql = buildDescribeCommand(catalog, schema, nameUnparsed, type);
    try (ResultSet rs = describe(sql, type)) {
      if (rs == null || !rs.next()) {
        return null;
      }
      String args = rs.getString("value");
      rs.next();
      String res = rs.getString("value");
      return parseColumns(res, args);
    }
  }

  private ResultSet describe(String sql, String routineType) throws SQLException {
    try {
      return stmt.executeQuery(sql);
    } catch (Throwable e) {
      // Legacy behavior: only swallow syntax errors for functions, as some have odd signatures
      // that fail DESCRIBE. For procedures, surface the error so bugs aren't silently masked.
      if ("function".equals(routineType) && isSyntaxError(e)) {
        return null;
      }
      if (isMissingMetadataObject(e)) {
        return null;
      }
      throw e;
    }
  }

  private ResultSet executeShowQuery() throws SQLException {
    try {
      return stmt.executeQuery(sqlQuery);
    } catch (Throwable e) {
      if (isMissingMetadataObject(e)) {
        return null;
      }
      throw e;
    }
  }

  private static final class ParsedParams {
    final String[] names;
    final String[] types;
    final int resultsetColumnNum;

    private ParsedParams(String[] names, String[] types, int resultsetColumnNum) {
      this.names = names;
      this.types = types;
      this.resultsetColumnNum = resultsetColumnNum;
    }
  }

  /**
   * Parses DESCRIBE output into parallel name/type arrays. The interleaved list has names at even
   * indices and types at odd indices. For table-returning routines, {@code resultsetColumnNum} is
   * the count of result-set columns; -1 for scalar return values.
   */
  private static ParsedParams parseColumns(String retType, String args) {
    List<String> columns = new ArrayList<>();
    int resultsetColumnNum = -1;
    // Divergence from snowflake-jdbc: use regionMatches instead of substring(0,5) to avoid
    // StringIndexOutOfBoundsException on short return types like DATE, TIME, REAL
    if (retType.regionMatches(true, 0, "table", 0, 5)) {
      String typeStr = retType.substring(retType.indexOf('(') + 1, retType.lastIndexOf(')'));
      String[] types = typeStr.split("\\s+|, ");
      if (types.length != 1) {
        Collections.addAll(columns, types);
        resultsetColumnNum = columns.size() / 2;
      }
    } else {
      columns.add("");
      columns.add(retType);
    }
    String argStr = args.substring(args.indexOf('(') + 1, args.lastIndexOf(')'));
    String[] arguments = argStr.split("\\s+|, ");
    if (arguments.length != 1) {
      Collections.addAll(columns, arguments);
    }
    String[] names = new String[columns.size() / 2];
    String[] types = new String[columns.size() / 2];
    for (int i = 0; i < columns.size(); i++) {
      if (i % 2 == 0) {
        names[i / 2] = columns.get(i);
      } else {
        types[i / 2] = columns.get(i);
      }
    }
    return new ParsedParams(names, types, resultsetColumnNum);
  }

  private static String buildDescribeCommand(
      String catalog, String schema, String unparsedName, String routineType) {
    if (isNullOrEmpty(unparsedName)) {
      return "";
    }
    String paramSignature =
        unparsedName.substring(unparsedName.indexOf("("), unparsedName.indexOf(" RETURN"));
    String quotedName = "\"" + unparsedName.substring(0, unparsedName.indexOf("(")) + "\"";
    String qualifiedName = quotedName + paramSignature;
    if (!isNullOrEmpty(catalog) && !isNullOrEmpty(schema)) {
      return "desc " + routineType + " " + catalog + "." + schema + "." + qualifiedName;
    } else if (!isNullOrEmpty(schema)) {
      return "desc " + routineType + " " + schema + "." + qualifiedName;
    }
    return "desc " + routineType + " " + qualifiedName;
  }
}
