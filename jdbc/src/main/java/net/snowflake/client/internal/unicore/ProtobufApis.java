package net.snowflake.client.internal.unicore;

import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverServiceClient;

public class ProtobufApis {
  public static DatabaseDriverServiceClient databaseDriverV1 =
      new DatabaseDriverServiceClient(new JNICoreTransport());
}
