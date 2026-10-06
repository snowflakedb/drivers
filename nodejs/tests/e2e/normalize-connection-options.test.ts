import type { ConnectionOptions as NewConnectionOptions } from 'snowflake-sdk';
import type { ConnectionOptions as OldConnectionOptions } from 'snowflake-sdk-old';
import fs from 'node:fs';
import { describe, expect, it, onTestFinished } from 'vitest';
import { createTempDir } from './utils/fixtures.js';
import {
  destroyConnectionAsync,
  executeAsync,
  snowflake,
  TEST_CONNECTION_OPTIONS,
} from './utils/index.js';

describe('snowflake.normalizeConnectionOptions()', () => {
  it.each([
    [
      'camelCase before snake_case',
      { privateKeyPath: '/camel', private_key_path: '/snake' },
      '/camel',
    ],
    [
      'snake_case before camelCase',
      { private_key_path: '/snake', privateKeyPath: '/camel' },
      '/snake',
    ],
    [
      'an alias before its target',
      { private_key_file: '/alias', privateKeyPath: '/camel' },
      '/alias',
    ],
  ])('should keep the first key when %s resolve to the same option', (_order, input, expected) => {
    expect(snowflake.normalizeConnectionOptions(input)).toEqual({ privateKeyPath: expected });
  });

  it('should produce options that connect from snake_case keys', async () => {
    const { username, privateKey, privateKeyPass, ...camelCaseOptions } =
      TEST_CONNECTION_OPTIONS as Record<string, unknown>;
    const snakeCaseOptions: Record<string, unknown> = { ...camelCaseOptions, user: username };
    if (typeof privateKey === 'string') {
      const keyFile = createTempDir().writeFile('rsa_key.p8', privateKey);
      fs.chmodSync(keyFile, 0o600);
      snakeCaseOptions.private_key_file = keyFile;
      snakeCaseOptions.private_key_pass = privateKeyPass;
    }

    const normalized = snowflake.normalizeConnectionOptions(snakeCaseOptions);
    const connection = snowflake.createConnection(
      normalized as NewConnectionOptions & OldConnectionOptions,
    );
    await connection.connectAsync();
    onTestFinished(() => destroyConnectionAsync(connection));

    const { rows } = await executeAsync(connection, 'SELECT 1 AS ONE');
    expect(rows).toEqual([{ ONE: 1 }]);
  });
});
