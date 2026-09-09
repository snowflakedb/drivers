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

const CORNER_CASES: { name: string; hex: string }[] = [
  { name: 'empty binary', hex: '' },
  { name: 'single min byte', hex: '00' },
  { name: 'single max byte', hex: 'FF' },
  { name: 'all zeros', hex: '0000000000' },
  { name: 'all ones', hex: 'FFFFFFFFFF' },
  { name: 'embedded nulls', hex: '48006500' },
];

// The old driver monkey-patches BINARY Buffers with extra own methods (BD#12) that break a
// plain-Buffer .toEqual; copying into a fresh Buffer drops them. The new driver already returns
// a plain Buffer, so its value is compared as-is.
function toBdFriendlyValue(value: unknown): Buffer | null {
  if (value === null) {
    return null;
  }
  if (isRunningNewDriverWithBD('BD#12')) {
    return value as Buffer;
  }
  return Buffer.from(value as Buffer);
}

describe('BINARY data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/binary.feature', () => {
    it('should cast binary values to appropriate type', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY, TO_BINARY('V29ybGQ=', 'BASE64')::BINARY" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY AS COL1, TO_BINARY('V29ybGQ=', 'BASE64')::BINARY AS COL2`,
      );

      // Then All values should be returned as appropriate binary type
      for (const index of [0, 1]) {
        const column = getStatementColumn(statement, index);
        expect(column.getType()).toBe('binary');
        expect(column.isBinary()).toBe(true);
      }

      // And the result should contain binary values:
      expect(Object.values(rows[0]).map(toBdFriendlyValue)).toEqual([
        Buffer.from('48656C6C6F', 'hex'),
        Buffer.from('V29ybGQ=', 'base64'),
      ]);
    });

    it('should select binary literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Queries selecting binary literals are executed:
      const { rows } = await executeAsync(
        connection,
        `SELECT X'48656C6C6F'::BINARY AS BIN1,
                TO_BINARY('48656C6C6F', 'HEX')::BINARY AS BIN2,
                TO_BINARY('ASNFZ4mrze8=', 'BASE64')::BINARY AS BIN3`,
      );

      // Then the results should contain expected binary values
      expect(Object.values(rows[0]).map(toBdFriendlyValue)).toEqual([
        Buffer.from('48656C6C6F', 'hex'),
        Buffer.from('48656C6C6F', 'hex'),
        Buffer.from('ASNFZ4mrze8=', 'base64'),
      ]);
    });

    it('should handle binary corner case values from literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query selecting corner case binary literals is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT ${CORNER_CASES.map(({ hex }, index) => `X'${hex}' AS C${index}`).join(', ')}`,
      );

      // Then the result should contain expected corner case binary values
      CORNER_CASES.forEach(({ name, hex }, index) => {
        expect(toBdFriendlyValue(rows[0][`C${index}`]), name).toEqual(Buffer.from(hex, 'hex'));
      });
    });

    it('should handle NULL binary values from literals', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT NULL::{type}, X'ABCD', NULL::{type}" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT NULL::BINARY AS COL1, X'ABCD' AS COL2, NULL::BINARY AS COL3`,
      );

      // Then Result should contain [NULL, 0xABCD, NULL]
      expect(Object.values(rows[0]).map(toBdFriendlyValue)).toEqual([
        null,
        Buffer.from('ABCD', 'hex'),
        null,
      ]);
    });

    it('should select binary values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with BINARY column is created
      const tableName = await createTemporaryTable(connection, 'COL BINARY');

      // And The table is populated with binary values [X'48656C6C6F', X'576F726C64', X'0123456789ABCDEF']
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (COL) VALUES (X'48656C6C6F'), (X'576F726C64'), (X'0123456789ABCDEF')`,
      );

      // When Query "SELECT * FROM {table} ORDER BY col" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY COL`);

      // Then the result should contain binary values in order:
      expect(rows.map((row) => toBdFriendlyValue(row.COL))).toEqual([
        Buffer.from('0123456789ABCDEF', 'hex'),
        Buffer.from('48656C6C6F', 'hex'),
        Buffer.from('576F726C64', 'hex'),
      ]);
    });

    it('should select corner case binary values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with BINARY column is created
      const tableName = await createTemporaryTable(connection, 'COL BINARY');

      // And The table is populated with corner case binary values
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (COL) VALUES ${CORNER_CASES.map(({ hex }) => `(X'${hex}')`).join(', ')}`,
      );

      // When Query "SELECT * FROM {table} ORDER BY 1" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY 1`);

      // Then the result should contain the inserted corner case binary values
      const actual = rows.map((row) => toBdFriendlyValue(row.COL) as Buffer).sort(Buffer.compare);
      const expected = CORNER_CASES.map(({ hex }) => Buffer.from(hex, 'hex')).sort(Buffer.compare);
      expect(actual).toEqual(expected);
    });

    it('should select NULL binary values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with BINARY column is created
      const tableName = await createTemporaryTable(connection, 'COL BINARY');

      // And The table is populated with NULL and non-NULL binary values [NULL, X'ABCD', NULL]
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (COL) VALUES (NULL), (X'ABCD'), (NULL)`,
      );

      // When Query "SELECT * FROM {table}" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName}`);
      const values = rows.map((row) => toBdFriendlyValue(row.COL));

      // Then there are 3 rows returned
      expect(rows).toHaveLength(3);
      // And 2 rows should contain NULL values
      expect(values.filter((value) => value === null)).toHaveLength(2);
      // And 1 row should contain 0xABCD
      expect(
        values.filter((value) => value !== null && value.equals(Buffer.from('ABCD', 'hex'))),
      ).toHaveLength(1);
    });

    it('should select binary with specified length from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with columns (bin5 BINARY(5), bin10 BINARY(10), bin_default BINARY) exists
      const tableName = await createTemporaryTable(
        connection,
        'BIN5 BINARY(5), BIN10 BINARY(10), BIN_DEFAULT BINARY',
      );
      // And Row (X'0102030405', X'01020304050607080910', X'48656C6C6F') is inserted
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (BIN5, BIN10, BIN_DEFAULT)
           VALUES (X'0102030405', X'01020304050607080910', X'48656C6C6F')`,
      );

      // When Query "SELECT * FROM {table}" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName}`);

      // Then Result should contain binary values with correct lengths
      expect(rows[0].BIN5).toHaveLength(5);
      expect(toBdFriendlyValue(rows[0].BIN5)).toEqual(Buffer.from('0102030405', 'hex'));
      expect(rows[0].BIN10).toHaveLength(10);
      expect(toBdFriendlyValue(rows[0].BIN10)).toEqual(Buffer.from('01020304050607080910', 'hex'));
      expect(toBdFriendlyValue(rows[0].BIN_DEFAULT)).toEqual(Buffer.from('48656C6C6F', 'hex'));
    });

    it('should handle VARBINARY as synonym for BINARY', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And A temporary table with VARBINARY column is created
      const tableName = await createTemporaryTable(connection, 'COL VARBINARY');
      // And The table is populated with binary values via VARBINARY column
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} (COL) VALUES (X'48656C6C6F'), (X'576F726C64'), (X'0123456789ABCDEF')`,
      );

      // When Query "SELECT * FROM {table} ORDER BY col" is executed
      const { statement, rows } = await executeAsync(
        connection,
        `SELECT * FROM ${tableName} ORDER BY COL`,
      );

      // Then the result should match the equivalent BINARY behavior
      expect(getStatementColumn(statement, 0).getType()).toBe('binary');
      expect(rows.map((row) => toBdFriendlyValue(row.COL))).toEqual([
        Buffer.from('0123456789ABCDEF', 'hex'),
        Buffer.from('48656C6C6F', 'hex'),
        Buffer.from('576F726C64', 'hex'),
      ]);
    });

    describe('parameter binding', () => {
      it('should select binary literals using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::BINARY, ?::BINARY, ?::BINARY" is executed with bound binary values [0x48656C6C6F, 0x576F726C64, 0x0123456789ABCDEF]
        const { rows } = await executeAsync(
          connection,
          'SELECT ?::BINARY AS COL1, ?::BINARY AS COL2, ?::BINARY AS COL3',
          { binds: ['48656C6C6F', '576F726C64', '0123456789ABCDEF'] },
        );

        // Then the result should contain:
        expect(Object.values(rows[0]).map(toBdFriendlyValue)).toEqual([
          Buffer.from('48656C6C6F', 'hex'),
          Buffer.from('576F726C64', 'hex'),
          Buffer.from('0123456789ABCDEF', 'hex'),
        ]);
      });

      it('should insert binary using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with BINARY column exists
        const tableName = await createTemporaryTable(connection, 'COL BINARY');
        // When Binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x] are inserted using binding
        const values = ['48656C6C6F', '576F726C64', '00', 'FF', ''];
        await executeAsync(connection, `INSERT INTO ${tableName} (COL) VALUES (?)`, {
          binds: values.map((hex) => [hex]),
        });

        // And Query "SELECT * FROM {table}" is executed
        const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName}`);

        // Then Result should contain binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x]
        const actual = rows.map((row) => toBdFriendlyValue(row.COL) as Buffer).sort(Buffer.compare);
        const expected = values.map((hex) => Buffer.from(hex, 'hex')).sort(Buffer.compare);
        expect(actual).toEqual(expected);
      });

      it.each([
        { name: 'empty binary', hex: '' },
        { name: 'single null byte', hex: '00' },
        { name: 'single max byte', hex: 'FF' },
        { name: 'embedded nulls', hex: '48006500' },
        { name: 'NULL', hex: null },
      ])('should bind corner case binary values ($name)', async ({ hex }) => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT ?::BINARY" is executed with each corner case binary value bound
        const { rows } = await executeAsync(connection, 'SELECT ?::BINARY', { binds: [hex] });

        // Then the result should match the bound corner case value
        expect(toBdFriendlyValue(Object.values(rows[0])[0])).toEqual(
          hex === null ? null : Buffer.from(hex, 'hex'),
        );
      });
    });

    describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('multiple chunks', () => {
      const rowCount = 30_000;

      it('should download binary data in multiple chunks using GENERATOR', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT seq8() AS id, TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') AS bin_val FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v ORDER BY id" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT seq8() AS id, TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') AS bin_val FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})) v ORDER BY id`,
        );

        // Then there are 30000 rows returned
        expect(rows).toHaveLength(rowCount);
        // And all returned binary values should match the generated values in order
        expect(rows.map((row) => (row.BIN_VAL as Buffer).toString('utf-8'))).toEqual(
          Array.from({ length: rowCount }, (_, index) => String(index).padStart(10, '0')),
        );
      });

      it('should download binary data in multiple chunks from table', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with (bin_data BINARY) exists with 30000 sequential binary values
        const tableName = await createTemporaryTable(connection, 'BIN_DATA BINARY');
        await executeAsync(
          connection,
          `INSERT INTO ${tableName} (BIN_DATA)
             SELECT TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8')
             FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount}))`,
        );

        // When Query "SELECT * FROM {table} ORDER BY bin_data" is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT * FROM ${tableName} ORDER BY BIN_DATA`,
        );

        // Then there are 30000 rows returned
        expect(rows).toHaveLength(rowCount);
        // And all returned binary values should match the inserted values in order
        expect(rows.map((row) => (row.BIN_DATA as Buffer).toString('utf-8'))).toEqual(
          Array.from({ length: rowCount }, (_, index) => String(index).padStart(10, '0')),
        );
      });
    });
  });

  describe('fetchAsString', () => {
    it("should render a NULL BINARY cell as the string 'NULL'", async () => {
      const { rows } = await executeAsync(connection, 'SELECT NULL::BINARY', {
        fetchAsString: ['Buffer'],
      });
      expect(Object.values(rows[0])).toEqual(['NULL']);
    });

    it('should render a NULL BINARY cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, 'SELECT NULL::BINARY', {
        fetchAsString: ['Buffer'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });

    describe('tests/definitions/shared/types/binary_to_string.feature', () => {
      it('should encode binary as HEX string by default', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT X'0123456789ABCDEF' AS bin" is executed
        const { rows } = await executeAsync(connection, `SELECT X'0123456789ABCDEF' AS bin`, {
          // And bin is converted to string representation
          fetchAsString: ['Buffer'],
        });

        // Then the string should be '0123456789ABCDEF'
        expect(rows[0].BIN).toBe('0123456789ABCDEF');
      });

      // BUG in old driver, this feature isn't working properly. Should fix in new driver at some point.
      it.todo('should encode binary as BASE64 string when BINARY_OUTPUT_FORMAT is set to BASE64 for the query', async () => {
        // Given Snowflake client is logged in
        void connection;

        // When Query "SELECT X'0123456789ABCDEF' AS bin" is executed with statement-level BINARY_OUTPUT_FORMAT 'BASE64'
        const { rows } = await executeAsync(connection, `SELECT X'0123456789ABCDEF' AS bin`, {
          // And bin is converted to string representation
          fetchAsString: ['Buffer'],
          parameters: { BINARY_OUTPUT_FORMAT: 'BASE64' },
        });

        // Then the string should be 'ASNFZ4mrze8='
        expect(rows[0].BIN).toBe('ASNFZ4mrze8=');
      });
    });
  });
});
