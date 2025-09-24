package com.snowflake.unicore;

import com.snowflake.unicore.protobuf_gen.DatabaseDriverServiceClient;

public class ProtobufApis {
    public static DatabaseDriverServiceClient databaseDriverV1 = new DatabaseDriverServiceClient(new JNICoreTransport());
}
