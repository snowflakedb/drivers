package net.snowflake.client.internal.api.implementation.resultset;

@FunctionalInterface
public interface RowConverter {

  Object[] convert(ColumnAccessor row);
}
