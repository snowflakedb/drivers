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

const POINT_SQL = 'POINT(1820.12 890.56)';
const POINT_GEOJSON = { type: 'Point', coordinates: [1820.12, 890.56] };

const LINESTRING_SQL = 'LINESTRING(0 0, 1 1, 2 2)';
const LINESTRING_GEOJSON = {
  type: 'LineString',
  coordinates: [
    [0, 0],
    [1, 1],
    [2, 2],
  ],
};

const POLYGON_SQL = 'POLYGON((0 0, 4 0, 4 3, 0 3, 0 0))';
const POLYGON_GEOJSON = {
  type: 'Polygon',
  coordinates: [
    [
      [0, 0],
      [4, 0],
      [4, 3],
      [0, 3],
      [0, 0],
    ],
  ],
};

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('GEOMETRY data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/geometry.feature', () => {
    it.each([
      { shape: 'Point', queryValue: `TO_GEOMETRY('${POINT_SQL}')`, expected: POINT_GEOJSON },
      {
        shape: 'LineString',
        queryValue: `TO_GEOMETRY('${LINESTRING_SQL}')`,
        expected: LINESTRING_GEOJSON,
      },
      { shape: 'Polygon', queryValue: `TO_GEOMETRY('${POLYGON_SQL}')`, expected: POLYGON_GEOJSON },
    ])('should select $shape geometry literal', async ({ queryValue, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <query_value>" is executed
      const { rows, statement } = await executeAsync(connection, `SELECT ${queryValue}`);

      // Then Result should contain a GeoJSON <shape> value
      expect(getStatementColumn(statement, 0).getType()).toBe('object');
      expect(Object.values(rows[0])).toEqual([expected]);
    });

    it.each([
      { format: 'GeoJSON', expectedType: 'object' },
      { format: 'WKT', expectedType: 'text' },
      { format: 'WKB', expectedType: 'binary' },
      { format: 'EWKT', expectedType: 'text' },
      { format: 'EWKB', expectedType: 'binary' },
    ])(
      'should cast geometry to $expectedType for $format output format',
      async ({ format, expectedType }) => {
        // Given Snowflake client is logged in
        void connection;

        // And Session parameter GEOMETRY_OUTPUT_FORMAT is set to <format>
        void 0;

        // When Query "SELECT TO_GEOMETRY('POINT(1820.12 890.56)')" is executed
        const { rows, statement } = await executeAsync(
          connection,
          `SELECT TO_GEOMETRY('${POINT_SQL}')`,
          {
            parameters: { GEOMETRY_OUTPUT_FORMAT: format },
          },
        );

        // Then Result should be returned as <expected_type> type
        const value = Object.values(rows[0])[0];
        const columnType = getStatementColumn(statement, 0).getType();
        expect(columnType).toBe(expectedType);
        if (expectedType === 'object') {
          expect(value).toEqual(POINT_GEOJSON);
        } else if (expectedType === 'binary') {
          expect(Buffer.isBuffer(value)).toBe(true);
        } else {
          expect(typeof value).toBe('string');
        }
      },
    );

    it('should select geometry values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with GEOMETRY column exists with WKT values
      const tableName = await createTemporaryTable(connection, 'ID INT, GEO GEOMETRY');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName}
           SELECT 1, TO_GEOMETRY('${POINT_SQL}')
           UNION ALL SELECT 2, TO_GEOMETRY('${LINESTRING_SQL}')
           UNION ALL SELECT 3, TO_GEOMETRY('${POLYGON_SQL}')`,
      );

      // When Query "SELECT * FROM <table> ORDER BY id" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY ID`);

      // Then Result should contain the expected GeoJSON values
      expect(rows.map((row) => row.GEO)).toEqual([
        POINT_GEOJSON,
        LINESTRING_GEOJSON,
        POLYGON_GEOJSON,
      ]);
    });

    it('should handle NULL geometry values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with GEOMETRY column exists containing NULLs and values
      const tableName = await createTemporaryTable(connection, 'ID INT, GEO GEOMETRY');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} SELECT 1, TO_GEOMETRY('${POINT_SQL}') UNION ALL SELECT 2, NULL`,
      );

      // When Query "SELECT * FROM <table> ORDER BY id" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY ID`);

      // Then Result should contain [GeoJSON Point, NULL]
      expect(rows.map((row) => row.GEO)).toEqual([POINT_GEOJSON, null]);
    });

    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should download geometry data in multiple chunks',
      async () => {
        const rowCount = 20_000;

        // Given Snowflake client is logged in
        void connection;

        // When Query generating 20000 geometry points is executed
        const { rows } = await executeAsync(
          connection,
          `SELECT ID, TO_GEOMETRY('POINT(' || ID || ' ' || ID || ')') AS GEO
           FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS ID
                 FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})))
           ORDER BY ID`,
        );

        // Then All 20000 rows should be fetched with valid GeoJSON Point values
        expect(rows).toHaveLength(rowCount);
        expect(rows.map((row) => row.ID)).toEqual(Array.from({ length: rowCount }, (_, id) => id));
        expect(rows.map((row) => row.GEO)).toEqual(
          Array.from({ length: rowCount }, (_, id) => ({ type: 'Point', coordinates: [id, id] })),
        );
      },
    );

    describe('parameter binding', () => {
      it.each([
        { inputType: 'WKT string', bind: POINT_SQL, expected: POINT_GEOJSON },
        { inputType: 'NULL', bind: null, expected: null },
      ])(
        'should select geometry using parameter binding with $inputType value',
        async ({ bind, expected }) => {
          // Given Snowflake client is logged in
          void connection;

          // When Query "SELECT TO_GEOMETRY(?)" is executed with bound <input_type> value
          const { rows } = await executeAsync(connection, `SELECT TO_GEOMETRY(?)`, {
            binds: [bind],
          });

          // Then Result should <expected_result>
          expect(Object.values(rows[0])).toEqual([expected]);
        },
      );

      it('should insert geometry using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with GEOMETRY column exists
        const tableName = await createTemporaryTable(connection, 'ID INT, GEO GEOMETRY');

        // When Geometry WKT values are inserted using parameter binding via TO_GEOMETRY(?)
        const shapes = [POINT_SQL, LINESTRING_SQL, POLYGON_SQL];
        for (const [index, shape] of shapes.entries()) {
          await executeAsync(connection, `INSERT INTO ${tableName} SELECT ?, TO_GEOMETRY(?)`, {
            binds: [index + 1, shape],
          });
        }

        // Then SELECT should return the inserted GeoJSON values
        const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY ID`);
        expect(rows.map((row) => row.GEO)).toEqual([
          POINT_GEOJSON,
          LINESTRING_GEOJSON,
          POLYGON_GEOJSON,
        ]);
      });
    });
  });

  describe('fetchAsString', () => {
    it('should return GEOMETRY unchanged when fetchAsString is set', async () => {
      const { rows } = await executeAsync(connection, `SELECT TO_GEOMETRY('${POINT_SQL}')`, {
        fetchAsString: ['JSON'],
      });
      expect(Object.values(rows[0])).toEqual([POINT_GEOJSON]);
    });

    it("should render a NULL GEOMETRY cell as the string 'NULL' when fetchAsString is set", async () => {
      const { rows } = await executeAsync(connection, `SELECT NULL::GEOMETRY`, {
        fetchAsString: ['JSON'],
      });
      const expectedNull = isRunningNewDriverWithBD('BD#18') ? 'NULL' : null;
      expect(Object.values(rows[0])).toEqual([expectedNull]);
    });

    it('should render a NULL GEOMETRY cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, `SELECT NULL::GEOMETRY`, {
        fetchAsString: ['JSON'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });
});
