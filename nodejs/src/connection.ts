import { SfCoreClient } from "./sf_core_client/generated/client";
import { database_driver_v1 as proto } from "./sf_core_client/generated/proto";

export interface ConnectionOptions {
  username: string;
  account?: string;
  password?: string;
  database?: string;
  schema?: string;
  warehouse?: string;
  role?: string;
  host?: string;
  authenticator?: string;
}

const KEY_REMAP: Record<string, string> = {
  username: "user",
};

const client = new SfCoreClient();

export class Connection {
  private readonly options: ConnectionOptions;
  private dbHandle: proto.IDatabaseHandle | null = null;
  private connHandle: proto.IConnectionHandle | null = null;

  constructor(options: ConnectionOptions) {
    this.options = { ...options };
  }

  async connect(): Promise<Connection> {
    const dbNewResp = await client.databaseNew({});
    this.dbHandle = dbNewResp.dbHandle!; // ! should be handled in types

    await client.databaseInit({ dbHandle: this.dbHandle });

    const connNewResp = await client.connectionNew({});
    this.connHandle = connNewResp.connHandle!;

    for (const [rawKey, value] of Object.entries(this.options)) {
      if (value === undefined) continue;
      const key = KEY_REMAP[rawKey] ?? rawKey;

      if (typeof value === "number") {
        await client.connectionSetOptionInt({
          connHandle: this.connHandle,
          key,
          value,
        });
      } else if (typeof value === "string") {
        await client.connectionSetOptionString({
          connHandle: this.connHandle,
          key,
          value,
        });
      }
    }

    await client.connectionInit({
      connHandle: this.connHandle,
      dbHandle: this.dbHandle,
    });

    return this;
  }
}
