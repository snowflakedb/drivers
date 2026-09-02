import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  getStatementColumn,
} from '../e2e/utils/index.js';

// Gherkin-tracked coverage for tests/definitions/shared/types/date.feature (@nodejs_e2e).
// Broader DATE coverage not tied to these specific scenarios lives in query-data-types.test.ts;
// this file exists so its filename matches the feature's, which the validator
// (tests/tests_format_validator) requires to discover and check this coverage. Only the 5
// scenarios below have a faithful existing nodejs equivalent to retrofit; the remaining
// date.feature scenarios (table operations, parameter binding, large result sets) have no
// nodejs coverage yet and are left untagged rather than claimed.

function dateAtUtcMidnight(dateLiteral: string): Date {
  return new Date(`${dateLiteral}T00:00:00.000Z`);
}

describe('DATE type support', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should cast date values to appropriate type', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT
        '2024-01-15'::DATE AS DATE_2024_01_15,
        '1970-01-01'::DATE AS EPOCH_DATE,
        '1999-12-31'::DATE AS DATE_1999_12_31`;

    // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
    const { statement, rows } = await executeAsync(connection, query);

    const columnNames = ['DATE_2024_01_15', 'EPOCH_DATE', 'DATE_1999_12_31'];
    for (const columnName of columnNames) {
      const column = getStatementColumn(statement, columnName);
      // Then All values should be returned as DATE type
      expect(column.getType()).toBe('date');
      expect(column.isDate()).toBe(true);
      // And No precision loss should occur
      expect(rows![0][columnName]).toBeInstanceOf(Date);
    }
  });

  it('should select date literals', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE`;

    // When Query "SELECT '2024-01-15'::DATE, '1970-01-01'::DATE, '1999-12-31'::DATE" is executed
    const { rows } = await executeAsync(connection, query);

    // Then Result should contain dates [2024-01-15, 1970-01-01, 1999-12-31]
    expect(Object.values(rows![0])).toEqual([
      dateAtUtcMidnight('2024-01-15'),
      dateAtUtcMidnight('1970-01-01'),
      dateAtUtcMidnight('1999-12-31'),
    ]);
  });

  it('should select epoch and pre-epoch dates', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE`;

    // When Query "SELECT '1970-01-01'::DATE, '1969-12-31'::DATE, '1900-01-01'::DATE" is executed
    const { rows } = await executeAsync(connection, query);

    // Then Result should contain dates [1970-01-01, 1969-12-31, 1900-01-01]
    expect(Object.values(rows![0])).toEqual([
      dateAtUtcMidnight('1970-01-01'),
      dateAtUtcMidnight('1969-12-31'),
      dateAtUtcMidnight('1900-01-01'),
    ]);
  });

  it('should select historical and boundary dates', async () => {
    // Given Snowflake client is logged in
    //
    // 1582-10-15 is the Julian-to-Gregorian cutover date, so it pins the decoder to a
    // proleptic Gregorian calendar.
    const query = `SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE`;

    // When Query "SELECT '0001-01-01'::DATE, '1582-10-15'::DATE, '9999-12-31'::DATE" is executed
    const { rows } = await executeAsync(connection, query);

    // Then Result should contain dates [0001-01-01, 1582-10-15, 9999-12-31]
    expect(Object.values(rows![0])).toEqual([
      dateAtUtcMidnight('0001-01-01'),
      dateAtUtcMidnight('1582-10-15'),
      dateAtUtcMidnight('9999-12-31'),
    ]);
  });

  it('should handle NULL values for date', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE as null_column2`;

    // When Query "SELECT NULL::DATE, '2024-01-15'::DATE, NULL::DATE" is executed
    const { rows } = await executeAsync(connection, query);

    // Then Result should contain [NULL, 2024-01-15, NULL]
    expect(Object.values(rows![0])).toEqual([null, dateAtUtcMidnight('2024-01-15'), null]);
  });
});
