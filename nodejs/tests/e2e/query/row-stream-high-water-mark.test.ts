import { once } from 'node:events';
import { describe, it, expect } from 'vitest';
import { createLiveConnection } from '../utils/fixtures.js';
import { executeAsync } from '../utils/index.js';

async function createRowStreamHighWaterMark(rowStreamHighWaterMark?: number): Promise<number> {
  const connection = await createLiveConnection(
    rowStreamHighWaterMark === undefined ? {} : { rowStreamHighWaterMark },
  );
  const { statement } = await executeAsync(connection, 'SELECT 1', { streamResult: true });
  const stream = statement.streamRows();
  const highWaterMark = stream.readableHighWaterMark;
  await stream.toArray();
  return highWaterMark;
}

describe('rowStreamHighWaterMark', () => {
  it('should default the streamRows highWaterMark to 10', async () => {
    expect(await createRowStreamHighWaterMark()).toBe(10);
  });

  it('should use rowStreamHighWaterMark from the connection options', async () => {
    expect(await createRowStreamHighWaterMark(25)).toBe(25);
  });

  it('should stop buffering rows at rowStreamHighWaterMark until they are read', async () => {
    const connection = await createLiveConnection({ rowStreamHighWaterMark: 5 });
    const { statement } = await executeAsync(
      connection,
      'SELECT seq8() FROM table(generator(rowCount => 200))',
      { streamResult: true },
    );
    const stream = statement.streamRows();

    await once(stream, 'readable');
    expect(stream.readableLength).toBe(5);

    expect(await stream.toArray()).toHaveLength(200);
  });
});
