import { describe, it, beforeAll, afterAll } from 'vitest';
import { createConnection, connectAsync, destroyAsync, sleepAsync } from './utils.js';
import type { Connection } from './utils.js';

describe('Query Cancelation', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createConnection();
    await connectAsync(connection);
  });

  afterAll(async () => {
    await destroyAsync(connection);
  });

  it('cancels a running query', async () => {
    const statement = connection.execute({
      sqlText: 'select count(*) from table(generator(timeLimit => 3600))',
    });
    await sleepAsync(2000);
    await new Promise<void>((resolve, reject) => {
      statement.cancel((err) => (err ? reject(err) : resolve()));
    });
  });
});
