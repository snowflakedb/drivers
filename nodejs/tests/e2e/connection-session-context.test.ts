import { describe, expect, it, onTestFinished } from 'vitest';
import { createLiveConnection } from './utils/fixtures.js';
import { executeAsync, randomizeName } from './utils/index.js';

describe('Connection session context options', () => {
  it('should use the warehouse from connection options', async () => {
    // The configured warehouse is also the user's DEFAULT_WAREHOUSE, so passing it
    // would prove nothing: the session would report it even if the option were
    // dropped. Switch to any other warehouse the role can use instead.
    const setupConnection = await createLiveConnection();
    const { rows: sessionRows } = await executeAsync(
      setupConnection,
      'SELECT CURRENT_WAREHOUSE() AS WAREHOUSE_NAME',
    );
    const defaultWarehouse = String(sessionRows[0].WAREHOUSE_NAME).toUpperCase();
    // Pipe SHOW into a SELECT of just the name: the full SHOW WAREHOUSES output
    // has TIMESTAMP_LTZ columns the driver cannot decode yet.
    const { rows: warehouseRows } = await executeAsync(
      setupConnection,
      'SHOW WAREHOUSES ->> SELECT "name" AS WAREHOUSE_NAME FROM $1',
    );
    const warehouse = warehouseRows
      .map((row) => String(row.WAREHOUSE_NAME))
      .find((name) => name.toUpperCase() !== defaultWarehouse);
    if (warehouse === undefined) {
      throw new Error(
        `The test account must grant a warehouse other than ${defaultWarehouse}, ` +
          'otherwise this test cannot tell an applied warehouse option from an ignored one',
      );
    }

    const connection = await createLiveConnection({ warehouse });
    const { rows } = await executeAsync(connection, 'SELECT CURRENT_WAREHOUSE() AS WAREHOUSE_NAME');

    expect(rows[0].WAREHOUSE_NAME).toBe(warehouse.toUpperCase());
  });

  it('should use the database from connection options', async () => {
    const database = randomizeName('NODEJS_CONNECTION_OPTION_DATABASE_');
    const setupConnection = await createLiveConnection();
    await executeAsync(setupConnection, `CREATE DATABASE ${database}`);
    onTestFinished(async () => {
      await executeAsync(setupConnection, `DROP DATABASE IF EXISTS ${database}`);
    });

    const connection = await createLiveConnection({ database });
    const { rows } = await executeAsync(connection, 'SELECT CURRENT_DATABASE() AS DATABASE_NAME');

    expect(rows[0].DATABASE_NAME).toBe(database.toUpperCase());
  });

  it('should use the schema from connection options', async () => {
    const schema = randomizeName('NODEJS_CONNECTION_OPTION_SCHEMA_');
    const setupConnection = await createLiveConnection();
    await executeAsync(setupConnection, `CREATE SCHEMA ${schema}`);
    onTestFinished(async () => {
      await executeAsync(setupConnection, `DROP SCHEMA IF EXISTS ${schema}`);
    });

    const connection = await createLiveConnection({ schema });
    const { rows } = await executeAsync(connection, 'SELECT CURRENT_SCHEMA() AS SCHEMA_NAME');

    expect(rows[0].SCHEMA_NAME).toBe(schema.toUpperCase());
  });
});
