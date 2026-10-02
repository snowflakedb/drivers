import type { ConnectionOptions } from 'snowflake-sdk';
import { beforeAll, describe, expect, it } from 'vitest';
import { createLiveConnection } from '../utils/fixtures.js';
import getTestParameter from '../utils/getTestParameter.js';
import { executeAsync } from '../utils/index.js';

describe('Workload identity authentication', { tags: ['requires_wif_vm'] }, () => {
  let WIF_PROVIDER: NonNullable<ConnectionOptions['workloadIdentityProvider']>;

  beforeAll(() => {
    WIF_PROVIDER = getTestParameter('SNOWFLAKE_TEST_WIF_PROVIDER', true) as NonNullable<
      ConnectionOptions['workloadIdentityProvider']
    >;
  });

  it('should authenticate with the cloud identity of the host', async () => {
    const expectedUser = getTestParameter('SNOWFLAKE_TEST_WIF_USER', true);
    const connection = await createLiveConnection({
      account: getTestParameter('SNOWFLAKE_TEST_WIF_ACCOUNT', true),
      host: getTestParameter('SNOWFLAKE_TEST_HOST', true),
      authenticator: 'WORKLOAD_IDENTITY',
      workloadIdentityProvider: WIF_PROVIDER,
    });

    const { rows } = await executeAsync(connection, 'SELECT CURRENT_USER() AS USERNAME');
    expect(rows).toEqual([{ USERNAME: expectedUser }]);
  });
});
