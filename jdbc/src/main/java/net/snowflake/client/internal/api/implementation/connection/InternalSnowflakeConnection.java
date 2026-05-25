package net.snowflake.client.internal.api.implementation.connection;

import java.sql.Connection;
import java.sql.Statement;
import net.snowflake.client.api.connection.SnowflakeConnection;
import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverV1.ConnectionHandle;

/** Internal interface combining JDBC Connection and SnowflakeConnection with handle access. */
public interface InternalSnowflakeConnection extends Connection, SnowflakeConnection {

  ConnectionHandle getHandle();

  void removeStatement(Statement stmt);
}
