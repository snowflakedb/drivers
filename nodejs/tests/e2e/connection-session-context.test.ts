import { describe, expect, it } from 'vitest';
import { createLiveConnection } from './utils/fixtures.js';
import { executeAsync } from './utils/index.js';

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
});
