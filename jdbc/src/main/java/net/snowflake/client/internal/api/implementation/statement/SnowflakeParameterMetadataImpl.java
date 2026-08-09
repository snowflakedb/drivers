package net.snowflake.client.internal.api.implementation.statement;

import java.sql.ParameterMetaData;
import java.util.List;
import java.util.stream.Collectors;
import lombok.AccessLevel;
import lombok.RequiredArgsConstructor;
import lombok.Value;
import net.snowflake.client.internal.api.implementation.exception.SFSQLException;
import net.snowflake.client.internal.api.implementation.exception.SFSQLFeatureNotSupportedException;
import net.snowflake.client.internal.codegen.JdbcBoundary;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.util.DelegatingWrapper;
import net.snowflake.client.internal.util.SnowflakeTypeHelper;

@JdbcBoundary
@RequiredArgsConstructor(access = AccessLevel.PRIVATE)
class SnowflakeParameterMetadataImpl implements ParameterMetaData, DelegatingWrapper {
  private final List<BindMetadata> binds;

  @Value
  static class BindMetadata {
    String type;
    boolean nullable;
    int precision;
    int scale;

    static BindMetadata from(ColumnMetadata cm) {
      return new BindMetadata(
          cm.getType(), cm.getNullable(), (int) cm.getPrecision(), (int) cm.getScale());
    }
  }

  static ParameterMetaData from(List<ColumnMetadata> metadata) {
    List<BindMetadata> binds =
        metadata.stream().map(BindMetadata::from).collect(Collectors.toList());
    return new SnowflakeParameterMetadataImpl(binds);
  }

  @Override
  public int getParameterCount() {
    return binds.size();
  }

  @Override
  public int isNullable(int param) {
    return bind(param).isNullable() ? parameterNullable : parameterNoNulls;
  }

  @Override
  public boolean isSigned(int param) {
    throw new SFSQLFeatureNotSupportedException("isSigned not supported");
  }

  @Override
  public int getPrecision(int param) {
    return bind(param).getPrecision();
  }

  @Override
  public int getScale(int param) {
    return bind(param).getScale();
  }

  @Override
  public int getParameterType(int param) {
    return SnowflakeTypeHelper.convertStringToType(bind(param).getType());
  }

  @Override
  public String getParameterTypeName(int param) {
    // Return the server-reported type name verbatim (the server reports bind type
    // names in lowercase, e.g. "text", "fixed"); this matches the reference driver.
    return bind(param).getType();
  }

  @Override
  public String getParameterClassName(int param) {
    throw new SFSQLFeatureNotSupportedException("getParameterClassName not supported");
  }

  @Override
  public int getParameterMode(int param) {
    throw new SFSQLFeatureNotSupportedException("getParameterMode not supported");
  }

  private SnowflakeParameterMetadataImpl.BindMetadata bind(int param) {
    if (param < 1 || param > binds.size()) {
      throw new SFSQLException(
          "Invalid parameter index: " + param + " (parameter count: " + binds.size() + ")");
    }
    return binds.get(param - 1);
  }
}
