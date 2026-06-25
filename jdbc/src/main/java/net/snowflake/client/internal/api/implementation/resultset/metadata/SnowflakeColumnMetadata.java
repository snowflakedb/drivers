package net.snowflake.client.internal.api.implementation.resultset.metadata;

import static net.snowflake.client.internal.util.SnowflakeColumnTypes.getSnowflakeType;
import static net.snowflake.client.internal.util.SnowflakeColumnTypes.isVectorType;

import java.io.Serializable;
import java.sql.Types;
import java.util.Collections;
import java.util.List;
import lombok.Data;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.FieldMetadata;
import net.snowflake.client.api.resultset.SnowflakeType;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.util.SnowflakeColumnTypes;
import net.snowflake.client.internal.util.SnowflakeColumnTypes.ColumnTypeInfo;

@Data
class SnowflakeColumnMetadata implements Serializable {
  private static final long serialVersionUID = 1L;
  private String name;
  private String typeName;
  private int type;
  private boolean nullable;
  private int length;
  private int precision;
  private int scale;
  private boolean fixed;
  private SnowflakeType base;
  private List<FieldMetadata> fields;
  private String columnSrcTable;
  private String columnSrcSchema;
  private String columnSrcDatabase;

  private boolean isAutoIncrement;
  private int dimension; // vector type contains dimension

  /**
   * Builds column metadata from a protobuf {@link ColumnMetadata} description returned by the
   * native core.
   *
   * <p>The protobuf message carries only the basic per-column attributes (name, internal type name,
   * precision, scale, length, byte length and nullability). The JDBC type, external type name and
   * {@link SnowflakeType} base are derived from the internal type name using {@link
   * SnowflakeColumnTypes#getSnowflakeType}, reusing the mapping from the legacy driver.
   *
   * @param colMetadata the protobuf column metadata
   * @param jdbcTreatDecimalAsInt whether scale-0 fixed columns should be reported as {@link
   *     Types#BIGINT} instead of {@link Types#DECIMAL}
   */
  SnowflakeColumnMetadata(ColumnMetadata colMetadata, boolean jdbcTreatDecimalAsInt)
      throws SnowflakeSQLException {
    this.name = colMetadata.getName();
    this.nullable = colMetadata.getNullable();
    this.precision = (int) colMetadata.getPrecision();
    this.scale = (int) colMetadata.getScale();
    this.length = (int) colMetadata.getLength();
    // TODO(SNOW-3695645): !not ported! - dimension
    //    int dimension =
    //        colNode
    //            .path("dimension")
    //            .asInt(); // vector dimension when checking columns via connection.getMetadata
    //    int vectorDimension =
    //        colNode
    //            .path("vectorDimension")
    //            .asInt(); // dimension when checking columns via resultSet.getMetadata
    //    this.dimension = dimension > 0 ? dimension : vectorDimension;
    this.dimension = 0;

    // TODO(SNOW-3695645): !not ported! - `fixed` was read directly from the JSON node, here we
    //  derive it from the internal type name since the proto carries no `fixed` flag.
    //    this.fixed = colNode.path("fixed").asBoolean();
    String internalColTypeName = colMetadata.getType();
    this.fixed = SnowflakeType.FIXED == SnowflakeColumnTypes.fromStringOrNull(internalColTypeName);

    // TODO(SNOW-3695645): !not ported! - the proto carries no external type name (`extTypeName`)
    //  nor UDT `outputType`, so `getSnowflakeType` below cannot honor them and `typeName`
    //  falls back to the default per type.
    //    JsonNode udtOutputType = colNode.path("outputType");
    //    JsonNode extColTypeNameNode = colNode.path("extTypeName");
    //    String extColTypeName = null;
    //    if (!extColTypeNameNode.isMissingNode() && !isNullOrEmpty(extColTypeNameNode.asText())) {
    //      extColTypeName = extColTypeNameNode.asText();
    //    }
    List<FieldMetadata> fieldsMetadata = Collections.emptyList();
    int fixedColType = jdbcTreatDecimalAsInt && this.scale == 0 ? Types.BIGINT : Types.DECIMAL;
    ColumnTypeInfo columnTypeInfo =
        getSnowflakeType(
            internalColTypeName,
            /*extColTypeName*/ null,
            /*udtOutputType*/ null,
            fixedColType,
            !fieldsMetadata.isEmpty(),
            isVectorType(internalColTypeName));

    this.typeName = columnTypeInfo.getExtColTypeName();
    this.type = columnTypeInfo.getColumnType();
    this.base = columnTypeInfo.getSnowflakeType();

    // TODO(SNOW-3695645): !not ported! - structured-type field metadata.
    //  The proto has no nested fields, so this is always empty (and `getSnowflakeType`
    //  therefore treats every column as non-structured).
    //    List<FieldMetadata> fieldsMetadata =
    //        getFieldMetadata(jdbcTreatDecimalAsInt, internalColTypeName, colNode);
    //    this.fields = fieldsMetadata;
    this.fields = fieldsMetadata;

    // TODO(SNOW-3695645): !not ported! - column source database/schema/table.
    //    this.columnSrcDatabase = colNode.path("database").asText();
    //    this.columnSrcSchema = colNode.path("schema").asText();
    //    this.columnSrcTable = colNode.path("table").asText();
    this.columnSrcDatabase = "";
    this.columnSrcSchema = "";
    this.columnSrcTable = "";

    // TODO(SNOW-3695645): !not ported! - auto-increment flag. Not present in the proto.
    //    this.isAutoIncrement = colNode.path("isAutoIncrement").asBoolean();
    this.isAutoIncrement = false;
  }
}
