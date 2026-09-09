import { describe, it, expect } from 'vitest';
import type { RowMode } from '../../types/sdk-types.js';
import { createLiveConnection } from '../utils/fixtures.js';
import { executeAsync } from '../utils/index.js';

const SQL = `select 1 as id, 'name1' as name, 'name2' as name`;

const EXPECTED_BY_MODE = {
  array: [1, 'name1', 'name2'],
  object: { ID: 1, NAME: 'name2' },
  object_with_renamed_duplicated_columns: { ID: 1, NAME: 'name1', NAME_2: 'name2' },
} satisfies Record<RowMode, unknown>;
const ROW_MODES = Object.keys(EXPECTED_BY_MODE) as RowMode[];

describe('Query Row Mode', () => {
  it('defaults to object when neither connection nor statement set rowMode', async () => {
    const connection = await createLiveConnection();
    const { rows } = await executeAsync(connection, SQL);
    expect(rows[0]).toEqual(EXPECTED_BY_MODE.object);
  });

  describe('Connection rowMode', () => {
    it.each(ROW_MODES)('shapes rows according to connection rowMode = %s', async (rowMode) => {
      const connection = await createLiveConnection({ rowMode });
      const { rows } = await executeAsync(connection, SQL);
      expect(rows[0]).toEqual(EXPECTED_BY_MODE[rowMode]);
    });
  });

  describe('Statement rowMode', () => {
    it.each(ROW_MODES)('shapes rows according to statement rowMode = %s', async (rowMode) => {
      const connection = await createLiveConnection();
      const { rows } = await executeAsync(connection, SQL, { rowMode });
      expect(rows[0]).toEqual(EXPECTED_BY_MODE[rowMode]);
    });
  });

  it('statement rowMode overrides connection rowMode', async () => {
    const connection = await createLiveConnection({ rowMode: 'array' });
    const { rows } = await executeAsync(connection, SQL, { rowMode: 'object' });
    expect(rows[0]).toEqual(EXPECTED_BY_MODE.object);
  });
});
