import { it, expect } from 'vitest';
import core from '../../src/core/index.js';

// TODO:
// This is a placeholder test to verify that the core module compiles correctly.
// Replace or remove as proper unit tests are implemented.
it('loads core binary', () => {
  expect(core.Connection).toBeTypeOf('function');
});
