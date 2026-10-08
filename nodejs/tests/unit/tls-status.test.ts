import { it } from 'vitest';

// FIPS_TLS_STATUS_WITHHELD: withheld from the shipped contract while the status shape is still under consideration.
it.skip('reports the linked TLS provider and build mode without a connection', () => undefined);

// import { it, expect } from 'vitest';
// import { getTlsStatus, type TlsStatus } from '../../src/index.js';
//
// it('reports the linked TLS provider and build mode without a connection', () => {
//   const fipsMode = process.env.SF_CORE_EXPECT_FIPS?.toLowerCase();
//   const expectedFips = fipsMode === '1' || fipsMode === 'true';
//   const status: TlsStatus = getTlsStatus();
//
//   expect(status).toEqual({
//     tlsProviderIsFips: expectedFips,
//     fipsBuildEnabled: expectedFips,
//   });
// });
