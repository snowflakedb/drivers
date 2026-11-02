const connParameters = require('./connectionParameters');
const AuthTest = require('./authTestsBaseClass.js');

// TODO(SNOW-4397810): `Successful connection` stays in this file.
// `should authenticate using authorization code with an Okta IdP` is skipped
// until parameters_preprod has the Okta redirect URI and authorize URL.
describe('Oauth Okta Authorization code tests', function () {
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

  it('Successful connection', async () => {
    const connectionOption = {
      ...connParameters.oauthOktaAuthorizationCode,
      clientStoreTemporaryCredential: false,
    };
    authTest.createConnection(connectionOption);
    const provideCredentialsPromise = authTest.execWithTimeout(
      'node',
      [provideBrowserCredentialsPath, 'externalOauthOktaSuccess', login, password],
      15000,
    );
    await authTest.connectAndProvideCredentials(provideCredentialsPromise);
    authTest.verifyNoErrorWasThrown();
    await authTest.verifyConnectionIsUp();
  });
});
