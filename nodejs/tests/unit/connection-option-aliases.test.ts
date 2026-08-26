import { describe, it, expect } from 'vitest';
import { normalizeConnectionOptions } from '../../src/connection-option-aliases.js';

describe('normalizeConnectionOptions', () => {
  it('maps legacy snowflake-sdk aliases onto canonical snake_case names', () => {
    expect(
      normalizeConnectionOptions({
        username: 'alice',
        privateKey: '-----BEGIN PRIVATE KEY-----',
        privateKeyPass: 'secret',
      }),
    ).toEqual({
      user: 'alice',
      private_key: '-----BEGIN PRIVATE KEY-----',
      private_key_password: 'secret',
    });
  });

  it('passes through keys that are not aliases unchanged', () => {
    expect(
      normalizeConnectionOptions({
        account: 'sfctest0',
        password: 'p@ss',
        warehouse: 'testwh',
      }),
    ).toEqual({
      account: 'sfctest0',
      password: 'p@ss',
      warehouse: 'testwh',
    });
  });

  it('handles a mix of aliased and canonical keys in one call', () => {
    expect(
      normalizeConnectionOptions({
        account: 'sfctest0',
        username: 'bob',
        password: 'p@ss',
      }),
    ).toEqual({
      account: 'sfctest0',
      user: 'bob',
      password: 'p@ss',
    });
  });

  it('returns an empty object for empty input', () => {
    expect(normalizeConnectionOptions({})).toEqual({});
  });
});
