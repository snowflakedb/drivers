import { describe, expect, it } from 'vitest';
import { createConnection, createLiveConnection } from '../utils/fixtures.js';
import { executeAsync, isRunningNewDriverWithBD } from '../utils/index.js';

describe('queryTag', () => {
  describe('tests/definitions/shared/query/query_tag.feature', () => {
    it('should tag queries when QUERY_TAG is set at connection level', async () => {
      // Given Snowflake client is logged in with connection option QUERY_TAG set to "conn_tag_e2e"
      const connection = await createLiveConnection({ queryTag: 'conn_tag_e2e' });

      // When Query "SELECT CURRENT_QUERY_TAG()" is executed
      const { rows } = await executeAsync(connection, 'SELECT CURRENT_QUERY_TAG()');

      // Then the result should contain value "conn_tag_e2e"
      expect(Object.values(rows[0])).toEqual(['conn_tag_e2e']);
    });

    it('should tag a single query via statement-level query tag', async () => {
      // Given Snowflake client is logged in
      const connection = await createLiveConnection();

      // When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt_tag_e2e"
      const { rows } = await executeAsync(connection, 'SELECT CURRENT_QUERY_TAG()', {
        parameters: { QUERY_TAG: 'stmt_tag_e2e' },
      });

      // Then the result should contain value "stmt_tag_e2e"
      expect(Object.values(rows[0])).toEqual(['stmt_tag_e2e']);
    });

    it('should not leak statement-level query tag into session state', async () => {
      // Given Snowflake client is logged in
      const connection = await createLiveConnection();

      // When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt_tag_e2e"
      await executeAsync(connection, 'SELECT CURRENT_QUERY_TAG()', {
        parameters: { QUERY_TAG: 'stmt_tag_e2e' },
      });

      // And Query "SELECT CURRENT_QUERY_TAG()" is executed without a statement-level tag
      const { rows } = await executeAsync(connection, 'SELECT CURRENT_QUERY_TAG()');

      // Then the last result should contain empty value
      expect(Object.values(rows[0])).toEqual(['']);
    });

    it('should let a statement QUERY_TAG override the connection QUERY_TAG', async () => {
      // Given Snowflake client is logged in with connection option QUERY_TAG set to "conn"
      const connection = await createLiveConnection({ queryTag: 'conn' });

      // When Query "SELECT CURRENT_QUERY_TAG()" is executed with statement-level QUERY_TAG "stmt"
      const { rows } = await executeAsync(connection, 'SELECT CURRENT_QUERY_TAG()', {
        parameters: { QUERY_TAG: 'stmt' },
      });

      // Then the result should contain value "stmt"
      expect(Object.values(rows[0])).toEqual([isRunningNewDriverWithBD('BD#68') ? 'stmt' : 'conn']);
    });
  });

  it('should reject a queryTag longer than 2000 characters', async () => {
    const queryTag = 'a'.repeat(2001);

    if (isRunningNewDriverWithBD('BD#69')) {
      const connection = createConnection({ queryTag });
      await expect(connection.connectAsync()).rejects.toMatchObject({
        name: 'Error',
        code: '001008',
        message: expect.stringContaining("parameter 'QUERY_TAG'"),
      });
    } else {
      expect(() => createConnection({ queryTag })).toThrow(
        expect.objectContaining({
          name: 'InvalidParameterError',
          code: 404063,
          message: 'Invalid query tag. The specified value must be a string;',
        }),
      );
    }
  });
});
