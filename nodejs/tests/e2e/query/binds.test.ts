import { describe, expect, it } from 'vitest';
import type { Connection, Binds } from '../../types/sdk-types.js';
import { executeAsync, isRunningNewDriverWithBD, withConnection } from '../utils/index.js';
import { withTemporaryTable } from '../utils/query.js';

const LOW_THRESHOLD = 3;
const HIGH_THRESHOLD = 1_000_000;

describe('Query Binds', () => {
  async function insertRowsAndSelect(connection: Connection, binds: Binds): Promise<unknown[]> {
    return withTemporaryTable(connection, 'ID NUMBER, VAL STRING', async (tableName) => {
      await executeAsync(connection, `INSERT INTO ${tableName} (ID, VAL) VALUES (?, ?)`, { binds });
      const { rows } = await executeAsync(connection, `SELECT VAL FROM ${tableName} ORDER BY ID`);
      return rows.map((row) => row.VAL);
    });
  }

  async function expectStagePathUsed(connection: Connection): Promise<void> {
    const { rows: stageFiles } = await executeAsync(connection, 'LIST @SYSTEM$BIND');
    expect(stageFiles.length).toBeGreaterThanOrEqual(1);
  }

  async function expectStagePathNotUsed(connection: Connection): Promise<void> {
    await expect(executeAsync(connection, 'LIST @SYSTEM$BIND')).rejects.toMatchObject({
      error: { message: expect.stringContaining('does not exist') },
    });
  }

  describe('arrayBindingThreshold connection parameter', () => {
    it('should upload binds to the SYSTEM$BIND stage above the threshold', async () => {
      await withConnection({ arrayBindingThreshold: LOW_THRESHOLD }, async (connection) => {
        const selected = await insertRowsAndSelect(connection, [
          [1, 'a'],
          [2, 'b'],
          [3, 'c'],
        ]);
        await expectStagePathUsed(connection);
        expect(selected).toEqual(['a', 'b', 'c']);
      });
    });

    it('should send binds inline below the threshold', async () => {
      await withConnection({ arrayBindingThreshold: HIGH_THRESHOLD }, async (connection) => {
        const selected = await insertRowsAndSelect(connection, [
          [1, 'a'],
          [2, 'b'],
          [3, 'c'],
        ]);
        await expectStagePathNotUsed(connection);
        expect(selected).toEqual(['a', 'b', 'c']);
      });
    });

    it('should produce identical rows via the stage and inline paths', async () => {
      const binds = [
        [1, 'plain'],
        [2, 'has,comma'],
        [3, 'has"quote'],
        [4, 'has\nnewline'],
        [5, null],
      ];

      let viaStage: unknown[] = [];
      await withConnection({ arrayBindingThreshold: LOW_THRESHOLD }, async (connection) => {
        viaStage = await insertRowsAndSelect(connection, binds);
      });
      let viaInline: unknown[] = [];
      await withConnection({ arrayBindingThreshold: HIGH_THRESHOLD }, async (connection) => {
        viaInline = await insertRowsAndSelect(connection, binds);
      });

      expect(viaStage).toEqual(['plain', 'has,comma', 'has"quote', 'has\nnewline', null]);
      expect(viaStage).toEqual(viaInline);
    });
  });

  describe('CLIENT_STAGE_ARRAY_BINDING_THRESHOLD session parameter', () => {
    it('should resolve the threshold from the session parameter when no connection option is set', async () => {
      await withConnection({}, async (connection) => {
        await executeAsync(
          connection,
          `ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = ${LOW_THRESHOLD}`,
        );
        const selected = await insertRowsAndSelect(connection, [
          [1, 'a'],
          [2, 'b'],
          [3, 'c'],
        ]);
        await expectStagePathUsed(connection);
        expect(selected).toEqual(['a', 'b', 'c']);
      });
    });

    it('should let the connection option override the session parameter', async () => {
      await withConnection({ arrayBindingThreshold: HIGH_THRESHOLD }, async (connection) => {
        await executeAsync(
          connection,
          `ALTER SESSION SET CLIENT_STAGE_ARRAY_BINDING_THRESHOLD = ${LOW_THRESHOLD}`,
        );
        const selected = await insertRowsAndSelect(connection, [
          [1, 'a'],
          [2, 'b'],
          [3, 'c'],
        ]);
        if (isRunningNewDriverWithBD('BD#26')) {
          await expectStagePathUsed(connection);
        } else {
          await expectStagePathNotUsed(connection);
        }
        expect(selected).toEqual(['a', 'b', 'c']);
      });
    });
  });
});
