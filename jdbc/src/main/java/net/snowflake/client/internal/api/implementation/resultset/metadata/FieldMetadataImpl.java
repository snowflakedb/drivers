package net.snowflake.client.internal.api.implementation.resultset.metadata;

import java.util.List;
import lombok.Data;
import net.snowflake.client.api.resultset.FieldMetadata;
import net.snowflake.client.api.resultset.SnowflakeType;

/** Implementation of {@link FieldMetadata} for structured type field information. */
@Data
class FieldMetadataImpl implements FieldMetadata {

  private String name;
  private String typeName;
  private int type;
  private boolean nullable;

  private int byteLength;

  private int precision;
  private int scale;
  private boolean fixed;
  private SnowflakeType base;
  private List<FieldMetadata> fields;
}
