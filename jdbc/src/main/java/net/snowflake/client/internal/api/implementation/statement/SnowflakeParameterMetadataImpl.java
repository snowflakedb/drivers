package net.snowflake.client.internal.api.implementation.statement;

import java.sql.ParameterMetaData;
import java.sql.SQLException;
import java.sql.SQLFeatureNotSupportedException;
import java.util.List;
import java.util.stream.Collectors;
import lombok.AccessLevel;
import lombok.RequiredArgsConstructor;
import lombok.Value;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ColumnMetadata;
import net.snowflake.client.internal.util.DelegatingWrapper;
import net.snowflake.client.internal.util.SnowflakeTypeHelper;

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
  public int getParameterCount() throws SQLException {
    return binds.size();
  }

  @Override
  public int isNullable(int param) throws SQLException {
    return bind(param).isNullable() ? parameterNullable : parameterNoNulls;
  }

  @Override
  public boolean isSigned(int param) throws SQLException {
    throw new SQLFeatureNotSupportedException("isSigned not supported");
  }

  @Override
  public int getPrecision(int param) throws SQLException {
    return bind(param).getPrecision();
  }

  @Override
  public int getScale(int param) throws SQLException {
    return bind(param).getScale();
  }

  @Override
  public int getParameterType(int param) throws SQLException {
    return SnowflakeTypeHelper.convertStringToType(bind(param).getType());
  }

  @Override
  public String getParameterTypeName(int param) throws SQLException {
    // Return the server-reported type name verbatim (the server reports bind type
    // names in lowercase, e.g. "text", "fixed"); this matches the reference driver.
    return bind(param).getType();
  }

  @Override
  public String getParameterClassName(int param) throws SQLException {
    throw new SQLFeatureNotSupportedException("getParameterClassName not supported");
  }

  @Override
  public int getParameterMode(int param) throws SQLException {
    throw new SQLFeatureNotSupportedException("getParameterMode not supported");
  }

  private SnowflakeParameterMetadataImpl.BindMetadata bind(int param) throws SQLException {
    if (param < 1 || param > binds.size()) {
      throw new SQLException(
          "Invalid parameter index: " + param + " (parameter count: " + binds.size() + ")");
    }
    return binds.get(param - 1);
  }
}
