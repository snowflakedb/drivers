import { describe, expect, it } from 'vitest';
import { createLiveConnection } from '../utils/fixtures.js';
import { executeAsync } from '../utils/index.js';

describe('tests/definitions/shared/tls/crl_enabled.feature', () => {
  it('should connect and select with CRL enabled', async () => {
    // Given Snowflake client is logged in
    const connection = await createLiveConnection({ certRevocationCheckMode: 'ENABLED' });

    // When Query "SELECT 1" is executed
    const { rows } = await executeAsync(connection, 'SELECT 1');

    // Then the request attempt should be successful
    expect(rows).toHaveLength(1);
    expect(Object.values(rows[0])).toEqual([1]);
  });
});
