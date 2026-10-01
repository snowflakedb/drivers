import {
  createPool as createGenericPool,
  type Factory,
  type Options as PoolOptions,
  type Pool,
} from 'generic-pool';
import { createConnection, type Connection, type ConnectionOptions } from './index.js';

export type { Pool, PoolOptions };

type WaitingClient = {
  reject(err: Error): void;
};

type WaitingClientsQueue = {
  dequeue(): WaitingClient | undefined;
};

class ConnectionFactory implements Factory<Connection> {
  readonly #connectionOptions: ConnectionOptions;

  constructor(connectionOptions: ConnectionOptions) {
    this.#connectionOptions = connectionOptions;
  }

  async create(): Promise<Connection> {
    const connection = createConnection(this.#connectionOptions);
    await connection.connectAsync();
    return connection;
  }

  destroy(connection: Connection): Promise<void> {
    return new Promise((resolve) => {
      connection.destroy(() => {
        resolve();
      });
    });
  }

  validate(connection: Connection): Promise<boolean> {
    return connection.isValidAsync();
  }
}

export function createPool(
  options?: ConnectionOptions | null,
  poolOptions?: PoolOptions,
): Pool<Connection> {
  // TODO: SNOW-4218289 load connection options from connections.toml when they are omitted or null.
  if (options == null) {
    throw new Error('createPool requires connection options');
  }

  const connectionPool = createGenericPool(new ConnectionFactory(options), poolOptions);

  connectionPool.on('factoryCreateError', (err: Error) => {
    // generic-pool retries a failed factory and leaves waiters on this queue; drain it so they reject.
    const waiting = (
      connectionPool as Pool<Connection> & { _waitingClientsQueue: WaitingClientsQueue }
    )._waitingClientsQueue;
    let request = waiting.dequeue();
    while (request) {
      request.reject(err);
      request = waiting.dequeue();
    }
  });

  return connectionPool;
}
