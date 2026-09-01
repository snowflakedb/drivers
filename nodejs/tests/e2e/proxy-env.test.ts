import { describe, it } from 'vitest';

// These cases exercise the `useEnvProxy` global option (set via `configure({ useEnvProxy })`), which
// controls whether the driver reads proxy settings from the HTTP_PROXY / HTTPS_PROXY / NO_PROXY
// environment variables. Asserting that a connection actually routes (or refuses to route) through a
// proxy needs a network-mock / proxy test utility we don't have yet, so they are kept as todos
// rather than half-real assertions. Once that utility lands, implement them by pointing HTTPS_PROXY
// at the mock and observing whether the login request traverses it.
describe('proxy from environment variables (useEnvProxy)', () => {
  it.todo('routes traffic through HTTPS_PROXY by default (useEnvProxy defaults to true)');

  it.todo('ignores HTTPS_PROXY after configure({ useEnvProxy: false })');

  it.todo('honors NO_PROXY bypass entries when env proxy is active');

  it.todo('prefers an explicit proxyHost/proxyPort over the env proxy');
});
