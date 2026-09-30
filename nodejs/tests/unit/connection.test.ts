import { describe, it, expect } from 'vitest';
import { createConnection } from '../../src/index.js';

const UUID_V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

describe('Connection', () => {
  describe('getId', () => {
    it('should return a UUID assigned at construction', () => {
      const connection = createConnection({ account: 'acct', username: 'user' });
      expect(connection.getId()).toMatch(UUID_V4);
    });

    it('should return a stable id for the same connection object', () => {
      const connection = createConnection({ account: 'acct', username: 'user' });
      expect(connection.getId()).toBe(connection.getId());
    });

    it('should assign distinct ids to different connection objects', () => {
      const a = createConnection({ account: 'acct', username: 'user' });
      const b = createConnection({ account: 'acct', username: 'user' });
      expect(a.getId()).not.toBe(b.getId());
    });
  });
});
