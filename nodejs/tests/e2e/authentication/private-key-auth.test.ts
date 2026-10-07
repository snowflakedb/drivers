import { createPrivateKey } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import { createTempDir } from '../utils/fixtures.js';
import getTestParameter from '../utils/getTestParameter.js';
import {
  baseConnectionOptions,
  connectAsyncWithErrorBD,
  isRunningNewDriverWithBD,
  snowflake,
} from '../utils/index.js';
import { verifySimpleQuery } from '../utils/query.js';
import { loginSuccess, logoutSuccess, WiremockServer } from '../utils/wiremock/index.js';
import {
  destroyConnectionAfterTest,
  expectConnectionNotUp,
  NEVER_ESTABLISHED_QUERY,
  TERMINATED_QUERY,
} from './utils.js';

function createTempKeyFile(contents: string): string {
  return createTempDir({ prefix: 'ud-jwt-key' }).writeFile('key.p8', contents);
}

function createJwtConnection(
  overrides: Record<string, unknown>,
  options: { includeAuthenticator?: boolean } = {},
): Connection {
  return snowflake.createConnection({
    ...baseConnectionOptions,
    ...(options.includeAuthenticator === false ? {} : { authenticator: 'SNOWFLAKE_JWT' }),
    ...overrides,
  });
}

describe('SNOWFLAKE_JWT authentication', () => {
  const INVALID_RSA_KEY_PATH = fileURLToPath(
    new URL('../../../../tests/test_data/invalid_rsa_key.p8', import.meta.url),
  );
  const JWT_TOKEN_INVALID_MESSAGE = isRunningNewDriverWithBD('BD#75')
    ? /^Failed to login: Login error: JWT token is invalid\. {2}\[[0-9a-f-]+\], code: 390144$/
    : /^JWT token is invalid\. {2}\[[0-9a-f-]+\]$/;

  describe('with configured JWT credentials', () => {
    const PRIVATE_KEY_PASSWORD = 'test-password';
    let ENCRYPTED_PRIVATE_KEY_CONTENTS: string;
    let UNENCRYPTED_PRIVATE_KEY_CONTENTS: string;

    beforeAll(() => {
      const configuredPassword = getTestParameter('SNOWFLAKE_TEST_PRIVATE_KEY_PASSWORD');
      const configuredPrivateKey = createPrivateKey({
        key: getTestParameter('SNOWFLAKE_TEST_PRIVATE_KEY_CONTENTS', true),
        format: 'pem',
        ...(configuredPassword ? { passphrase: configuredPassword } : {}),
      });
      ENCRYPTED_PRIVATE_KEY_CONTENTS = configuredPrivateKey
        .export({
          type: 'pkcs8',
          format: 'pem',
          cipher: 'aes-256-cbc',
          passphrase: PRIVATE_KEY_PASSWORD,
        })
        .toString();
      UNENCRYPTED_PRIVATE_KEY_CONTENTS = configuredPrivateKey
        .export({ type: 'pkcs8', format: 'pem' })
        .toString();
    });

    describe('tests/definitions/shared/authentication/private_key_auth.feature', () => {
      it('should authenticate using private file with password', async () => {
        // Given Authentication is set to JWT and private file with password is provided
        const privateKeyPath = createTempKeyFile(ENCRYPTED_PRIVATE_KEY_CONTENTS);
        const connection = createJwtConnection({
          privateKeyPath,
          privateKeyPass: PRIVATE_KEY_PASSWORD,
        });

        // When Trying to Connect
        await connection.connectAsync();
        destroyConnectionAfterTest(connection);

        // Then Login is successful and simple query can be executed
        expect(connection.isUp()).toBe(true);
        await verifySimpleQuery(connection);
      });

      it('should authenticate using unencrypted private key file', async () => {
        // Given Authentication is set to JWT and an unencrypted private key file is provided (no password)
        const privateKeyPath = createTempKeyFile(UNENCRYPTED_PRIVATE_KEY_CONTENTS);
        const connection = createJwtConnection({ privateKeyPath });

        // When Trying to Connect
        await connection.connectAsync();
        destroyConnectionAfterTest(connection);

        // Then Login is successful and simple query can be executed
        expect(connection.isUp()).toBe(true);
        await verifySimpleQuery(connection);
      });

      it('should fail JWT authentication when invalid private key provided', async () => {
        // Given Authentication is set to JWT and invalid private key file is provided
        const privateKeyPath = createTempKeyFile(await readFile(INVALID_RSA_KEY_PATH, 'utf8'));
        const connection = createJwtConnection({ privateKeyPath });

        // TODO(SNOW-3894966): this does not pin `sqlState` or `name`.
        //
        // When Trying to Connect
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          message: JWT_TOKEN_INVALID_MESSAGE,
          code: '390144',
        });

        // Then There is error returned
        await expectConnectionNotUp(connection, TERMINATED_QUERY);
      });

      it('should fail JWT authentication when private key password is wrong', async () => {
        // Given Authentication is set to JWT and an encrypted private key file is provided with the wrong password
        const privateKeyPath = createTempKeyFile(ENCRYPTED_PRIVATE_KEY_CONTENTS);
        const connection = createJwtConnection({
          privateKeyPath,
          privateKeyPass: 'invalid',
        });

        // TODO(SNOW-3894966): this does not pin `sqlState`, `name`, or `code`.
        //
        // When Trying to Connect
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          message: isRunningNewDriverWithBD('BD#74')
            ? 'Failed to login: Authentication failed'
            : 'error:1C800064:Provider routines::bad decrypt',
        });

        // Then There is error returned
        await expectConnectionNotUp(
          connection,
          isRunningNewDriverWithBD('BD#61') ? TERMINATED_QUERY : NEVER_ESTABLISHED_QUERY,
        );
      });

      it('should authenticate using private_key as pem string', async () => {
        // Given Authentication is set to JWT and private key is provided as plaintext PEM
        const connection = createJwtConnection({
          privateKey: UNENCRYPTED_PRIVATE_KEY_CONTENTS,
        });

        // When Trying to Connect
        await connection.connectAsync();
        destroyConnectionAfterTest(connection);

        // Then Login is successful and simple query can be executed
        expect(connection.isUp()).toBe(true);
        await verifySimpleQuery(connection);
      });

      it('should authenticate using private_key as base64 string', async () => {
        // Given Authentication is set to JWT and private key is provided as base64-encoded string
        const privateKey = createPrivateKey({
          key: UNENCRYPTED_PRIVATE_KEY_CONTENTS,
          format: 'pem',
        })
          .export({ type: 'pkcs8', format: 'der' })
          .toString('base64');

        // When Trying to Connect
        if (isRunningNewDriverWithBD('BD#77')) {
          const connection = createJwtConnection({ privateKey });
          await connection.connectAsync();
          destroyConnectionAfterTest(connection);

          // Then Login is successful and simple query can be executed
          expect(connection.isUp()).toBe(true);
          await verifySimpleQuery(connection);
        } else {
          expect(() => createJwtConnection({ privateKey })).toThrow(
            'Invalid private key. The specified value must be a PEM-formatted private key.',
          );
        }
      });

      it('should automatically update authenticator to JWT if key pair params present', async () => {
        // Given private key or private key file is provided and authenticator is not explicitly set
        const options = {
          privateKey: UNENCRYPTED_PRIVATE_KEY_CONTENTS,
        };

        // When Trying to Connect
        if (isRunningNewDriverWithBD('BD#78')) {
          const connection = createJwtConnection(options, { includeAuthenticator: false });
          await connection.connectAsync();
          destroyConnectionAfterTest(connection);

          // Then Connector changes authenticator to JWT and login is successful and simple query can be executed
          expect(connection.isUp()).toBe(true);
          await verifySimpleQuery(connection);
        } else {
          expect(() => createJwtConnection(options, { includeAuthenticator: false })).toThrow(
            'A password must be specified.',
          );
        }
      });

      it('should fail JWT authentication when private_key is not PEM or base64', async () => {
        // Given Authentication is set to JWT and private_key is not PEM or base64
        const options = {
          account: 'testaccount',
          username: 'alice',
          authenticator: 'SNOWFLAKE_JWT',
          privateKey: 'invalidKey',
        };

        // When Trying to Connect
        if (isRunningNewDriverWithBD('BD#73')) {
          const connection = snowflake.createConnection(options);
          // TODO(SNOW-3894966): this does not pin `sqlState`, `name`, or `code`.
          //
          // Then There is error returned
          await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
            message:
              "Configuration error: Invalid value '(redacted)' for parameter 'private_key' - Could not decode base64 private key: Invalid padding",
          });
          await expectConnectionNotUp(connection, TERMINATED_QUERY);
        } else {
          // Then There is error returned
          expect(() => snowflake.createConnection(options)).toThrow(
            'Invalid private key. The specified value must be a PEM-formatted private key.',
          );
        }
      });

      it('should fail JWT authentication when invalid private_key PEM string provided', async () => {
        // Given Authentication is set to JWT and invalid private_key PEM string is provided
        const connection = createJwtConnection({
          privateKey: await readFile(INVALID_RSA_KEY_PATH, 'utf8'),
        });

        // TODO(SNOW-3894966): this does not pin `sqlState` or `name`.
        //
        // When Trying to Connect
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          message: JWT_TOKEN_INVALID_MESSAGE,
          code: '390144',
        });

        // Then There is error returned
        await expectConnectionNotUp(connection, TERMINATED_QUERY);
      });

      it('should fail JWT authentication when no private file provided', async () => {
        // Given Authentication is set to JWT
        const options = {
          account: 'testaccount',
          username: 'alice',
          authenticator: 'SNOWFLAKE_JWT',
        };
        const connection = snowflake.createConnection(options);

        // When Trying to Connect with no private file provided
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          message: isRunningNewDriverWithBD('BD#76')
            ? "Configuration error: Configuration validation failed (1 issue(s)): private_key: Missing required parameter: 'private_key' or 'private_key_file'"
            : 'secretOrPrivateKey must have a value',
        });

        // Then There is error returned
        await expectConnectionNotUp(
          connection,
          isRunningNewDriverWithBD('BD#61') ? TERMINATED_QUERY : NEVER_ESTABLISHED_QUERY,
        );
      });
    });

    describe('WireMock-based JWT tests', () => {
      let wiremock: WiremockServer;

      beforeAll(async () => {
        wiremock = await WiremockServer.spawn();
      });

      beforeEach(async () => {
        await wiremock.reset();
        await wiremock.stub([loginSuccess(), logoutSuccess()]);
      });

      afterAll(async () => {
        await wiremock.destroy();
      });

      it('should send AUTHENTICATOR=SNOWFLAKE_JWT when authenticator is omitted', async () => {
        const options = {
          account: 'testaccount',
          username: 'alice',
          privateKey: UNENCRYPTED_PRIVATE_KEY_CONTENTS,
          ...wiremock.connectionOptions,
        };

        if (isRunningNewDriverWithBD('BD#78')) {
          const connection = createJwtConnection(options, { includeAuthenticator: false });
          await connection.connectAsync();
          destroyConnectionAfterTest(connection);

          const [loginRequest] = await wiremock.findRequests('/session/v1/login-request.*');
          const { data } = JSON.parse(loginRequest.body) as {
            data: { AUTHENTICATOR?: string };
          };
          expect(data.AUTHENTICATOR).toBe('SNOWFLAKE_JWT');
        } else {
          expect(() => createJwtConnection(options, { includeAuthenticator: false })).toThrow(
            'A password must be specified.',
          );
        }
      });
    });

    it('should authenticate using an inline encrypted private key', async () => {
      const connection = createJwtConnection({
        privateKey: ENCRYPTED_PRIVATE_KEY_CONTENTS,
        privateKeyPass: PRIVATE_KEY_PASSWORD,
      });
      await connection.connectAsync();
      destroyConnectionAfterTest(connection);
      expect(connection.isUp()).toBe(true);
      await verifySimpleQuery(connection);
    });
  });
});
