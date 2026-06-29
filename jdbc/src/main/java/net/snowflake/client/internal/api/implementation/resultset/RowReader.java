package net.snowflake.client.internal.api.implementation.resultset;

/** Combined cursor + column accessor. Represents a readable, forward-only row stream. */
interface RowReader extends RowCursor, ColumnAccessor {}
