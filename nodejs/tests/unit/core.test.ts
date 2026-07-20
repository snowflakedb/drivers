import { it, expect } from 'vitest';
import core from '../../src/core/index.js';

it('able to execute code from core binary', () => {
  expect(core.dummyTestEntrypoint()).toBe('nodejs_bridge 0.0.1 ok');
});
