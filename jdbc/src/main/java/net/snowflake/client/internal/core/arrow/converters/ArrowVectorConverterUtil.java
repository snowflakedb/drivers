package net.snowflake.client.internal.core.arrow.converters;

import java.util.Map;
import net.snowflake.client.api.exception.SnowflakeSQLException;
import net.snowflake.client.api.resultset.SnowflakeType;
import org.apache.arrow.vector.FieldVector;
import org.apache.arrow.vector.ValueVector;
import org.apache.arrow.vector.types.Types;
import org.apache.arrow.vector.types.pojo.Field;

public final class ArrowVectorConverterUtil {
  private ArrowVectorConverterUtil() {}

  public static SnowflakeType getSnowflakeTypeFromFieldMetadata(Field field) {
    Map<String, String> customMeta = field.getMetadata();
    if (customMeta != null && customMeta.containsKey("logicalType")) {
      return SnowflakeType.valueOf(customMeta.get("logicalType"));
    }
    return null;
  }

  /** Read the {@code scale} from an Arrow field's metadata, defaulting to 0 when absent. */
  private static int getScaleFromFieldMetadata(ValueVector vector) {
    String scaleStr = vector.getField().getMetadata().get("scale");
    return scaleStr == null ? 0 : Integer.parseInt(scaleStr);
  }

  /**
   * Given an arrow vector (a single column in a single record batch), return an arrow vector
   * converter. Converter is built on top of arrow vector, so arrow data can be converted back to
   * java data.
   */
  public static ArrowVectorConverter initConverter(
      ValueVector vector, DataConversionContext context, int idx) throws SnowflakeSQLException {
    Types.MinorType type = Types.getMinorTypeForArrowType(vector.getField().getType());
    SnowflakeType st = getSnowflakeTypeFromFieldMetadata(vector.getField());

    if (type == Types.MinorType.DECIMAL) {
      return new DecimalToScaledFixedConverter(vector, idx, context);
    }

    if (st != null) {
      switch (st) {
        case ANY:
        case CHAR:
        case TEXT:
        case VARIANT:
          return new VarCharConverter(vector, idx, context);

        case BINARY:
          return new VarBinaryToBinaryConverter(vector, idx, context);

        case BOOLEAN:
          return new BitToBooleanConverter(vector, idx, context);

        case DATE:
          return new DateConverter(vector, idx, context);

        case FIXED:
          int sfScale = getScaleFromFieldMetadata(vector);
          switch (type) {
            case TINYINT:
              if (sfScale == 0) {
                return new TinyIntToFixedConverter(vector, idx, context);
              }
              return new TinyIntToScaledFixedConverter(vector, idx, context, sfScale);
            case SMALLINT:
              if (sfScale == 0) {
                return new SmallIntToFixedConverter(vector, idx, context);
              }
              return new SmallIntToScaledFixedConverter(vector, idx, context, sfScale);
            case INT:
              if (sfScale == 0) {
                return new IntToFixedConverter(vector, idx, context);
              }
              return new IntToScaledFixedConverter(vector, idx, context, sfScale);
            case BIGINT:
              if (sfScale == 0) {
                return new BigIntToFixedConverter(vector, idx, context);
              }
              return new BigIntToScaledFixedConverter(vector, idx, context, sfScale);
            default:
              break;
          }
          break;

        case DECFLOAT:
          return new DecfloatToDecimalConverter(vector, idx, context);

        case REAL:
          return new DoubleToRealConverter(vector, idx, context);

        case TIME:
          int timeScale = getScaleFromFieldMetadata(vector);
          switch (type) {
            case INT:
            case BIGINT:
              return new TimeConverter(vector, idx, context, timeScale);
            default:
              throw new SnowflakeSQLException("Unsupported Arrow physical type for TIME: " + type);
          }

          // Structured types (MAP/ARRAY/OBJECT) currently fall back to string rendering, matching
          // legacy snowflake-jdbc's VarCharConverter fallback when the column is not materialized
          // as
          // a native complex Arrow vector. Once the universal driver materializes these as complex
          // vectors, dispatch to a dedicated converter (guarded by the vector type, as legacy
          // does):
          //   MAP    -> MapConverter    for MapVector    (else VarCharConverter)
          //   ARRAY  -> ArrayConverter  for ListVector   (else VarCharConverter)
          //   OBJECT -> StructConverter for StructVector (else VarCharConverter)
          // TODO(SNOW-2881790): implement the dedicated MAP/ARRAY/OBJECT converters above.
        case MAP:
        case ARRAY:
        case OBJECT:
          return new VarCharConverter(vector, idx, context);

        default:
          throw new SnowflakeSQLException("Unsupported Arrow logical type: " + st.name());
      }
    }

    throw new SnowflakeSQLException("Unsupported Arrow field type: " + type);
  }

  public static ArrowVectorConverter initConverter(
      FieldVector vector, DataConversionContext context, int columnIndex)
      throws SnowflakeSQLException {
    return initConverter((ValueVector) vector, context, columnIndex);
  }
}
