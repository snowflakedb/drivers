package net.snowflake.client.internal.core.arrow.converters;

import java.util.List;
import net.snowflake.client.api.exception.SFException;
import net.snowflake.client.api.resultset.SnowflakeType;
import org.apache.arrow.vector.complex.FixedSizeListVector;

public class VectorTypeConverter extends AbstractArrowVectorConverter {

  private final FixedSizeListVector vector;

  public VectorTypeConverter(
      FixedSizeListVector valueVector, int vectorIndex, DataConversionContext context) {
    super(SnowflakeType.ARRAY.name(), valueVector, vectorIndex, context);
    this.vector = valueVector;
  }

  @Override
  public String toString(int index) throws SFException {
    List<?> object = vector.getObject(index);
    if (object == null) {
      return null;
    }
    return object.toString();
  }

  // Returns the string representation rather than the raw List<?> from the Arrow vector.
  // The old driver's converter returns List<?> here, but SFArrowResultSet intercepts VECTOR
  // columns and wraps them in a StructObjectWrapper that unwraps back to the string. The
  // universal driver has no StructObjectWrapper layer, so we return the string directly to
  // preserve the public ResultSet.getObject() == getString() parity.
  @Override
  public Object toObject(int index) throws SFException {
    return toString(index);
  }

  @Override
  public byte[] toBytes(int index) throws SFException {
    return isNull(index) ? null : toString(index).getBytes();
  }
}
