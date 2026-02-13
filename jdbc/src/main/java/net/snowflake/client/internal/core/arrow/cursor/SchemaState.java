package net.snowflake.client.internal.core.arrow.cursor;

import java.sql.SQLException;
import java.sql.Types;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.core.arrow.converters.ArrowVectorConverter;
import net.snowflake.client.internal.core.arrow.converters.ArrowVectorConverterUtil;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;
import org.apache.arrow.vector.FieldVector;
import org.apache.arrow.vector.VectorSchemaRoot;
import org.apache.arrow.vector.types.pojo.Field;

public final class SchemaState {
  private static final DataConversionContext EMPTY_CONTEXT = new DataConversionContext() {};

  private String[] columnNames;
  private int[] columnTypes;
  private ArrowVectorConverter[] converterCache;

  public SchemaState(VectorSchemaRoot root) throws SQLException {
    List<Field> fields = root.getSchema().getFields();
    columnNames = new String[fields.size()];
    columnTypes = new int[fields.size()];
    converterCache = new ArrowVectorConverter[fields.size()];
    for (int i = 0; i < fields.size(); i++) {
      Field field = fields.get(i);
      columnNames[i] = field.getName();
      SnowflakeType logicalType = ArrowVectorConverterUtil.getSnowflakeTypeFromFieldMetadata(field);
      columnTypes[i] = mapLogicalTypeToSqlType(logicalType);
    }
  }

  public String[] getColumnNames() {
    return columnNames;
  }

  public int[] getColumnTypes() {
    return columnTypes;
  }

  public int getColumnCount() {
    return columnNames.length;
  }

  public ArrowVectorConverter getConverter(int columnIndex, VectorSchemaRoot root)
      throws SQLException {
    int index = columnIndex - 1;
    if (index < 0 || index >= converterCache.length) {
      throw new SQLException("Invalid column index: " + columnIndex);
    }
    ArrowVectorConverter cached = converterCache[index];
    if (cached != null) {
      return cached;
    }
    try {
      FieldVector vector = root.getVector(index);
      ArrowVectorConverter converter =
          ArrowVectorConverterUtil.initConverter(vector, EMPTY_CONTEXT, index);
      converterCache[index] = converter;
      return converter;
    } catch (SnowflakeSQLException e) {
      throw new SQLException("Unable to create converter for column " + columnIndex, e);
    }
  }

  private void clearConverterCache() {
    if (converterCache != null) {
      Arrays.fill(converterCache, null);
    }
  }

  void resetConverterCache() throws SQLException {
    clearConverterCache();
  }

  public void reset() {
    clearConverterCache();
    converterCache = null;
    columnNames = null;
    columnTypes = null;
  }

  private int mapLogicalTypeToSqlType(SnowflakeType logicalType) {
    if (logicalType == null) {
      return Types.OTHER;
    }
    // TODO: Other types will be handled later
    switch (logicalType) {
      case TEXT:
      case CHAR:
      case VARIANT:
        return Types.VARCHAR;
      case FIXED:
        return Types.DECIMAL;
      case REAL:
        return Types.DOUBLE;
      case BOOLEAN:
        return Types.BOOLEAN;
      case BINARY:
        return Types.BINARY;
      default:
        return Types.OTHER;
    }
  }
}
