import { describe, expect, it } from 'vitest';
import type { Connection } from '../../types/sdk-types.js';
import getTestParameter from '../utils/getTestParameter.js';
import {
  baseConnectionOptions,
  connectAsyncWithErrorBD,
  isRunningNewDriverWithBD,
  snowflake,
} from '../utils/index.js';
import { verifySimpleQuery } from '../utils/query.js';
import { destroyConnectionAfterTest, expectConnectionNotUp, TERMINATED_QUERY } from './utils.js';

const INCORRECT_USERNAME_OR_PASSWORD_MESSAGE = 'Incorrect username or password was specified';

function createPasswordConnection(overrides: Record<string, unknown> = {}): Connection {
  return snowflake.createConnection({
    ...baseConnectionOptions,
    ...overrides,
  });
}

describe(
  'tests/definitions/shared/authentication/user_password.feature',
  { tags: ['requires_no_mfa'] },
  () => {
    const WRONG_PASSWORD = 'definitely_not_a_valid_password_12345';

    it('should authenticate using username and password', async () => {
      // Given Authentication is set to default (snowflake) with valid username and password
      const connection = createPasswordConnection({
        password: getTestParameter('SNOWFLAKE_TEST_PASSWORD', true),
      });

      // When Trying to Connect
      await connection.connectAsync();
      destroyConnectionAfterTest(connection);

      // Then Login is successful and simple query can be executed
      expect(connection.isUp()).toBe(true);
      await verifySimpleQuery(connection);
    });

    it('should authenticate using explicit snowflake authenticator', async () => {
      // Given Authentication is explicitly set to snowflake with valid username and password
      const connection = createPasswordConnection({
        password: getTestParameter('SNOWFLAKE_TEST_PASSWORD', true),
        authenticator: 'snowflake',
      });

      // When Trying to Connect
      await connection.connectAsync();
      destroyConnectionAfterTest(connection);

      // Then Login is successful and simple query can be executed
      expect(connection.isUp()).toBe(true);
      await verifySimpleQuery(connection);
    });

    it('should fail authentication when wrong password is provided', async () => {
      // Given Authentication is set to default with valid username and wrong password
      const connection = createPasswordConnection({ password: WRONG_PASSWORD });

      // When Trying to Connect
      if (isRunningNewDriverWithBD('BD#125')) {
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          name: 'OperationFailedError',
          code: '390100',
          message: expect.stringContaining(INCORRECT_USERNAME_OR_PASSWORD_MESSAGE),
          sqlState: '28000',
        });
      } else {
        const error = await connectAsyncWithErrorBD(connection).catch((caught: unknown) => caught);
        expect(error).toMatchObject({
          name: 'OperationFailedError',
          code: '390100',
          message: expect.stringContaining(INCORRECT_USERNAME_OR_PASSWORD_MESSAGE),
        });
        expect((error as { sqlState?: unknown }).sqlState).toBeUndefined();
      }

      // Then There is error returned
      await expectConnectionNotUp(connection, TERMINATED_QUERY);
    });
  },
);

describe('password inputs', { tags: ['requires_no_mfa'] }, () => {
  const MISSING_PASSWORD_MESSAGE =
    "Configuration error: Configuration validation failed (1 issue(s)): password: Missing required parameter 'password' for password authentication";

  it('should reject an empty password', async () => {
    const connection = createPasswordConnection({ password: '' });
    if (isRunningNewDriverWithBD('BD#126')) {
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        name: 'InvalidParameterError',
        code: undefined,
        message: MISSING_PASSWORD_MESSAGE,
      });
    } else {
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        name: 'OperationFailedError',
        code: '390100',
        message: expect.stringContaining(INCORRECT_USERNAME_OR_PASSWORD_MESSAGE),
      });
    }
  });

  it.each([
    { label: 'an omitted password', overrides: {} },
    { label: 'an undefined password', overrides: { password: undefined } },
    { label: 'a null password', overrides: { password: null } },
  ])('should reject $label', async ({ overrides }) => {
    if (isRunningNewDriverWithBD('BD#101')) {
      const connection = createPasswordConnection(overrides);
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        name: 'InvalidParameterError',
        code: undefined,
        message: MISSING_PASSWORD_MESSAGE,
      });
    } else {
      expect(() => createPasswordConnection(overrides)).toThrow(
        expect.objectContaining({
          name: 'MissingParameterError',
          code: 404005,
          message: 'A password must be specified.',
        }),
      );
    }
  });
});
