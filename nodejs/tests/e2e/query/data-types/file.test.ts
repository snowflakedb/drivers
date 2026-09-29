import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Connection } from '../../../types/sdk-types.js';
import { createLiveConnection, createTemporaryTable } from '../../utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  getStatementColumn,
  isRunningNewDriverWithBD,
  NOT_IMPLEMENTED_IN_NEW_DRIVER,
} from '../../utils/index.js';
import { createLiveNullPreservingConnection } from '../utils.js';

const FILE_DOCUMENT = {
  RELATIVE_PATH: 'some_new_file.jpeg',
  STAGE: '@myStage',
  STAGE_FILE_URL: 'some_new_file.jpeg',
  SIZE: 123,
  ETAG: 'xxx',
  CONTENT_TYPE: 'image/jpeg',
  LAST_MODIFIED: '2025-01-01',
};
const FILE_EXPRESSION = `TO_FILE(PARSE_JSON('${JSON.stringify(FILE_DOCUMENT)}'))`;

const OTHER_FILE_DOCUMENT = {
  RELATIVE_PATH: 'quarterly_report.pdf',
  STAGE: '@otherStage',
  STAGE_FILE_URL: 'reports/quarterly_report.pdf',
  SIZE: 45678,
  ETAG: 'yyy',
  CONTENT_TYPE: 'application/pdf',
  LAST_MODIFIED: '2025-06-30',
};
const OTHER_FILE_EXPRESSION = `TO_FILE(PARSE_JSON('${JSON.stringify(OTHER_FILE_DOCUMENT)}'))`;

describe('FILE data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/file.feature', () => {
    it('should cast FILE values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When A FILE column is populated via TO_FILE and queried
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, FILE_COL FILE');
      await executeAsync(connection, `INSERT INTO ${tableName} SELECT 1, ${FILE_EXPRESSION}`);
      const { rows, statement } = await executeAsync(
        connection,
        `SELECT FILE_COL FROM ${tableName}`,
      );

      // Then the FILE column should be returned as appropriate type with the expected JSON document
      expect(getStatementColumn(statement, 'FILE_COL').getType()).toBe('object');
      expect(rows[0].FILE_COL).toEqual(FILE_DOCUMENT);
    });

    it('should select a FILE value built by TO_FILE without a table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT TO_FILE(PARSE_JSON('{"RELATIVE_PATH": "some_new_file.jpeg", ...}'))" is executed
      const { rows } = await executeAsync(connection, `SELECT ${FILE_EXPRESSION}`);

      // Then the result should contain the expected FILE JSON document
      expect(Object.values(rows[0])).toEqual([FILE_DOCUMENT]);
    });

    it('should handle NULL FILE values from literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT TO_FILE(PARSE_JSON('...')), TO_FILE(NULL)" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT ${FILE_EXPRESSION} AS FILE_COL, TO_FILE(NULL) AS NULL_COL`,
      );

      // Then the result should contain the expected FILE JSON document and NULL
      expect(rows[0].FILE_COL).toEqual(FILE_DOCUMENT);
      expect(rows[0].NULL_COL).toBeNull();
    });

    it('should select FILE values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with an ID and a FILE column is created
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, FILE_COL FILE');

      // And The table is populated with two different FILE values
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} SELECT 1, ${FILE_EXPRESSION} UNION ALL SELECT 2, ${OTHER_FILE_EXPRESSION}`,
      );

      // When Query "SELECT * FROM {table} ORDER BY ID" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY ID`);

      // Then the result should contain the inserted FILE JSON documents in order
      expect(rows.map((row) => row.FILE_COL)).toEqual([FILE_DOCUMENT, OTHER_FILE_DOCUMENT]);
    });

    it('should handle NULL FILE values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with an ID and a FILE column is created
      const tableName = await createTemporaryTable(connection, 'ID NUMBER, FILE_COL FILE');

      // And The table is populated with a FILE value, a NULL and another FILE value
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} SELECT 1, ${FILE_EXPRESSION} UNION ALL SELECT 2, TO_FILE(NULL) UNION ALL SELECT 3, ${OTHER_FILE_EXPRESSION}`,
      );

      // When Query "SELECT * FROM {table} ORDER BY ID" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY ID`);

      // Then the result should contain the inserted FILE JSON documents and NULL in order
      expect(rows.map((row) => row.FILE_COL)).toEqual([FILE_DOCUMENT, null, OTHER_FILE_DOCUMENT]);
    });

    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should download FILE data in multiple chunks',
      async () => {
        const rowCount = 20_000;
        // Given Snowflake client is logged in
        void connection;

        // When Query generating 20000 FILE values is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT ID, ${FILE_EXPRESSION} AS FILE_COL
             FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS ID
                   FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})))
             ORDER BY ID`,
        );

        // Then All 20000 rows should be fetched with the expected FILE JSON documents
        expect(rows).toHaveLength(rowCount);
        expect(rows.map((row) => row.ID)).toEqual(Array.from({ length: rowCount }, (_, id) => id));
        expect(rows.map((row) => row.FILE_COL)).toEqual(
          Array.from({ length: rowCount }, () => FILE_DOCUMENT),
        );
      },
    );
  });

  // fetchAsString JSON does not stringify object, array, or map; see BCR_LOG.md
  describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('fetchAsString', () => {
    it('should return FILE unchanged when fetchAsString is set', async () => {
      const { rows } = await executeAsync(connection, `SELECT ${FILE_EXPRESSION}`, {
        fetchAsString: ['JSON'],
      });
      expect(Object.values(rows[0])).toEqual([FILE_DOCUMENT]);
    });

    it("should render a NULL FILE cell as the string 'NULL' when fetchAsString is set", async () => {
      const { rows } = await executeAsync(connection, `SELECT TO_FILE(NULL)`, {
        fetchAsString: ['JSON'],
      });
      const expectedNull = isRunningNewDriverWithBD('BD#18') ? 'NULL' : null;
      expect(Object.values(rows[0])).toEqual([expectedNull]);
    });

    it('should render a NULL FILE cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, `SELECT TO_FILE(NULL)`, {
        fetchAsString: ['JSON'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });
});
