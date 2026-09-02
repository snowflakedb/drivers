import { describe, it, expect } from 'vitest';
import { normalizeConnectionOptions } from '../../src/connection-option-aliases.js';

describe('normalizeConnectionOptions', () => {
  it('maps camelCase driver options onto their sf_core snake_case keys', () => {
    expect(
      normalizeConnectionOptions({
        account: 'sfctest0',
        username: 'alice',
        privateKey: '-----BEGIN PRIVATE KEY-----',
        privateKeyPass: 'secret',
      }),
    ).toEqual({
      account: 'sfctest0',
      user: 'alice',
      private_key: '-----BEGIN PRIVATE KEY-----',
      private_key_password: 'secret',
    });
  });

  it('throws on a key that is not in the alias map', () => {
    expect(() =>
      normalizeConnectionOptions({
        account: 'sfctest0',
        notARealOption: 'x',
      }),
    ).toThrow('Unknown connection option: notARealOption');
  });

  it('returns an empty object for empty input', () => {
    expect(normalizeConnectionOptions({})).toEqual({});
  });
});
