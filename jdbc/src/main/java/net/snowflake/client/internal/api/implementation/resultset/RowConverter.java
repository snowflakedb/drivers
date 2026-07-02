package net.snowflake.client.internal.api.implementation.resultset;

import java.sql.SQLException;

@FunctionalInterface
public interface RowConverter {

  Object[] convert(ColumnAccessor row) throws SQLException;
}
