import { afterAll, beforeAll, beforeEach, describe, expect, it, onTestFinished } from 'vitest';
import {
  acceptUntrustedHttps,
  createConnection,
  createLiveConnection,
  stubEnv,
} from '../utils/fixtures.js';
import {
  connectAsyncWithErrorBD,
  IS_RUNNING_FOR_OLD_DRIVER,
  isRunningNewDriverWithBD,
  resetGlobalConfig,
  snowflake,
} from '../utils/index.js';
import {
  type LoggedRequest,
  loginSuccess,
  logoutSuccess,
  WiremockServer,
} from '../utils/wiremock/index.js';

const TARGET_HOST = 'nonexistent.invalid';
const TARGET_PORT = 8090;
const PROXY_USER = 'proxy-user';
const PROXY_PASSWORD = 'proxy-password';

describe('proxy session options', () => {
  let wiremock: WiremockServer;

  beforeAll(async () => {
    snowflake.configure({ useEnvProxy: false });
    wiremock = await WiremockServer.spawnForwardProxy();
  });

  beforeEach(async () => {
    await wiremock.reset();
    await wiremock.stub(logoutSuccess());
  });

  afterAll(async () => {
    resetGlobalConfig();
    snowflake.configure({ useEnvProxy: true });
    await wiremock.destroy();
  });

  function getWiremockPort(proxy: WiremockServer): number {
    // @ts-ignore BD#2 - port is not typed in old driver ConnectionOptions
    return Number(proxy.connectionOptions.port);
  }

  function setHttpProxy(proxy: WiremockServer): void {
    stubEnv('HTTP_PROXY', `http://localhost:${getWiremockPort(proxy)}`);
  }

  function enableUseEnvProxy(): void {
    snowflake.configure({ useEnvProxy: true });
    onTestFinished(() => {
      snowflake.configure({ useEnvProxy: false });
    });
  }

  function proxyEndpoint() {
    return {
      proxyHost: 'localhost',
      proxyPort: getWiremockPort(wiremock),
    };
  }

  async function getLoginRequests(proxy: WiremockServer): Promise<LoggedRequest[]> {
    return proxy.findRequests('/session/v1/login-request.*');
  }

  function headerValue(headers: Record<string, string>, name: string): string | undefined {
    const found = Object.entries(headers).find(([key]) => key.toLowerCase() === name);
    return found?.[1];
  }

  describe('tests/definitions/shared/session/proxy.feature', () => {
    it('should route request through proxy when proxy host and port are configured', async () => {
      // Given a forward-proxy WireMock serving a canned login response
      await wiremock.stub(loginSuccess());

      // When the driver connects with the proxy host and port pointing at the proxy
      await createLiveConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
        proxyProtocol: 'http:',
        ...proxyEndpoint(),
      });

      // Then the connect succeeds and the proxy received the login request
      expect(await getLoginRequests(wiremock)).toHaveLength(1);
    });

    it.skipIf(IS_RUNNING_FOR_OLD_DRIVER)(
      'should route request through proxy when proxy url is configured',
      async () => {
        // Given a forward-proxy WireMock serving a canned login response
        await wiremock.stub(loginSuccess());

        // When the driver connects with the proxy url pointing at the proxy
        await createLiveConnection({
          host: TARGET_HOST,
          port: TARGET_PORT,
          protocol: 'http',
          proxy: `http://localhost:${getWiremockPort(wiremock)}`,
        });

        // Then the connect succeeds and the proxy received the login request
        expect(await getLoginRequests(wiremock)).toHaveLength(1);
      },
    );

    it('should bypass proxy when the target host is excluded from proxying', async () => {
      // Given a forward-proxy WireMock serving a canned login response
      await wiremock.stub(loginSuccess());

      // When the driver connects with a proxy and the target host excluded from proxying
      const connection = createConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
        proxyProtocol: 'http:',
        ...proxyEndpoint(),
        noProxy: TARGET_HOST,
        ...(IS_RUNNING_FOR_OLD_DRIVER ? { sfRetryMaxLoginRetries: 0 } : {}),
      });

      // Then the connect fails and the proxy received no requests
      await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow();
      expect(await getLoginRequests(wiremock)).toHaveLength(0);
    });

    it('should route request through proxy when proxy env vars are enabled', async () => {
      // Given HTTP_PROXY env var points at a forward-proxy WireMock
      await wiremock.stub(loginSuccess());
      enableUseEnvProxy();
      setHttpProxy(wiremock);

      // When the driver connects with proxy env vars enabled
      await createLiveConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
      });

      // Then the connect succeeds and the proxy received the login request
      expect(await getLoginRequests(wiremock)).toHaveLength(1);
    });

    it('should prefer explicit proxy_host over HTTP_PROXY env var', async () => {
      // Given two forward-proxy WireMock instances are running
      const envProxy = await WiremockServer.spawnForwardProxy();
      onTestFinished(async () => {
        await envProxy.destroy();
      });
      await wiremock.stub(loginSuccess());
      await envProxy.stub(loginSuccess());
      await envProxy.stub(logoutSuccess());

      // And HTTP_PROXY env var points at the second proxy
      enableUseEnvProxy();
      setHttpProxy(envProxy);

      // When the driver connects with proxy_host pointing at the first proxy
      await createLiveConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
        proxyProtocol: 'http:',
        ...proxyEndpoint(),
      });

      // Then only the first proxy received the login request
      expect(await getLoginRequests(wiremock)).toHaveLength(1);
      expect(await getLoginRequests(envProxy)).toHaveLength(0);
    });
  });

  describe('proxy', () => {
    it.skipIf(IS_RUNNING_FOR_OLD_DRIVER)(
      'should prefer proxyHost and proxyPort over the proxy URL',
      async () => {
        const decoy = await WiremockServer.spawnForwardProxy();
        onTestFinished(async () => {
          await decoy.destroy();
        });
        await wiremock.stub(loginSuccess());
        await decoy.stub([loginSuccess(), logoutSuccess()]);

        await createLiveConnection({
          host: TARGET_HOST,
          port: TARGET_PORT,
          protocol: 'http',
          proxy: `http://localhost:${getWiremockPort(decoy)}`,
          ...proxyEndpoint(),
        });

        expect(await getLoginRequests(wiremock)).toHaveLength(1);
        expect(await getLoginRequests(decoy)).toHaveLength(0);
      },
    );
  });

  describe('useEnvProxy', () => {
    it('should route through HTTP_PROXY when useEnvProxy is at its default', async () => {
      await wiremock.stub(loginSuccess());
      resetGlobalConfig();
      enableUseEnvProxy();
      setHttpProxy(wiremock);

      await createLiveConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
      });

      expect(await getLoginRequests(wiremock)).toHaveLength(1);
    });

    it('should ignore HTTP_PROXY after configure({ useEnvProxy: false })', async () => {
      await wiremock.stub(loginSuccess());
      setHttpProxy(wiremock);

      const connection = createConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
        ...(IS_RUNNING_FOR_OLD_DRIVER ? { sfRetryMaxLoginRetries: 0 } : {}),
      });
      await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow();

      expect(await getLoginRequests(wiremock)).toHaveLength(0);
    });

    it('should honor NO_PROXY bypass entries when env proxy is active', async () => {
      await wiremock.stub(loginSuccess());
      enableUseEnvProxy();
      setHttpProxy(wiremock);
      stubEnv('NO_PROXY', TARGET_HOST);

      const connection = createConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
        ...(IS_RUNNING_FOR_OLD_DRIVER ? { sfRetryMaxLoginRetries: 0 } : {}),
      });
      await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow();

      expect(await getLoginRequests(wiremock)).toHaveLength(0);
    });
  });

  describe('proxyProtocol', () => {
    it('should not reach the proxy when proxyProtocol is https', async () => {
      await wiremock.stub(loginSuccess());

      const connection = createConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
        ...proxyEndpoint(),
        proxyProtocol: 'https',
        ...(IS_RUNNING_FOR_OLD_DRIVER ? { sfRetryMaxLoginRetries: 0 } : {}),
      });
      await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow();

      expect(await getLoginRequests(wiremock)).toHaveLength(0);
    });
  });

  describe('https account URL', () => {
    it('should route an https login through the proxy when proxy host and port are configured', async () => {
      acceptUntrustedHttps();
      await wiremock.stub(loginSuccess());

      await createLiveConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'https',
        ...proxyEndpoint(),
      });

      expect(await getLoginRequests(wiremock)).toHaveLength(1);
    });

    it('should use https_proxy when HTTPS_PROXY is also set', async () => {
      const uppercaseProxy = await WiremockServer.spawnForwardProxy();
      onTestFinished(async () => {
        await uppercaseProxy.destroy();
      });
      await wiremock.stub(loginSuccess());
      await uppercaseProxy.stub([loginSuccess(), logoutSuccess()]);
      acceptUntrustedHttps();
      enableUseEnvProxy();
      stubEnv('https_proxy', `http://localhost:${getWiremockPort(wiremock)}`);
      stubEnv('HTTPS_PROXY', `http://localhost:${getWiremockPort(uppercaseProxy)}`);

      await createLiveConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'https',
      });

      const lowercaseHits = await getLoginRequests(wiremock);
      const uppercaseHits = await getLoginRequests(uppercaseProxy);
      if (isRunningNewDriverWithBD('BD#112')) {
        expect(lowercaseHits).toHaveLength(0);
        expect(uppercaseHits).toHaveLength(1);
      } else {
        expect(lowercaseHits).toHaveLength(1);
        expect(uppercaseHits).toHaveLength(0);
      }
    });

    it('should ignore HTTP_PROXY for an https login', async () => {
      acceptUntrustedHttps();
      enableUseEnvProxy();
      setHttpProxy(wiremock);
      await wiremock.stub(loginSuccess());

      const connection = createConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'https',
        ...(IS_RUNNING_FOR_OLD_DRIVER ? { sfRetryMaxLoginRetries: 0 } : {}),
      });
      await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow();

      expect(await getLoginRequests(wiremock)).toHaveLength(0);
    });
  });

  describe('http account URL through a connection-option proxy', () => {
    it('should send the login when proxyProtocol is left at its default', async () => {
      await wiremock.stub(loginSuccess());

      const options = {
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
        ...proxyEndpoint(),
      };
      if (isRunningNewDriverWithBD('BD#114')) {
        await createLiveConnection(options);
        expect(await getLoginRequests(wiremock)).toHaveLength(1);
      } else {
        const connection = createConnection({ ...options, sfRetryMaxLoginRetries: 0 });
        await expect(connectAsyncWithErrorBD(connection)).rejects.toThrow(
          'Protocol "http:" not supported. Expected "http"',
        );
        expect(await getLoginRequests(wiremock)).toHaveLength(0);
      }
    });

    it('should send Proxy-Authorization when proxyUser and proxyPassword are set', async () => {
      await wiremock.stub(loginSuccess());

      await createLiveConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
        proxyProtocol: 'http:',
        ...proxyEndpoint(),
        proxyUser: PROXY_USER,
        proxyPassword: PROXY_PASSWORD,
      });

      const requests = await getLoginRequests(wiremock);
      expect(requests).toHaveLength(1);
      const actual = headerValue(requests[0].headers, 'proxy-authorization');
      if (isRunningNewDriverWithBD('BD#114')) {
        const expected = `Basic ${Buffer.from(`${PROXY_USER}:${PROXY_PASSWORD}`).toString('base64')}`;
        expect(actual).toBe(expected);
      } else {
        expect(actual).toBeUndefined();
      }
    });
  });

  describe('proxy environment matching', () => {
    const SUBDOMAIN_HOST = 'account.nonexistent.invalid';

    async function isLoginProxied(host: string): Promise<boolean> {
      enableUseEnvProxy();
      setHttpProxy(wiremock);
      const connection = createConnection({
        host,
        port: TARGET_PORT,
        protocol: 'http',
        ...(IS_RUNNING_FOR_OLD_DRIVER ? { sfRetryMaxLoginRetries: 0 } : {}),
      });
      try {
        await connectAsyncWithErrorBD(connection);
      } catch {
        expect(await getLoginRequests(wiremock)).toHaveLength(0);
        return false;
      }
      expect(await getLoginRequests(wiremock)).toHaveLength(1);
      return true;
    }

    it('should use http_proxy when HTTP_PROXY is also set', async () => {
      const uppercaseProxy = await WiremockServer.spawnForwardProxy();
      onTestFinished(async () => {
        await uppercaseProxy.destroy();
      });
      await wiremock.stub(loginSuccess());
      await uppercaseProxy.stub([loginSuccess(), logoutSuccess()]);
      enableUseEnvProxy();
      stubEnv('http_proxy', `http://localhost:${getWiremockPort(wiremock)}`);
      stubEnv('HTTP_PROXY', `http://localhost:${getWiremockPort(uppercaseProxy)}`);

      await createLiveConnection({
        host: TARGET_HOST,
        port: TARGET_PORT,
        protocol: 'http',
      });

      const lowercaseHits = await getLoginRequests(wiremock);
      const uppercaseHits = await getLoginRequests(uppercaseProxy);
      if (isRunningNewDriverWithBD('BD#112')) {
        expect(lowercaseHits).toHaveLength(0);
        expect(uppercaseHits).toHaveLength(1);
      } else {
        expect(lowercaseHits).toHaveLength(1);
        expect(uppercaseHits).toHaveLength(0);
      }
    });

    it('should not treat a bare NO_PROXY domain as including subdomains', async () => {
      await wiremock.stub(loginSuccess());
      stubEnv('NO_PROXY', 'nonexistent.invalid');

      const proxied = await isLoginProxied(SUBDOMAIN_HOST);
      expect(proxied).toBe(!isRunningNewDriverWithBD('BD#113'));
    });

    it.each([
      '*.nonexistent.invalid',
      'account.*.invalid',
      'acc*.*.invalid',
      'localhost,account.*.invalid',
      'localhost|account.*.invalid',
    ])('should bypass the proxy for NO_PROXY pattern %s', async (noProxy) => {
      await wiremock.stub(loginSuccess());
      stubEnv('NO_PROXY', noProxy);

      const proxied = await isLoginProxied(SUBDOMAIN_HOST);
      expect(proxied).toBe(isRunningNewDriverWithBD('BD#113'));
    });

    it('should use no_proxy when NO_PROXY is also set', async () => {
      await wiremock.stub(loginSuccess());
      stubEnv('no_proxy', 'localhost');
      stubEnv('NO_PROXY', '*');

      const proxied = await isLoginProxied(TARGET_HOST);
      expect(proxied).toBe(!isRunningNewDriverWithBD('BD#112'));
    });
  });
});
