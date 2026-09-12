import { afterEach, describe, it, vi } from 'vitest';
import { createLiveConnection, createTempDir } from './utils/fixtures.js';

const UNUSED_CA_PEM = `-----BEGIN CERTIFICATE-----
MIIDFTCCAf2gAwIBAgIUUw1GCbZJNDWcAiJnEPZjMXJA344wDQYJKoZIhvcNAQEL
BQAwGjEYMBYGA1UEAwwPVW51c2VkIEV4dHJhIENBMB4XDTI2MDkwOTExNDQ0NVoX
DTM2MDkwNjExNDQ0NVowGjEYMBYGA1UEAwwPVW51c2VkIEV4dHJhIENBMIIBIjAN
BgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAklE1DlzqNDOZ9oER56dcVs6ihDJE
IleScA/0wH3OKPv4No2EeiUFIEgSQ2GLIgijoUx2wTSMZ3a8WEltuNhzrXrh/o8i
qirv2bkavPInPcka2Cl/1awRm88t0vjdjL3BcvJrl3zG1NjuyFOAJnsk/8a2jb5G
5RDwX2Gj6e3RgrkdsDP9wByizewSeVUay0gl3D4oQbyhhkjZKWZgr1zcjKj7POYX
K/yMrycnsNa1T9bQdG8djnN+kAJgn/2q4ljRyuAqBNq6GD9dG4r1W4c7UWJ01xP8
cZixnC9S1uCmft93ImvkWdv2RwwrV2lSb0bBhYFDaELBrnIsJEI4cqhaRQIDAQAB
o1MwUTAdBgNVHQ4EFgQUgrmEJM94AJePQvsb7i9CsY5RA20wHwYDVR0jBBgwFoAU
grmEJM94AJePQvsb7i9CsY5RA20wDwYDVR0TAQH/BAUwAwEB/zANBgkqhkiG9w0B
AQsFAAOCAQEAVVlBfXUcig24LBex/WZTMpE04p7rQij9lEzIujs4zCYvJOMWjaea
5MiJx4XjVcrXSwew4GJed9kK+YpH9piQU0k6CppdVX3vcT53X38/y8YKx3uavJhG
B0nl9BvXdFFB+mSOZ391twdpVnhWw9Uc/rK0jpr5xooVAMpWNgcaSn/CICD9xo1i
IFsMGT9Bbz89uoqVlglIOWjPEbp+UtVqBJ7ORhw1VxV64QkTOHn9u07YVatwNMNs
f5gtRKxtAPmRX/LwWYp7ifrVseqjUCQ+yXmPv404daU35W0YkFgRSaVO1z+cYJq9
kEJUD2DJGhy+ADL/hCgztbm7/0AMc64L1Q==
-----END CERTIFICATE-----
`;

describe('node TLS environment options', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it.todo('should disable certificate verification when NODE_TLS_REJECT_UNAUTHORIZED=0');

  // TODO:
  // This test is kind of pointless: it doesn't really check that NODE_EXTRA_CA_CERTS passed to core
  // Later we should do better tests with wiremocks over https and custom certs
  it('should keep default roots trusted when NODE_EXTRA_CA_CERTS points at an unused PEM', async () => {
    const tmpDir = createTempDir();
    const pemPath = tmpDir.writeFile('unused-ca.pem', UNUSED_CA_PEM);
    vi.stubEnv('NODE_EXTRA_CA_CERTS', pemPath);
    await createLiveConnection();
  });

  it('should ignore empty NODE_EXTRA_CA_CERTS', async () => {
    vi.stubEnv('NODE_EXTRA_CA_CERTS', '');
    await createLiveConnection();
  });
});
