import { describe, it, beforeAll, afterAll, expect } from 'vitest';
import type { Connection, RowMode } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
} from './utils/index.js';

const SQL = `select 1 as id, 'name1' as name, 'name2' as name`;

const EXPECTED_BY_MODE = {
  array: [1, 'name1', 'name2'],
  object: { ID: 1, NAME: 'name2' },
  object_with_renamed_duplicated_columns: { ID: 1, NAME: 'name1', NAME_2: 'name2' },
} satisfies Record<RowMode, unknown>;
const ROW_MODES = Object.keys(EXPECTED_BY_MODE) as RowMode[];

describe('Query Row Mode', () => {
  const snowflake = getSnowflakeSDK();

  it('defaults to object when neither connection nor statement set rowMode', async () => {
    const connection = createTestConnection(snowflake);
    try {
      await connection.connectAsync();
      const { rows } = await executeAsync(connection, SQL);
      expect(rows[0]).toEqual(EXPECTED_BY_MODE.object);
    } finally {
      await destroyConnectionAsync(connection);
    }
  });

  describe('connection rowMode', () => {
    it.each(ROW_MODES)('shapes rows according to connection rowMode = %s', async (rowMode) => {
      const connection = createTestConnection(snowflake, { rowMode });
      try {
        await connection.connectAsync();
        const { rows } = await executeAsync(connection, SQL);
        expect(rows[0]).toEqual(EXPECTED_BY_MODE[rowMode]);
      } finally {
        await destroyConnectionAsync(connection);
      }
    });
  });

  describe('statement rowMode', () => {
    let connection: Connection;

    beforeAll(async () => {
      connection = createTestConnection(snowflake);
      await connection.connectAsync();
    });

    afterAll(async () => {
      await destroyConnectionAsync(connection);
    });

    it.each(ROW_MODES)('shapes rows according to statement rowMode = %s', async (rowMode) => {
      const { rows } = await executeAsync(connection, SQL, { rowMode });
      expect(rows[0]).toEqual(EXPECTED_BY_MODE[rowMode]);
    });
  });

  it('statement rowMode overrides connection rowMode', async () => {
    const connection = createTestConnection(snowflake, { rowMode: 'array' });
    try {
      await connection.connectAsync();
      const { rows } = await executeAsync(connection, SQL, { rowMode: 'object' });
      expect(rows[0]).toEqual(EXPECTED_BY_MODE.object);
    } finally {
      await destroyConnectionAsync(connection);
    }
  });
});
