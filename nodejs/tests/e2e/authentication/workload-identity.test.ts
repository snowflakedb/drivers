import {
  afterAll,
  afterEach,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  onTestFinished,
  vi,
} from 'vitest';
import type { Connection, ConnectionOptions } from '../../types/sdk-types.js';
import { createLiveConnection } from '../utils/fixtures.js';
import getTestParameter from '../utils/getTestParameter.js';
import {
  connectAsyncWithErrorBD,
  destroyConnectionAsync,
  executeAsync,
  IS_RUNNING_FOR_OLD_DRIVER,
  isRunningNewDriverWithBD,
  snowflake,
  TEST_CONNECTION_OPTIONS,
} from '../utils/index.js';
import { verifySimpleQuery } from '../utils/query.js';
import { loginSuccess, logoutSuccess, WiremockServer } from '../utils/wiremock/index.js';

type WorkloadIdentityProvider = NonNullable<ConnectionOptions['workloadIdentityProvider']>;

const PROVIDER = getTestParameter('SNOWFLAKE_TEST_WIF_PROVIDER')?.toUpperCase();
const IS_AWS_PROVIDER = PROVIDER === 'AWS';
const IS_AZURE_PROVIDER = PROVIDER === 'AZURE';
const IS_GCP_PROVIDER = PROVIDER === 'GCP';
// Host check in ensure_allowed_host runs before OIDC JWT parse. Loopback is
// rejected, so tests that must reach attestation use a recognized suffix.
const WIF_ATTESTATION_HOST = 'wif-validation.snowflakecomputing.com';

function createWifConnection(
  connectionOptionsOverrides: Partial<ConnectionOptions> = {},
): Connection {
  const entraResource = getTestParameter('SNOWFLAKE_TEST_WIF_ENTRA_RESOURCE');
  return snowflake.createConnection({
    account: getTestParameter('SNOWFLAKE_TEST_WIF_ACCOUNT', true),
    host: getTestParameter('SNOWFLAKE_TEST_HOST', true),
    authenticator: 'WORKLOAD_IDENTITY',
    workloadIdentityProvider: getTestParameter(
      'SNOWFLAKE_TEST_WIF_PROVIDER',
      true,
    ) as WorkloadIdentityProvider,
    ...(entraResource ? { workloadIdentityAzureEntraIdResource: entraResource } : {}),
    ...connectionOptionsOverrides,
  });
}

function destroyConnectionAfterTest(connection: Connection): void {
  onTestFinished(async () => {
    await destroyConnectionAsync(connection);
  });
}

async function createLiveWifConnection(
  connectionOptionsOverrides: Partial<ConnectionOptions> = {},
  shouldCleanupAfterTest = true,
): Promise<Connection> {
  const connection = createWifConnection(connectionOptionsOverrides);
  await connection.connectAsync();
  if (shouldCleanupAfterTest) {
    destroyConnectionAfterTest(connection);
  }
  return connection;
}

// TODO(SNOW-4273458): investigate exposing the authenticated user from the login response.
async function verifyCurrentUser(connection: Connection, expectedUser: string): Promise<void> {
  const { rows } = await executeAsync(connection, 'SELECT CURRENT_USER() AS USERNAME');
  expect(rows).toEqual([{ USERNAME: expectedUser }]);
}

function formatSingleIssueConfigurationValidationMessage(
  parameter: string,
  detail: string,
): string {
  return `Configuration error: Configuration validation failed (1 issue(s)): ${parameter}: ${detail}`;
}

describe('tests/definitions/shared/authentication/workload_identity.feature', () => {
  // TODO(SNOW-4232907): these BD#106 branches pin the snake_case wording.
  // Drop them once validation errors use the invoking wrapper's parameter name.
  describe('Workload identity wrapper validation', () => {
    const OLD_INVALID_WIF_PROVIDER_ERROR = {
      name: 'InvalidParameterError',
      code: 404062,
      message:
        'Invalid authenticator: WORKLOAD_IDENTITY parameters. workloadIdentityProvider must be one of: AWS, AZURE, GCP, OIDC',
    };

    it('should fail WORKLOAD_IDENTITY when provider is missing', async () => {
      // Given Authentication is set to WORKLOAD_IDENTITY but WORKLOAD_IDENTITY_PROVIDER is absent
      const connection = snowflake.createConnection({
        account: 'test-account',
        host: WIF_ATTESTATION_HOST,
        authenticator: 'WORKLOAD_IDENTITY',
      });

      // When Trying to Connect
      const connect = connectAsyncWithErrorBD(connection);

      // Then Connection fails with a missing-parameter error citing workload_identity_provider
      await expect(connect).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#106')
          ? {
              name: 'InvalidParameterError',
              message: formatSingleIssueConfigurationValidationMessage(
                'workload_identity_provider',
                "Missing required parameter 'workload_identity_provider' for WORKLOAD_IDENTITY authentication. Allowed values: AWS, AZURE, GCP, OIDC",
              ),
            }
          : OLD_INVALID_WIF_PROVIDER_ERROR,
      );
    });

    it('should fail WORKLOAD_IDENTITY when provider is an invalid value', async () => {
      // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is an invalid value
      const connection = snowflake.createConnection({
        account: 'test-account',
        host: WIF_ATTESTATION_HOST,
        authenticator: 'WORKLOAD_IDENTITY',
        workloadIdentityProvider: 'INVALID_CLOUD' as never,
      });

      // When Trying to Connect
      const connect = connectAsyncWithErrorBD(connection);

      // Then Connection fails with an invalid-parameter error citing workload_identity_provider
      await expect(connect).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#106')
          ? {
              name: 'InvalidParameterError',
              message: formatSingleIssueConfigurationValidationMessage(
                'workload_identity_provider',
                "Invalid workload_identity_provider 'INVALID_CLOUD'. Allowed values: AWS, AZURE, GCP, OIDC",
              ),
            }
          : OLD_INVALID_WIF_PROVIDER_ERROR,
      );
    });

    it('should fail OIDC WIF when token is missing', async () => {
      // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC but token is absent
      const connection = snowflake.createConnection({
        account: 'test-account',
        host: WIF_ATTESTATION_HOST,
        authenticator: 'WORKLOAD_IDENTITY',
        workloadIdentityProvider: 'OIDC',
      });

      // When Trying to Connect
      const connect = connectAsyncWithErrorBD(connection);

      // Then Connection fails with a missing-parameter error citing token
      await expect(connect).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#106')
          ? {
              name: 'InvalidParameterError',
              message: formatSingleIssueConfigurationValidationMessage(
                'token',
                "Missing required parameter 'token' (or 'token_file_path') for OIDC workload identity authentication",
              ),
            }
          : {
              name: 'InvalidParameterError',
              code: 404062,
              message:
                'Invalid authenticator: WORKLOAD_IDENTITY parameters. workloadIdentityProvider: OIDC requires token in connection options',
            },
      );
    });

    it('should fail OIDC WIF on a disallowed host when config passes param checks', async () => {
      const connection = snowflake.createConnection({
        account: 'test-account',
        host: '127.0.0.1',
        authenticator: 'WORKLOAD_IDENTITY',
        workloadIdentityProvider: 'OIDC',
        token: 'e30.eyJzdWIiOiJzdmMifQ.signature',
      });

      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#110')
          ? {
              name: 'Error',
              message:
                "Failed to login: Workload Identity Federation attestation failed: Refusing to send a Workload Identity attestation to '127.0.0.1': host is not snowflakecomputing.com/.cn/.mil (or an SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES entry)",
            }
          : {
              name: 'Error',
              message:
                'WORKLOAD_IDENTITY requires a recognized Snowflake host. Got: 127.0.0.1. This allowlist can be extended via the SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES environment variable.',
            },
      );
    });

    it('should fail OIDC WIF when token is malformed', async () => {
      // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC
      const connection = snowflake.createConnection({
        account: 'test-account',
        host: WIF_ATTESTATION_HOST,
        authenticator: 'WORKLOAD_IDENTITY',
        workloadIdentityProvider: 'OIDC',
        // And Token is set to a malformed value that is not a valid JWT
        token: 'not-a-valid-jwt',
      });

      // When Trying to Connect
      const connect = connectAsyncWithErrorBD(connection);

      // The JWT sentence is the cause: AttestationError displays "OIDC attestation failed"
      // without its source.
      // Then Connection fails with an attestation error indicating a malformed token
      await expect(connect).rejects.toMatchObject(
        isRunningNewDriverWithBD('BD#107')
          ? {
              name: 'Error',
              message:
                'Failed to login: Workload Identity Federation attestation failed: OIDC attestation failed',
              cause: {
                name: 'Error',
                message: 'OIDC token is not a valid JWT (expected header.payload.signature)',
              },
            }
          : {
              name: 'RequestFailedError',
              message: 'Request to Snowflake failed.',
            },
      );
    });
  });

  describe('Workload identity live authentication', { tags: ['requires_wif_vm'] }, () => {
    let wifProvider: WorkloadIdentityProvider;

    beforeAll(() => {
      wifProvider = getTestParameter(
        'SNOWFLAKE_TEST_WIF_PROVIDER',
        true,
      ) as WorkloadIdentityProvider;
    });

    it('should authenticate with cloud identity using WIF', async () => {
      const expectedUser = getTestParameter('SNOWFLAKE_TEST_WIF_USER', true);
      // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is configured
      const connectionOptionsOverrides: Partial<ConnectionOptions> = {
        authenticator: 'WORKLOAD_IDENTITY',
        workloadIdentityProvider: wifProvider,
      };

      // When Trying to Connect
      const connection = await createLiveWifConnection(connectionOptionsOverrides);

      // Then Login is successful and a simple query can be executed
      await verifySimpleQuery(connection);
      await verifyCurrentUser(connection, expectedUser);
    });

    it('should authenticate with WIF using lowercase authenticator', async () => {
      const expectedUser = getTestParameter('SNOWFLAKE_TEST_WIF_USER', true);
      // Given Authentication is set to workload_identity (lowercase) and a valid provider is configured
      const connectionOptionsOverrides: Partial<ConnectionOptions> = {
        authenticator: 'workload_identity',
        workloadIdentityProvider: wifProvider,
      };

      // When Trying to Connect
      const connection = await createLiveWifConnection(connectionOptionsOverrides);

      // Then Login is successful and a simple query can be executed
      await verifySimpleQuery(connection);
      await verifyCurrentUser(connection, expectedUser);
    });

    it.skipIf(IS_AZURE_PROVIDER && !isRunningNewDriverWithBD('BD#109'))(
      'should authenticate with WIF using impersonation',
      async () => {
        // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_IMPERSONATION_PATH is configured
        const impersonationPath = getTestParameter('SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH', true);
        const expectedUser = getTestParameter('SNOWFLAKE_TEST_WIF_USERNAME_IMPERSONATION', true);
        const wifOverrides = {
          workloadIdentityImpersonationPath: impersonationPath.split(','),
        };

        // When Trying to Connect
        const connection = await createLiveWifConnection(wifOverrides);

        // Then Login is successful and a simple query can be executed
        await verifySimpleQuery(connection);
        await verifyCurrentUser(connection, expectedUser);
      },
    );

    it.skipIf(!IS_AZURE_PROVIDER || isRunningNewDriverWithBD('BD#109'))(
      'should fail WIF impersonation on Azure with the old driver',
      async () => {
        // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_IMPERSONATION_PATH is configured
        const impersonationPath = getTestParameter('SNOWFLAKE_TEST_WIF_IMPERSONATION_PATH', true);
        const connection = createWifConnection({
          workloadIdentityImpersonationPath: impersonationPath.split(','),
        });

        // When Trying to Connect
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          name: 'Error',
          message: 'workloadIdentityImpersonationPath for AZURE is not supported',
        });
      },
    );

    describe.skipIf(!IS_AWS_PROVIDER)('AWS workload identity modes', () => {
      const AWS_OUTBOUND_TOKEN_ENV = 'SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN';

      afterEach(() => {
        vi.unstubAllEnvs();
      });

      it('should authenticate AWS WIF using pre-signed GetCallerIdentity by default', async () => {
        const expectedUser = getTestParameter('SNOWFLAKE_TEST_WIF_USER', true);
        // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
        const connectionOptionsOverrides: Partial<ConnectionOptions> = {
          authenticator: 'WORKLOAD_IDENTITY',
          workloadIdentityProvider: 'AWS',
        };

        // And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is not set
        vi.stubEnv(AWS_OUTBOUND_TOKEN_ENV, undefined);

        // When Trying to Connect
        const connection = await createLiveWifConnection(connectionOptionsOverrides);

        // Then Login is successful and a simple query can be executed
        await verifySimpleQuery(connection);
        await verifyCurrentUser(connection, expectedUser);
      });

      // The old driver ignores both outbound opt-ins and stays on the pre-signed request.
      // That request cannot log in as the issuer-only user, so these tests run on the new driver.
      const issuerRoleArn =
        'arn:aws:iam::376129840140:role/drivers-wif-automated-tests-with-issuer';
      const issuerUser = 'TEST_WIF_E2E_AWS_WITH_ISSUER';

      describe.skipIf(IS_RUNNING_FOR_OLD_DRIVER)('AWS GetWebIdentityToken opt-ins', () => {
        it('should authenticate AWS WIF using GetWebIdentityToken when opted in', async () => {
          // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
          const connectionOptionsOverrides: Partial<ConnectionOptions> = {
            authenticator: 'WORKLOAD_IDENTITY',
            workloadIdentityProvider: 'AWS',
            workloadIdentityImpersonationPath: [issuerRoleArn],
          };

          // And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is set to true
          vi.stubEnv(AWS_OUTBOUND_TOKEN_ENV, 'true');

          // When Trying to Connect
          const connection = await createLiveWifConnection(connectionOptionsOverrides);

          // Then Login is successful and a simple query can be executed
          await verifySimpleQuery(connection);
          await verifyCurrentUser(connection, issuerUser);
        });

        it('should authenticate AWS WIF using GetWebIdentityToken when workload_identity_aws_use_outbound_token is true', async () => {
          // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is AWS
          const connectionOptionsOverrides: Partial<ConnectionOptions> = {
            authenticator: 'WORKLOAD_IDENTITY',
            workloadIdentityProvider: 'AWS',
            workloadIdentityImpersonationPath: [issuerRoleArn],
          };

          // And SNOWFLAKE_ENABLE_AWS_WIF_OUTBOUND_TOKEN is not set
          vi.stubEnv(AWS_OUTBOUND_TOKEN_ENV, undefined);

          // And workload_identity_aws_use_outbound_token is set to true
          const outboundConnectionOptions: Partial<ConnectionOptions> = {
            ...connectionOptionsOverrides,
            workloadIdentityAwsUseOutboundToken: true,
          };

          // When Trying to Connect
          const connection = await createLiveWifConnection(outboundConnectionOptions);

          // Then Login is successful and a simple query can be executed
          await verifySimpleQuery(connection);
          await verifyCurrentUser(connection, issuerUser);
        });
      });
    });

    describe.skipIf(!IS_GCP_PROVIDER)('OIDC workload identity', () => {
      it('should authenticate OIDC WIF using a GCP metadata identity token', async () => {
        const expectedUser = 'TEST_WIF_E2E_OIDC';
        // Given Authentication is set to WORKLOAD_IDENTITY and WORKLOAD_IDENTITY_PROVIDER is OIDC
        const connectionOptionsOverrides: Partial<ConnectionOptions> = {
          authenticator: 'WORKLOAD_IDENTITY',
          workloadIdentityProvider: 'OIDC',
        };

        // And A GCP metadata identity token is supplied for the snowflakecomputing.com audience
        const response = await fetch(
          'http://169.254.169.254/computeMetadata/v1/instance/service-accounts/default/identity?audience=snowflakecomputing.com',
          { headers: { 'Metadata-Flavor': 'Google' } },
        );
        if (!response.ok) {
          throw new Error(`GCP metadata identity request failed with HTTP ${response.status}`);
        }
        const token = (await response.text()).trim();
        if (!token) {
          throw new Error('GCP metadata identity request returned an empty token');
        }
        connectionOptionsOverrides.token = token;

        // When Trying to Connect
        const connection = await createLiveWifConnection(connectionOptionsOverrides);

        // Then Login is successful and a simple query can be executed
        await verifySimpleQuery(connection);
        await verifyCurrentUser(connection, expectedUser);
      });
    });
  });
});

describe('Workload identity wrapper rules', () => {
  // TODO(SNOW-4232907): same wording gap as the provider and token cases above.
  it.each([
    {
      option: 'workloadIdentityProvider',
      value: 'AWS',
      snakeCaseParam: 'workload_identity_provider',
    },
    {
      option: 'workloadIdentityAzureEntraIdResource',
      value: 'api://00000000-0000-0000-0000-000000000001',
      snakeCaseParam: 'workload_identity_entra_resource',
    },
    {
      option: 'workloadIdentityImpersonationPath',
      value: ['arn:aws:iam::123456789012:role/A'],
      snakeCaseParam: 'workload_identity_impersonation_path',
    },
    {
      option: 'workloadIdentityAwsUseOutboundToken',
      value: true,
      snakeCaseParam: 'workload_identity_aws_use_outbound_token',
    },
  ])(
    'should reject $option under a non-WIF authenticator',
    async ({ option, value, snakeCaseParam }) => {
      if (isRunningNewDriverWithBD('BD#108')) {
        const connection = snowflake.createConnection({
          ...TEST_CONNECTION_OPTIONS,
          [option]: value,
        });
        await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
          name: 'InvalidParameterError',
          message: formatSingleIssueConfigurationValidationMessage(
            snakeCaseParam,
            `${snakeCaseParam} was set but authenticator was not set to WORKLOAD_IDENTITY`,
          ),
        });
        return;
      }

      const connection = await createLiveConnection({ [option]: value });
      await verifySimpleQuery(connection);
    },
  );

  // TODO(SNOW-4232907): same wording gap as the provider and token cases above.
  it('should reject an impersonation path for OIDC WIF', async () => {
    const connection = snowflake.createConnection({
      account: 'test-account',
      host: WIF_ATTESTATION_HOST,
      authenticator: 'WORKLOAD_IDENTITY',
      workloadIdentityProvider: 'OIDC',
      workloadIdentityImpersonationPath: ['service-account@snowflake.com'],
      token: 'e30.eyJzdWIiOiJzdmMifQ.signature',
    });

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
      isRunningNewDriverWithBD('BD#106')
        ? {
            name: 'InvalidParameterError',
            message: formatSingleIssueConfigurationValidationMessage(
              'workload_identity_impersonation_path',
              'workload_identity_impersonation_path is currently only supported for GCP, AWS, and AZURE',
            ),
          }
        : {
            name: 'Error',
            message: 'workloadIdentityImpersonationPath for OIDC is not supported',
          },
    );
  });

  it('should reject Azure WIF two-hop impersonation', async () => {
    const connection = snowflake.createConnection({
      account: 'test-account',
      host: WIF_ATTESTATION_HOST,
      authenticator: 'WORKLOAD_IDENTITY',
      workloadIdentityProvider: 'AZURE',
      workloadIdentityImpersonationPath: [
        '00000000-0000-0000-0000-000000000001',
        '00000000-0000-0000-0000-000000000002',
      ],
    });

    await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject(
      isRunningNewDriverWithBD('BD#109')
        ? {
            name: 'InvalidParameterError',
            message: formatSingleIssueConfigurationValidationMessage(
              'workload_identity_impersonation_path',
              "Azure WIF impersonation only supports a single service principal (single-hop). 'workload_identity_impersonation_path' must contain exactly one client_id, got 2.",
            ),
          }
        : {
            name: 'Error',
            message: 'workloadIdentityImpersonationPath for AZURE is not supported',
          },
    );
  });
});

// stubEnv(undefined) removes the key from process.env, which sf_core reads.
// onTestFinished runs after each attempt, including retries, and after the
// connection cleanup registered later in the test.
function stubAwsRegionFallbackEnv(): void {
  vi.stubEnv('AWS_REGION', undefined);
  vi.stubEnv('AWS_DEFAULT_REGION', undefined);
  vi.stubEnv('AWS_SESSION_TOKEN', undefined);
  vi.stubEnv('AWS_PROFILE', undefined);
  vi.stubEnv('AWS_ACCESS_KEY_ID', 'AKIATESTEXAMPLE');
  vi.stubEnv('AWS_SECRET_ACCESS_KEY', 'test-secret');
  vi.stubEnv('AWS_EC2_METADATA_SERVICE_ENDPOINT', 'http://127.0.0.1:1');
  vi.stubEnv('AWS_EC2_METADATA_DISABLED', 'true');
  vi.stubEnv('SNOWFLAKE_WIF_ALLOWED_HOST_SUFFIXES', 'localhost');
  onTestFinished(() => {
    vi.unstubAllEnvs();
  });
}

describe('AWS WIF region fallback', () => {
  let wiremock: WiremockServer;

  beforeAll(async () => {
    wiremock = await WiremockServer.spawn();
  });

  beforeEach(async () => {
    await wiremock.reset();
  });

  afterAll(async () => {
    await wiremock.destroy();
  });

  it('should fall back to us-east-1 when AWS metadata has no region', async () => {
    stubAwsRegionFallbackEnv();
    await wiremock.stub([loginSuccess(), logoutSuccess()]);

    const connection = snowflake.createConnection({
      account: 'test-account',
      authenticator: 'WORKLOAD_IDENTITY',
      workloadIdentityProvider: 'AWS',
      ...wiremock.connectionOptions,
    });

    if (isRunningNewDriverWithBD('BD#139')) {
      await connection.connectAsync();
      onTestFinished(async () => {
        await destroyConnectionAsync(connection);
      });
      const [loginRequest] = await wiremock.findRequests('/session/v1/login-request.*');
      const token = JSON.parse(
        Buffer.from(JSON.parse(loginRequest.body).data.TOKEN, 'base64').toString('utf8'),
      ) as { url: string };
      expect(token.url).toBe(
        'https://sts.us-east-1.amazonaws.com/?Action=GetCallerIdentity&Version=2011-06-15',
      );
    } else {
      await expect(connectAsyncWithErrorBD(connection)).rejects.toMatchObject({
        name: 'Error',
        message:
          'Error making request to the metadata service: Error: connect ECONNREFUSED 127.0.0.1:1',
      });
      expect(await wiremock.findRequests('/session/v1/login-request.*')).toHaveLength(0);
    }
  });
});
