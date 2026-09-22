const connParameters = require('./connectionParameters');
const AuthTest = require('./authTestsBaseClass.js');

describe('External browser authentication tests', function () {
  const provideBrowserCredentialsPath = '/externalbrowser/provideBrowserCredentials.js';
  const login = connParameters.snowflakeTestBrowserUser;
  const password = connParameters.snowflakeAuthTestOktaPass;
  let authTest;

  beforeEach(async () => {
    authTest = new AuthTest();
    await authTest.cleanBrowserProcesses();
  });

  afterEach(async () => {
    await authTest.destroyConnection();
  });

  describe('External browser tests', async () => {
    // The "Successful connection" and "ID Token authentication tests" describe
    // blocks that used to live here have been migrated to
    // nodejs/tests/e2e/authentication/external-browser.test.ts (Okta happy path
    // and cached-ID-token reuse) and
    // sf_core/tests/integration/authentication/external_browser_id_token_cache.rs
    // (store / reuse / evict / retry-with-browser).
    //
    // The remaining live-IdP cases have no UD Gherkin counterpart. The timeout
    // cases use the legacy browserActionTimeout option and do not replace the
    // mocked no-callback integration scenario.

    it('Mismatched Username', async () => {
      const connectionOption = {
        ...connParameters.externalBrowser,
        username: 'differentUsername',
        clientStoreTemporaryCredential: false,
      };
      authTest.createConnection(connectionOption);
      const provideCredentialsPromise = authTest.execWithTimeout(
        'node',
        [provideBrowserCredentialsPath, 'success', login, password],
        15000,
      );
      await authTest.connectAndProvideCredentials(provideCredentialsPromise);
      authTest.verifyErrorWasThrown(
        'The user you were trying to authenticate as differs from the user currently logged in at the IDP.',
      );
      await authTest.verifyConnectionIsNotUp(
        'Unable to perform operation using terminated connection.',
      );
    });

    it('Wrong credentials', async () => {
      const login = 'itsnotanaccount.com';
      const password = 'fakepassword';
      const connectionOption = {
        ...connParameters.externalBrowser,
        browserActionTimeout: 10000,
        clientStoreTemporaryCredential: false,
      };
      authTest.createConnection(connectionOption);
      const provideCredentialsPromise = authTest.execWithTimeout('node', [
        provideBrowserCredentialsPath,
        'fail',
        login,
        password,
      ]);
      await authTest.connectAndProvideCredentials(provideCredentialsPromise);
      authTest.verifyErrorWasThrown(
        'Error while getting SAML token: Browser action timed out after 10000 ms.',
      );
      await authTest.verifyConnectionIsNotUp();
    });

    it('External browser timeout', async () => {
      const connectionOption = {
        ...connParameters.externalBrowser,
        browserActionTimeout: 100,
        clientStoreTemporaryCredential: false,
      };
      authTest.createConnection(connectionOption);
      const connectToBrowserPromise = authTest.execWithTimeout('node', [
        provideBrowserCredentialsPath,
        'timeout',
      ]);
      await authTest.connectAndProvideCredentials(connectToBrowserPromise);
      authTest.verifyErrorWasThrown(
        'Error while getting SAML token: Browser action timed out after 100 ms.',
      );
      await authTest.verifyConnectionIsNotUp();
    });
  });
});
