import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from './utils/index.js';

describe('Query returning data types', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection();
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('TIMESTAMP', () => {
    it('returns TIMESTAMP_LTZ as Date with zone offset', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        "SELECT to_timestamp_ltz('Thu, 21 Jan 2016 06:32:44 -0800') as LTZ_COLUMN",
      );
      const column = getStatementColumn(statement, 0);
      const selectedValue = rows![0].LTZ_COLUMN as Date;
      expect(column.isTimestamp()).toBe(true);
      expect(column.getType()).toBe('timestamp_ltz');
      expect(selectedValue).toBeInstanceOf(Date);
      expect(selectedValue.toJSON()).toBe('2016-01-21 06:32:44.000 -0800');
    });

    it('returns TIMESTAMP_TZ as Date with zone offset', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        "SELECT to_timestamp_tz('Thu, 21 Jan 2016 06:32:44 -0800') as TZ_COLUMN",
      );
      const column = getStatementColumn(statement, 0);
      const selectedValue = rows![0].TZ_COLUMN as Date;
      expect(column.isTimestamp()).toBe(true);
      expect(column.getType()).toBe('timestamp_tz');
      expect(selectedValue).toBeInstanceOf(Date);
      expect(selectedValue.toJSON()).toBe('2016-01-21 06:32:44.000 -0800');
    });

    it('returns TIMESTAMP_NTZ as Date with no zone offset', async () => {
      const { statement, rows } = await executeAsync(
        connection,
        "SELECT to_timestamp_ntz('Thu, 21 Jan 2016 06:32:44 -0800') as NTZ_COLUMN",
      );
      const column = getStatementColumn(statement, 0);
      const selectedValue = rows![0].NTZ_COLUMN as Date;
      expect(column.isTimestamp()).toBe(true);
      expect(column.getType()).toBe('timestamp_ntz');
      expect(selectedValue).toBeInstanceOf(Date);
      expect(selectedValue.toJSON()).toBe('2016-01-21 06:32:44.000');
    });
  });
});
