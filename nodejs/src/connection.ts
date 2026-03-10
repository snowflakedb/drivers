import { SfCoreClient } from "./sf_core_client/generated/client";
import { database_driver_v1 as proto } from "./sf_core_client/generated/proto";
import {
  openArrowStream,
  readNextBatch,
  closeArrowStream,
} from "./sf_core_client/transport";

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

export interface ExecuteOptions {
  sqlText: string;
}

export interface ColumnInfo {
  name: string;
  type: string;
}

export interface ExecuteResult {
  rows: Record<string, unknown>[];
  columns: ColumnInfo[];
  queryId: string;
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

  async execute(options: ExecuteOptions): Promise<ExecuteResult> {
    if (!this.connHandle) {
      throw new Error("Not connected. Call connect() first.");
    }

    const stmtResp = await client.statementNew({
      connHandle: this.connHandle,
    });
    const stmtHandle = stmtResp.stmtHandle!;

    try {
      await client.statementSetSqlQuery({
        stmtHandle,
        query: options.sqlText,
      });

      const execResp = await client.statementExecuteQuery({ stmtHandle });
      const result = execResp.result!;

      const columns: ColumnInfo[] = (result.columns ?? []).map((c) => ({
        name: c.name ?? "",
        type: c.type ?? "",
      }));
      const queryId = result.queryId ?? "";

      const streamPtrBytes = result.stream?.value;
      if (!streamPtrBytes || streamPtrBytes.length === 0) {
        return { rows: [], columns, queryId };
      }

      const ptrBuffer =
        Buffer.isBuffer(streamPtrBytes)
          ? streamPtrBytes
          : Buffer.from(
              streamPtrBytes.buffer,
              streamPtrBytes.byteOffset,
              streamPtrBytes.byteLength,
            );

      const { handle } = openArrowStream(ptrBuffer);

      try {
        const rows: Record<string, unknown>[] = [];
        while (true) {
          const batch = readNextBatch(handle);
          if (!batch) break;
          rows.push(...batch);
        }
        return { rows, columns, queryId };
      } finally {
        closeArrowStream(handle);
      }
    } finally {
      await client.statementRelease({ stmtHandle });
    }
  }
}
