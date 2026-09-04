const assert = require("assert");
const { chromium } = require("playwright");
const TotpGenerator = require("./totpGenerator.js");

const timeoutInMillis = 15000;

const connectToExternalBrowser = async (
  loginElementInput = 'input[type="submit"]',
  loginField = 'input[name="identifier"]',
) => {
  let page;
  const timeout = Date.now() + timeoutInMillis;
  while (Date.now() < timeout) {
    try {
      const browser = await chromium.connectOverCDP("http://localhost:9222");
      const defaultContext = await browser.contexts()[0];
      page = await defaultContext.pages()[0];
      await page.waitForSelector(loginElementInput, { timeout: 500 });
      await page.waitForSelector(loginField, { timeout: 500 });
      return page;
    } catch (err) {
      //Wait for browser to be ready
    }
  }
  if (!page) {
    await assert.fail("Cannot connect to browser");
  }
};

const fillExternalBrowserCredentialsInSnowflake = async (
  page,
  login,
  password,
  loginElementInput = 'input[name="identifier"]',
  credentialElementInput = 'input[name="credentials.passcode"]',
) => {
  const timeout = Date.now() + timeoutInMillis;

  while (Date.now() < timeout) {
    try {
      let providedLogin = await page.$eval(loginElementInput, (el) => el.value);
      let providedPassword = await page.$eval(
        credentialElementInput,
        (el) => el.value,
      );
      if (providedLogin !== login) {
        await page.fill(loginElementInput, login);
        providedLogin = await page.$eval(loginElementInput, (el) => el.value);
      }
      if (providedPassword !== password) {
        await page.fill(credentialElementInput, password);
        providedPassword = await page.$eval(
          credentialElementInput,
          (el) => el.value,
        );
      }
      if (providedLogin === login && providedPassword === password) {
        return true;
      }
    } catch (err) {
      //Wait for browser to be ready
    }
  }
  return false;
};

const fillExternalBrowserCredentialsInOkta = async (
  page,
  login,
  password,
  loginElementInput = 'input[name="identifier"]',
  credentialElementInput = 'input[name="credentials.passcode"]',
) => {
  const timeout = Date.now() + timeoutInMillis;

  while (Date.now() < timeout) {
    try {
      // Step 1: Fill login field
      let providedLogin = await page.$eval(loginElementInput, (el) => el.value);
      if (providedLogin !== login) {
        await page.fill(loginElementInput, login);
        providedLogin = await page.$eval(loginElementInput, (el) => el.value);
      }

      // Step 2: Click next button if login is filled correctly
      if (providedLogin === login) {
        await page.click('input[value="Next"]');

        // Step 3: Wait for password field to be available and fill it
        await page.waitForSelector(credentialElementInput, { timeout: 5000 });

        let providedPassword = await page.$eval(
          credentialElementInput,
          (el) => el.value,
        );
        if (providedPassword !== password) {
          await page.fill(credentialElementInput, password);
          providedPassword = await page.$eval(
            credentialElementInput,
            (el) => el.value,
          );
        }

        // Step 4: Check if both login and password are correctly filled
        if (providedLogin === login && providedPassword === password) {
          return true;
        }
      }
    } catch (err) {
      //Wait for browser to be ready or for elements to appear
    }
  }
  return false;
};

const submitAndCheckOktaErrorMessage = async (page) => {
  let errorMessage = "";
  await page.click('input[type="submit"]');
  try {
    await page.waitForSelector(
      ".okta-form-infobox-error.infobox.infobox-error",
      { timeout: 5000 },
    );
    errorMessage = await page.$eval(
      ".okta-form-infobox-error.infobox.infobox-error p",
      (el) => el.textContent,
    );
  } catch (err) {
    //No 'login error message'
  }
  if (errorMessage) {
    return errorMessage;
  } else {
    return "";
  }
};

const submitAndCheckSnowflakeErrorMessage = async (page) => {
  let errorMessage = "";
  await page.click('button:has-text("Sign in")');

  try {
    await page.waitForSelector('div[role="status"]', { timeout: 1000 });
    errorMessage = await page.$eval(
      'div[role="status"]',
      (el) => el.textContent,
    );
  } catch (err) {
    //No 'login error message'
  }
  if (errorMessage) {
    return errorMessage;
  } else {
    return "";
  }
};

const getExternalBrowserOktaResponse = async (page) => {
  return await page.waitForSelector("text=Your identity was confirmed", {
    timeout: 10000,
  });
};

// Fill Snowflake's authenticator-app MFA verification step, if presented. No-op if the
// field never appears, so this stays safe to call for accounts that don't require MFA.
const fillMfaCodeIfPresented = async (page, totpSeed) => {
  if (!totpSeed) {
    return;
  }
  const mfaCodeInput = 'input[autocomplete="one-time-code"]';
  try {
    await page.waitForSelector(mfaCodeInput, { timeout: 5000 });
  } catch (err) {
    return;
  }
  const totp = new TotpGenerator(totpSeed);
  // Jenkins already serializes Python/ODBC/JDBC under
  // drivers-auth-browser-shared-mfa-accounts (#1329). Do not re-enable
  // parallel stages on this shared seed without per-language users
  // (SNOW-4017658). Retry with a fresh window if this attempt is rejected.
  const maxAttempts = 2;
  const totpStepMs = 30_000;
  const detachTimeoutMs = 8000;
  for (let attempt = 0; attempt < maxAttempts; attempt++) {
    // generateTotp waits out a near-boundary window (MIN_VALIDITY_SECONDS)
    // so the code stays valid through fill + click + redirect latency.
    const current = await totp.generateTotp();
    const submittedWindow = Math.floor(Date.now() / totpStepMs);
    await page.fill(mfaCodeInput, current);
    await page.click('button:has-text("Continue")');
    try {
      // Success navigates away, detaching this input. A rejected code leaves it
      // attached (e.g. still showing an "invalid code" message) - retry in that case.
      await page.waitForSelector(mfaCodeInput, {
        state: "detached",
        timeout: detachTimeoutMs,
      });
      return;
    } catch (err) {
      // Only wait out this window if we are still in it — a detach poll that
      // already crossed the boundary must not burn the next unused window.
      if (Math.floor(Date.now() / totpStepMs) === submittedWindow) {
        await totp.sleep(totp.getTimeRemainingMs() + 1000);
      }
    }
  }
  throw new Error(`MFA verification failed after ${maxAttempts} attempts`);
};

// Click through Snowflake's one-time OAuth consent screen ("<client> would like
// access to your Snowflake Account... Allow / Cancel"), if presented. Snowflake
// shows this the first time a given user authenticates through a given OAuth
// client, then never again for that (user, client) pair — the long-lived shared
// test user never sees it because it's been through this client on every prior
// CI run. Any new MFA pool user will hit it exactly once.
//
// Uses a non-throwing existence check (locator.count()), not the try/catch
// waitForSelector idiom used elsewhere in this file: that pattern throws a real
// TimeoutError internally on every run where the screen doesn't appear (i.e.
// nearly all of them, after a user's first use) — caught and swallowed here,
// but still a real exception that would show up in any Playwright trace/debug
// log a future dev inspects for an unrelated issue, misleadingly suggesting a
// failure. count() never throws; it just polls.
const clickOauthConsentIfPresented = async (page) => {
  const allowButton = page.locator('button:has-text("Allow")');
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline && (await allowButton.count()) === 0) {
    await page.waitForTimeout(200);
  }
  if ((await allowButton.count()) > 0) {
    await allowButton.click();
  }
};

const checkOauthResponse = async (page) => {
  await clickOauthConsentIfPresented(page);

  const jdbcConfirmationMessage = "text=Authorization completed successfully";
  const odbcConfirmationMessage =
    "text=Access to Snowflake has been granted to the ODBC driver.";
  const golangConfirmationMessage =
    "text=OAuth authentication completed successfully.";
  const pythonConfirmationMessage =
    "text=Your identity was confirmed and propagated to Snowflake PythonConnector.";
  const dotnetConfirmationMessage =
    "text=Your identity was confirmed and propagated to Snowflake .NET driver.";
  const nodejsConfirmationMessage =
    "text=Your identity was confirmed and propagated to Snowflake Node.js driver";
  try {
    await Promise.race([
      page.waitForSelector(jdbcConfirmationMessage, { timeout: 15000 }),
      page.waitForSelector(pythonConfirmationMessage, { timeout: 15000 }),
      page.waitForSelector(odbcConfirmationMessage, { timeout: 15000 }),
      page.waitForSelector(golangConfirmationMessage, { timeout: 15000 }),
      page.waitForSelector(dotnetConfirmationMessage, { timeout: 15000 }),
      page.waitForSelector(nodejsConfirmationMessage, { timeout: 15000 }),
    ]);
  } catch (err) {
    throw new Error("Providing credentials was not successful");
  }
};

const closeBrowser = async (page) => {
  await page.close();
};

const successfulExternalBrowserOktaConnection = async (login, password) => {
  const page = await connectToExternalBrowser();
  await assert.notEqual(page, null, "Browser connection timed out");

  const credentialsProvided = await fillExternalBrowserCredentialsInOkta(
    page,
    login,
    password,
  );
  await assert.equal(credentialsProvided, true, "Cannot provided credentials");

  const errMsg = await submitAndCheckOktaErrorMessage(page);
  await assert.equal(errMsg, "", errMsg);

  const response = await getExternalBrowserOktaResponse(page);
  await assert.ok(response, "Your identity was confirmed");
  await closeBrowser(page);
};

const failedExternalBrowserOktaConnection = async (login, password) => {
  const page = await connectToExternalBrowser();
  await assert.notEqual(page, null, "Browser connection timed out");

  const credentialsProvided = await fillExternalBrowserCredentialsInOkta(
    page,
    login,
    password,
  );
  await assert.equal(credentialsProvided, true, "Cannot provided credentials");

  const errMsg = await submitAndCheckOktaErrorMessage(page);
  await assert.equal(errMsg, "Unable to sign in", errMsg);

  await closeBrowser(page);
};

const successfulExternalOauthOktaConnection = async (login, password) => {
  const page = await connectToExternalBrowser();
  await assert.notEqual(page, null, "Browser connection timed out");

  const credentialsProvided = await fillExternalBrowserCredentialsInOkta(
    page,
    login,
    password,
  );
  await assert.equal(credentialsProvided, true, "Cannot provided credentials");

  const errMsg = await submitAndCheckOktaErrorMessage(page);
  await assert.equal(errMsg, "", errMsg);
  await checkOauthResponse(page);

  await closeBrowser(page);
};

const successfulInternalOauthSnowflakeConnection = async (
  login,
  password,
  totpSeed,
) => {
  const loginElementInput = 'input[autocomplete="username"]';
  const credentialsElementInput = 'input[autocomplete="current-password"]';

  const page = await connectToExternalBrowser(
    loginElementInput,
    credentialsElementInput,
  );
  await assert.notEqual(page, null, "Browser connection timed out");

  const credentialsProvided = await fillExternalBrowserCredentialsInSnowflake(
    page,
    login,
    password,
    loginElementInput,
    credentialsElementInput,
  );
  await assert.equal(credentialsProvided, true, "Cannot provided credentials");

  const errMsg = await submitAndCheckSnowflakeErrorMessage(page);
  await assert.equal(errMsg, "", errMsg);
  await fillMfaCodeIfPresented(page, totpSeed);
  await checkOauthResponse(page);

  await closeBrowser(page);
};

const args = process.argv.slice(2);
async function main() {
  if (args[0] === "success") {
    await successfulExternalBrowserOktaConnection(args[1], args[2]);
  } else if (args[0] === "fail") {
    await failedExternalBrowserOktaConnection(args[1], args[2]);
  } else if (args[0] === "timeout") {
    await connectToExternalBrowser();
  } else if (args[0] === "externalOauthOktaSuccess") {
    await successfulExternalOauthOktaConnection(args[1], args[2]);
  } else if (args[0] === "internalOauthSnowflakeSuccess") {
    await successfulInternalOauthSnowflakeConnection(args[1], args[2], args[3]);
  }
  process.exit(0);
}

main();
