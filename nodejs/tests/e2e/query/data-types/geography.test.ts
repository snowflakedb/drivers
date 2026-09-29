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

const POINT_SQL = 'POINT(-122.35 37.55)';
const POINT_GEOJSON = { type: 'Point', coordinates: [-122.35, 37.55] };

const LINESTRING_SQL = 'LINESTRING(0 0, 1 1, 2 2)';
const LINESTRING_GEOJSON = {
  type: 'LineString',
  coordinates: [
    [0, 0],
    [1, 1],
    [2, 2],
  ],
};

const POLYGON_SQL = 'POLYGON((0 0, 10 0, 10 10, 0 10, 0 0))';
const POLYGON_GEOJSON = {
  type: 'Polygon',
  coordinates: [
    [
      [0, 0],
      [10, 0],
      [10, 10],
      [0, 10],
      [0, 0],
    ],
  ],
};

describe.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)('GEOGRAPHY data type', () => {
  let connection: Connection;

  beforeAll(async () => {
    connection = await createLiveConnection({}, false);
  });

  afterAll(async () => {
    await destroyConnectionAsync(connection);
  });

  describe('tests/definitions/shared/types/geography.feature', () => {
    it.each([
      { shape: 'Point', queryValue: `TO_GEOGRAPHY('${POINT_SQL}')`, expected: POINT_GEOJSON },
      {
        shape: 'LineString',
        queryValue: `TO_GEOGRAPHY('${LINESTRING_SQL}')`,
        expected: LINESTRING_GEOJSON,
      },
      { shape: 'Polygon', queryValue: `TO_GEOGRAPHY('${POLYGON_SQL}')`, expected: POLYGON_GEOJSON },
    ])('should select $shape geography literal', async ({ queryValue, expected }) => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT <query_value>" is executed
      const { rows, statement } = await executeAsync(connection, `SELECT ${queryValue}`);

      // Then Result should contain a GeoJSON <shape> value
      expect(getStatementColumn(statement, 0).getType()).toBe('object');
      expect(Object.values(rows[0])).toEqual([expected]);
    });

    it('should select geography from GeoJSON input', async () => {
      // Given Snowflake client is logged in
      void connection;

      // When Query "SELECT TO_GEOGRAPHY('{"type":"Point","coordinates":[-122.35,37.55]}')" is executed
      const { rows } = await executeAsync(
        connection,
        `SELECT TO_GEOGRAPHY('${JSON.stringify(POINT_GEOJSON)}')`,
      );

      // Then Result should contain a GeoJSON Point value
      expect(Object.values(rows[0])).toEqual([POINT_GEOJSON]);
    });

    it.each([
      { format: 'GeoJSON', expectedType: 'object' },
      { format: 'WKT', expectedType: 'text' },
      { format: 'WKB', expectedType: 'binary' },
      { format: 'EWKT', expectedType: 'text' },
      { format: 'EWKB', expectedType: 'binary' },
    ])(
      'should cast geography to $expectedType for $format output format',
      async ({ format, expectedType }) => {
        // Given Snowflake client is logged in
        void connection;

        // And Session parameter GEOGRAPHY_OUTPUT_FORMAT is set to <format>
        void 0;

        // When Query "SELECT TO_GEOGRAPHY('POINT(-122.35 37.55)')" is executed
        const { rows, statement } = await executeAsync(
          connection,
          `SELECT TO_GEOGRAPHY('${POINT_SQL}')`,
          {
            parameters: { GEOGRAPHY_OUTPUT_FORMAT: format },
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

    it('should select geography values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with GEOGRAPHY column exists with WKT values
      const tableName = await createTemporaryTable(connection, 'ID INT, GEO GEOGRAPHY');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName}
           SELECT 1, TO_GEOGRAPHY('${POINT_SQL}')
           UNION ALL SELECT 2, TO_GEOGRAPHY('${LINESTRING_SQL}')
           UNION ALL SELECT 3, TO_GEOGRAPHY('${POLYGON_SQL}')`,
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

    it('should handle NULL geography values from table', async () => {
      // Given Snowflake client is logged in
      void connection;

      // And Table with GEOGRAPHY column exists containing NULLs and values
      const tableName = await createTemporaryTable(connection, 'ID INT, GEO GEOGRAPHY');
      await executeAsync(
        connection,
        `INSERT INTO ${tableName} SELECT 1, TO_GEOGRAPHY('${POINT_SQL}') UNION ALL SELECT 2, NULL`,
      );

      // When Query "SELECT * FROM <table> ORDER BY id" is executed
      const { rows } = await executeAsync(connection, `SELECT * FROM ${tableName} ORDER BY ID`);

      // Then Result should contain [GeoJSON Point, NULL]
      expect(rows.map((row) => row.GEO)).toEqual([POINT_GEOJSON, null]);
    });

    it.skipIf(NOT_IMPLEMENTED_IN_NEW_DRIVER)(
      'should download geography data in multiple chunks',
      async () => {
        const rowCount = 20_000;

        // Given Snowflake client is logged in
        void connection;

        // When Query generating 20000 geography points is executed
        // GEOGRAPHY is on the WGS84 sphere, so longitude stays within [-180, 180) and latitude
        // within [-90, 90); id is folded into that range so every point is a valid Lng/Lat pair.
        const toLongitude = (id: number): number => (id % 360) - 180;
        const toLatitude = (id: number): number => (id % 180) - 90;
        const { rows } = await executeAsync(
          connection,
          `SELECT ID,
                  TO_GEOGRAPHY('POINT(' || (ID % 360 - 180) || ' ' || (ID % 180 - 90) || ')') AS GEO
           FROM (SELECT (ROW_NUMBER() OVER (ORDER BY seq8()) - 1) AS ID
                 FROM TABLE(GENERATOR(ROWCOUNT => ${rowCount})))
           ORDER BY ID`,
        );

        // Then All 20000 rows should be fetched with valid GeoJSON Point values
        expect(rows).toHaveLength(rowCount);
        expect(rows.map((row) => row.ID)).toEqual(Array.from({ length: rowCount }, (_, id) => id));
        expect(rows.map((row) => row.GEO)).toEqual(
          Array.from({ length: rowCount }, (_, id) => ({
            type: 'Point',
            coordinates: [toLongitude(id), toLatitude(id)],
          })),
        );
      },
    );

    describe('parameter binding', () => {
      it.each([
        { inputType: 'WKT string', bind: POINT_SQL, expected: POINT_GEOJSON },
        { inputType: 'NULL', bind: null, expected: null },
      ])(
        'should select geography using parameter binding with $inputType value',
        async ({ bind, expected }) => {
          // Given Snowflake client is logged in
          void connection;

          // When Query "SELECT TO_GEOGRAPHY(?)" is executed with bound <input_type> value
          const { rows } = await executeAsync(connection, `SELECT TO_GEOGRAPHY(?)`, {
            binds: [bind],
          });

          // Then Result should <expected_result>
          expect(Object.values(rows[0])).toEqual([expected]);
        },
      );

      it('should insert geography using parameter binding', async () => {
        // Given Snowflake client is logged in
        void connection;

        // And Table with GEOGRAPHY column exists
        const tableName = await createTemporaryTable(connection, 'ID INT, GEO GEOGRAPHY');

        // When Geography WKT values are inserted using parameter binding via TO_GEOGRAPHY(?)
        const shapes = [POINT_SQL, LINESTRING_SQL, POLYGON_SQL];
        for (const [index, shape] of shapes.entries()) {
          await executeAsync(connection, `INSERT INTO ${tableName} SELECT ?, TO_GEOGRAPHY(?)`, {
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
    it('should return GEOGRAPHY unchanged when fetchAsString is set', async () => {
      const { rows } = await executeAsync(connection, `SELECT TO_GEOGRAPHY('${POINT_SQL}')`, {
        fetchAsString: ['JSON'],
      });
      expect(Object.values(rows[0])).toEqual([POINT_GEOJSON]);
    });

    it("should render a NULL GEOGRAPHY cell as the string 'NULL' when fetchAsString is set", async () => {
      const { rows } = await executeAsync(connection, `SELECT NULL::GEOGRAPHY`, {
        fetchAsString: ['JSON'],
      });
      const expectedNull = isRunningNewDriverWithBD('BD#18') ? 'NULL' : null;
      expect(Object.values(rows[0])).toEqual([expectedNull]);
    });

    it('should render a NULL GEOGRAPHY cell as null when representNullAsStringNull is disabled', async () => {
      const nullPreservingConnection = await createLiveNullPreservingConnection();
      const { rows } = await executeAsync(nullPreservingConnection, `SELECT NULL::GEOGRAPHY`, {
        fetchAsString: ['JSON'],
      });
      expect(Object.values(rows[0])).toEqual([null]);
    });
  });
});
