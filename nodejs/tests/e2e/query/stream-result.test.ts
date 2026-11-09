import { describe, expect, it } from 'vitest';
import type { Connection, FetchResultOptions, RowStatement } from '../../types/sdk-types.js';
import { createLiveConnection } from '../utils/fixtures.js';
import { executeAsync, fetchResultAsync, getResultsFromQueryIdAsync } from '../utils/index.js';

const SQL = 'SELECT seq4() AS ID FROM table(generator(rowCount => 3)) ORDER BY ID';
const EXPECTED_ROWS = [{ ID: 0 }, { ID: 1 }, { ID: 2 }];

describe('streamResult', () => {
  describe('execute', () => {
    it.each<{
      connection: boolean | undefined;
      statement: boolean | undefined;
      rowsFrom: 'complete' | 'streamRows';
    }>([
      { connection: undefined, statement: undefined, rowsFrom: 'complete' },
      { connection: undefined, statement: false, rowsFrom: 'complete' },
      { connection: undefined, statement: true, rowsFrom: 'streamRows' },
      { connection: false, statement: undefined, rowsFrom: 'complete' },
      { connection: false, statement: false, rowsFrom: 'complete' },
      { connection: false, statement: true, rowsFrom: 'streamRows' },
      { connection: true, statement: undefined, rowsFrom: 'streamRows' },
      { connection: true, statement: false, rowsFrom: 'complete' },
      { connection: true, statement: true, rowsFrom: 'streamRows' },
    ])(
      'should return rows from $rowsFrom when connection streamResult is $connection and statement streamResult is $statement',
      async ({
        connection: connectionStreamResult,
        statement: statementStreamResult,
        rowsFrom,
      }) => {
        const connection = await createLiveConnection({ streamResult: connectionStreamResult });
        const { statement, rows } = await executeAsync(connection, SQL, {
          streamResult: statementStreamResult,
        });

        if (rowsFrom === 'streamRows') {
          expect(rows).toBeUndefined();
          expect(await statement.streamRows().toArray()).toEqual(EXPECTED_ROWS);
        } else {
          expect(rows).toEqual(EXPECTED_ROWS);
        }
      },
    );
  });

  describe.each<{
    method: string;
    fetch: (
      connection: Connection,
      options: FetchResultOptions,
    ) => Promise<{ statement: RowStatement; rows: Record<string, unknown>[] }>;
  }>([
    {
      method: 'fetchResult',
      fetch: (connection, options) => fetchResultAsync(connection, options).completion,
    },
    {
      method: 'getResultsFromQueryId',
      fetch: (connection, options) => getResultsFromQueryIdAsync(connection, options),
    },
  ])('$method', ({ fetch }) => {
    async function runQuery(connection: Connection): Promise<string> {
      const { statement } = await executeAsync(connection, SQL);
      return statement.getQueryId()!;
    }

    it('should leave rows for streamRows when the connection sets streamResult', async () => {
      const connection = await createLiveConnection({ streamResult: true });
      const queryId = await runQuery(connection);
      const { statement, rows } = await fetch(connection, { queryId });
      expect(rows).toBeUndefined();
      expect(await statement.streamRows().toArray()).toEqual(EXPECTED_ROWS);
    });

    it('should let its own streamResult override the connection streamResult', async () => {
      const connection = await createLiveConnection({ streamResult: true });
      const queryId = await runQuery(connection);
      const { rows } = await fetch(connection, { queryId, streamResult: false });
      expect(rows).toEqual(EXPECTED_ROWS);
    });
  });
});
