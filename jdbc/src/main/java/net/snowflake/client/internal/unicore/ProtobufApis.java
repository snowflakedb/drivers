package net.snowflake.client.internal.unicore;

import net.snowflake.client.internal.unicore.protobuf_gen.DatabaseDriverServiceClient;

public class ProtobufApis {

  public static CoreDriverApi coreDriverApi =
      new CoreDriverApiImpl(new DatabaseDriverServiceClient(new JNICoreTransport()));
}
