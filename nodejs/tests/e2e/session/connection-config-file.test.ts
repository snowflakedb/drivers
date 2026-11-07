import type { Connection as NewConnection, Pool as NewPool } from 'snowflake-sdk';
import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it, onTestFinished } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import { createTestDir, deletePathIgnoringErrors } from '../utils/files.js';
import {
  connectAsyncWithErrorBD,
  destroyConnectionAsync,
  executeAsync,
  isRunningNewDriverWithBD,
  snowflake,
  TEST_CONNECTION_OPTIONS,
} from '../utils/index.js';

describe('tests/definitions/shared/session/connection_config_file.feature', () => {
  it('should connect using the default profile when no connection options are given', async () => {
    // Given connections.toml defines a "default" profile with valid credentials
    const home = createSnowflakeHome();
    createConnectionsToml(home, { default: createValidProfile(home) });

    // When a connection is opened with no connection options
    const connection = await createBareConnection();

    // Then SELECT 1 returns 1
    await expectSelectOne(connection);
  });

  it('should honor SNOWFLAKE_DEFAULT_CONNECTION_NAME', async () => {
    // Given connections.toml defines a valid "fromFile" profile and an invalid "default" profile
    const home = createSnowflakeHome();
    createConnectionsToml(home, {
      default: { account: 'not-a-real-account' },
      fromFile: createValidProfile(home),
    });
    // And SNOWFLAKE_DEFAULT_CONNECTION_NAME is "fromFile"
    process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME = 'fromFile';

    // When a connection is opened with no connection options
    const connection = await createBareConnection();

    // Then SELECT 1 returns 1
    await expectSelectOne(connection);
  });

  it('should connect a pool when no connection options are given', async () => {
    // Given connections.toml defines a "default" profile with valid credentials
    const home = createSnowflakeHome();
    createConnectionsToml(home, { default: createValidProfile(home) });

    // When a pool is opened with no connection options
    const pool = snowflake.createPool(undefined, { max: 1, min: 0 }) as NewPool<NewConnection>;
    onTestFinished(async () => {
      await pool.drain();
      await pool.clear();
    });
    const connection = await pool.acquire();
    onTestFinished(async () => {
      if (pool.isBorrowedResource(connection)) {
        await pool.release(connection);
      }
    });

    // Then SELECT 1 returns 1
    await expectSelectOne(connection);
  });

  describe.skipIf(!isRunningNewDriverWithBD('BD#103'))('BD#103', () => {
    it('should connect using a named profile from connections.toml', async () => {
      // Given connections.toml defines a "fromFile" profile with valid credentials
      const home = createSnowflakeHome();
      createConnectionsToml(home, { fromFile: createValidProfile(home) });

      // When a connection is opened with connection name "fromFile"
      const connection = snowflake.createConnection({ connectionName: 'fromFile' });
      await connection.connectAsync();
      onTestFinished(async () => {
        await destroyConnectionAsync(connection);
      });

      // Then SELECT 1 returns 1
      await expectSelectOne(connection);
    });

    it('should fall back to a profile defined only in config.toml', async () => {
      // Given config.toml names "fromFile" as the default and defines that profile
      const home = createSnowflakeHome();
      writeConfigToml(home, 'fromFile', createValidProfile(home));
      // And connections.toml does not exist
      expect(fs.existsSync(path.join(home, 'connections.toml'))).toBe(false);

      // When a connection is opened with no connection options
      const connection = await createBareConnection();

      // Then SELECT 1 returns 1
      await expectSelectOne(connection);
    });

    it('should let explicit options override the profile', async () => {
      // Given connections.toml defines a "fromFile" profile with an invalid account
      const home = createSnowflakeHome();
      createConnectionsToml(home, { fromFile: { account: 'not-a-real-account' } });

      // When a connection is opened with connection name "fromFile" and valid credentials
      const connection = snowflake.createConnection({
        ...TEST_CONNECTION_OPTIONS,
        connectionName: 'fromFile',
      });
      await connection.connectAsync();
      onTestFinished(async () => {
        await destroyConnectionAsync(connection);
      });

      // Then SELECT 1 returns 1
      await expectSelectOne(connection);
    });
  });

  it('should fail when the named profile is missing', async () => {
    // Given connections.toml defines a "default" profile
    const home = createSnowflakeHome();
    createConnectionsToml(home, { default: { account: 'not-a-real-account' } });

    // When a connection is opened with connection name "missing"
    const open = createMissingProfileConnection;

    // Then the connection fails because the profile does not exist
    await expectProfileLoadFailure(open, /not found|does not exist/i);
  });

  it.skipIf(process.platform === 'win32')(
    'should reject a group-or-world-writable connections.toml',
    async () => {
      // Given connections.toml is writable by others
      const home = createSnowflakeHome();
      const file = createConnectionsToml(home, { default: createValidProfile(home) });
      fs.chmodSync(file, 0o666);

      // When a connection is opened with no connection options
      const open = () => snowflake.createConnection();

      // Then the connection fails because the file permissions are unsafe
      await expectProfileLoadFailure(open, /writable|permission/i);
    },
  );
});

describe('unknown option in a connection profile', () => {
  it('should connect when connections.toml contains an unknown option', async () => {
    const home = createSnowflakeHome();
    createConnectionsToml(home, {
      default: { ...createValidProfile(home), not_a_real_option: 'value' },
    });
    const connection = await createBareConnection();
    await expectSelectOne(connection);
  });
});

async function expectProfileLoadFailure(open: () => Connection, reason: RegExp): Promise<void> {
  if (!isRunningNewDriverWithBD('BD#103')) {
    expect(open).toThrow('Connection options must be specified.');
    return;
  }
  await expect(connectAsyncWithErrorBD(open())).rejects.toThrow(reason);
}

function createMissingProfileConnection(): Connection {
  if (!isRunningNewDriverWithBD('BD#103')) {
    process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME = 'missing';
    return snowflake.createConnection();
  }
  return snowflake.createConnection({ connectionName: 'missing' });
}

function createSnowflakeHome(): string {
  const previousHome = process.env.SNOWFLAKE_HOME;
  const previousName = process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME;
  const home = createTestDir('connection-config-file');
  process.env.SNOWFLAKE_HOME = home;
  delete process.env.SNOWFLAKE_DEFAULT_CONNECTION_NAME;
  onTestFinished(() => {
    restoreEnv('SNOWFLAKE_HOME', previousHome);
    restoreEnv('SNOWFLAKE_DEFAULT_CONNECTION_NAME', previousName);
    deletePathIgnoringErrors(home);
  });
  return home;
}

function restoreEnv(
  name: 'SNOWFLAKE_HOME' | 'SNOWFLAKE_DEFAULT_CONNECTION_NAME',
  previous?: string,
) {
  if (previous === undefined) {
    delete process.env[name];
  } else {
    process.env[name] = previous;
  }
}

async function createBareConnection(): Promise<Connection> {
  const connection = snowflake.createConnection();
  await connection.connectAsync();
  onTestFinished(async () => {
    await destroyConnectionAsync(connection);
  });
  return connection;
}

async function expectSelectOne(connection: Connection): Promise<void> {
  const { rows } = await executeAsync(connection, 'SELECT 1');
  expect(Object.values(rows[0])).toEqual([1]);
}

function createValidProfile(home: string): Record<string, string> {
  const options = TEST_CONNECTION_OPTIONS;
  const profile: Record<string, string> = {
    account: String(options.account),
    user: String(options.username),
    warehouse: String(options.warehouse),
    database: String(options.database),
    schema: String(options.schema),
    role: String(options.role),
  };
  if (options.host !== undefined) {
    profile.host = String(options.host);
  }
  if (options.password !== undefined) {
    profile.password = String(options.password);
  }
  if (options.authenticator !== undefined) {
    profile.authenticator = String(options.authenticator);
  }
  if (options.privateKey !== undefined) {
    const keyPath = path.join(home, 'key.p8');
    fs.writeFileSync(keyPath, String(options.privateKey), { mode: 0o600 });
    profile.private_key_file = keyPath;
  }
  if (options.privateKeyPass !== undefined) {
    profile.private_key_file_pwd = String(options.privateKeyPass);
  }
  return profile;
}

function createConnectionsToml(
  home: string,
  profiles: Record<string, Record<string, string>>,
): string {
  const file = path.join(home, 'connections.toml');
  const body = Object.entries(profiles)
    .map(([name, fields]) => `[${name}]\n${toTomlKeyValues(fields)}`)
    .join('\n');
  fs.writeFileSync(file, body, { mode: 0o600 });
  return file;
}

function writeConfigToml(home: string, name: string, fields: Record<string, string>): void {
  const body = `default_connection_name = ${JSON.stringify(name)}\n\n[connections.${name}]\n${toTomlKeyValues(fields)}`;
  fs.writeFileSync(path.join(home, 'config.toml'), body, { mode: 0o600 });
}

function toTomlKeyValues(fields: Record<string, string>): string {
  return Object.entries(fields)
    .map(([key, value]) => `${key} = ${JSON.stringify(value)}`)
    .join('\n');
}
