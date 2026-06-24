package net.snowflake.client.internal.core.arrow.cursor;

import java.sql.SQLException;
import java.time.LocalTime;
import java.util.Arrays;
import java.util.List;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.core.arrow.converters.ArrowVectorConverter;
import net.snowflake.client.internal.core.arrow.converters.ArrowVectorConverterUtil;
import net.snowflake.client.internal.core.arrow.converters.DataConversionContext;
import net.snowflake.client.internal.util.SnowflakeUtil;
import org.apache.arrow.vector.FieldVector;
import org.apache.arrow.vector.VectorSchemaRoot;
import org.apache.arrow.vector.types.pojo.Field;

public final class SchemaState {
  private static final DataConversionContext EMPTY_CONTEXT = new DataConversionContext() {};

  /**
   * Sample TIME used to measure the formatted display width of a TIME column: every field is
   * non-zero (12:34:56.123456789) so a scale-9 render exercises the widest possible output. Mirrors
   * snowflake-jdbc's {@code SFResultSetMetaData}.
   */
  private static final LocalTime TIME_WIDTH_SAMPLE = LocalTime.of(12, 34, 56, 123_456_789);

  private final DataConversionContext context;
  private String[] columnNames;
  private int[] columnTypes;
  private int[] columnScales;
  private ArrowVectorConverter[] converterCache;

  public SchemaState(VectorSchemaRoot root) throws SQLException {
    this(root, EMPTY_CONTEXT);
  }

  public SchemaState(VectorSchemaRoot root, DataConversionContext context) throws SQLException {
    this.context = context;
    List<Field> fields = root.getSchema().getFields();
    columnNames = new String[fields.size()];
    columnTypes = new int[fields.size()];
    columnScales = new int[fields.size()];
    converterCache = new ArrowVectorConverter[fields.size()];
    for (int i = 0; i < fields.size(); i++) {
      Field field = fields.get(i);
      columnNames[i] = field.getName();
      SnowflakeType logicalType = ArrowVectorConverterUtil.getSnowflakeTypeFromFieldMetadata(field);
      columnTypes[i] = SnowflakeUtil.toSqlType(logicalType);
      columnScales[i] = readScale(field);
    }
  }

  private static int readScale(Field field) {
    if (field.getMetadata() == null) {
      return 0;
    }
    String scaleStr = field.getMetadata().get("scale");
    if (scaleStr == null) {
      return 0;
    }
    try {
      return Integer.parseInt(scaleStr);
    } catch (NumberFormatException e) {
      return 0;
    }
  }

  public String[] getColumnNames() {
    return columnNames;
  }

  public int[] getColumnTypes() {
    return columnTypes;
  }

  public int[] getColumnScales() {
    return columnScales;
  }

  /**
   * Length of a TIME value formatted with the session {@code TIME_OUTPUT_FORMAT}, used for TIME
   * column precision/display size. Mirrors snowflake-jdbc's {@code SFResultSetMetaData}, which
   * formats a scale-9 sample time and takes its string length (8 for the default "HH24:MI:SS").
   */
  public int getTimeStringLength() {
    return context.getTimeFormatter().format(TIME_WIDTH_SAMPLE, 9).length();
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
          ArrowVectorConverterUtil.initConverter(vector, context, index);
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
    columnScales = null;
  }
}
