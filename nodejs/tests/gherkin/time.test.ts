import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../types/sdk-types.js';
import {
  createTestConnection,
  destroyConnectionAsync,
  executeAsync,
  getSnowflakeSDK,
  getStatementColumn,
  isRunningNewDriverWithBD,
} from '../e2e/utils/index.js';

// Gherkin-tracked coverage for tests/definitions/shared/types/time.feature (@nodejs_e2e).
// Broader TIME coverage (format-string/TIME_OUTPUT_FORMAT behavior) not tied to these
// specific scenarios lives in query-data-types.test.ts.
//
// Only 2 of time.feature's scenarios are covered here. The rest are deliberately left
// untagged rather than force-fit:
// - The literal/precision Scenario Outlines and the nanosecond-precision scenario all
//   depend on fractional-second output, but nodejs's default TIME_OUTPUT_FORMAT truncates
//   to whole seconds (confirmed by query-data-types.test.ts's own "returns HH:MM:SS string
//   by default" test) -- unlike Python/JDBC's typed time objects, which retain precision
//   independent of any output-format string. Making those scenarios pass would require an
//   ALTER SESSION step the scenarios themselves don't specify, and the exact
//   zero-padding/rounding behavior at each scale can't be confirmed without a live account.
// - Parameter binding isn't implemented in the new driver at all yet: `execute()`
//   (nodejs/src/index.ts) only threads `options.sqlText` through to the core, ignoring any
//   `binds` field entirely -- so the two parameter-binding scenarios are skipped, not just
//   deferred as untested.
// - Table operations and the large-result-set scenarios need CREATE TABLE test
//   infrastructure that doesn't exist yet anywhere in nodejs/tests/e2e.
//
// The NULL test also runs against the old driver and branches on BD#14: the old driver
// represents a NULL TIME value as the literal string 'NULL' by default (its
// representNullAsStringNull connection option, unimplemented in the new driver), while the new
// driver always returns real null.

describe('TIME type support', () => {
  const snowflake = getSnowflakeSDK();
  let connection: Connection;

  beforeAll(async () => {
    connection = createTestConnection(snowflake);
    await connection.connectAsync();
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  it('should cast time values to appropriate type', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT '10:30:00'::TIME AS T1, '00:00:00'::TIME AS T2, '23:59:59'::TIME AS T3`;

    // When Query "SELECT '10:30:00'::TIME, '00:00:00'::TIME, '23:59:59'::TIME" is executed
    const { statement } = await executeAsync(connection, query);

    // Then All values should be returned as appropriate type
    for (const columnName of ['T1', 'T2', 'T3']) {
      const column = getStatementColumn(statement, columnName);
      expect(column.getType()).toBe('time');
      expect(column.isTime()).toBe(true);
    }
  });

  it('should handle NULL values for time', async () => {
    // Given Snowflake client is logged in
    const query = `SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME`;

    // When Query "SELECT '10:30:00'::TIME, NULL::TIME, '23:59:59'::TIME" is executed
    const { rows } = await executeAsync(connection, query);

    // Then Result should contain [10:30:00, NULL, 23:59:59]
    if (isRunningNewDriverWithBD('BD#14')) {
      expect(Object.values(rows![0])).toEqual(['10:30:00', null, '23:59:59']);
    } else {
      expect(Object.values(rows![0])).toEqual(['10:30:00', 'NULL', '23:59:59']);
    }
  });
});
