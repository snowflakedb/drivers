import { it, expect } from 'vitest';
import { getTlsStatus, type TlsStatus } from '../../src/index.js';

it('reports the linked TLS provider and build mode without a connection', () => {
  const fipsTlsMode = process.env.SF_CORE_EXPECT_FIPS_TLS?.toLowerCase();
  const expectedFipsTls = fipsTlsMode === '1' || fipsTlsMode === 'true';
  const status: TlsStatus = getTlsStatus();

  expect(status).toEqual({
    tlsProviderIsFips: expectedFipsTls,
    fipsTlsBuildEnabled: expectedFipsTls,
  });
});
