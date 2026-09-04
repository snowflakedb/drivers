import { it, expect } from 'vitest';
import { CoreConnection } from '../../src/core/index.js';

// TODO:
// This is a placeholder test to verify that the core module compiles correctly.
// Replace or remove as proper unit tests are implemented.
it('loads core binary', () => {
  expect(CoreConnection).toBeTypeOf('function');
});

it('owns the database handle for its lifetime', async () => {
  const connection = new CoreConnection({}, {});

  await expect(connection.destroy()).resolves.toBeUndefined();
});
