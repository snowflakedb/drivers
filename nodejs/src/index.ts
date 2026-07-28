import core from './core';

// TODO: implement SnowflakeError like in old driver
export type SnowflakeError = Error;
// TODO: implement ConnectionOptions like in old driver
export type ConnectionOptions = Record<string, string>;
export type ConnectionCallback = (err: SnowflakeError | undefined, conn: Connection) => void;

// TODO: proper row typing once the bridge returns real column types.
export type Row = Record<string, unknown>;

// TODO:
// - think whether we should have connection class only in bridge that exposes same api as old driver
// - think how to export nicer types so we wouldnt have to use typeof
class Connection {
  private _core: InstanceType<typeof core.Connection>;

  constructor(options: ConnectionOptions) {
    this._core = new core.Connection(options);
  }

  connect(callback?: ConnectionCallback) {
    this.connectAsync()
      .then(() => {
        callback?.(undefined, this);
      })
      .catch((err) => {
        callback?.(err, this);
      });
  }

  connectAsync(): Promise<void> {
    return this._core.connect();
  }

  async execute(query: string): Promise<Row[]> {
    const statement = await this._core.execute(query);

    try {
      const rows: Row[] = [];

      while (true) {
        const row = await statement.getNextRow();
        if (row === null) {
          break;
        }
        rows.push(row);
      }

      return rows;
    } finally {
      statement.close();
    }
  }
}

export const createConnection = (options: ConnectionOptions) => {
  return new Connection(options);
};
