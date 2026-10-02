/* oxlint-disable unicorn/no-empty-file */
// The "surfaces Snowflake error message in the connect callback" case that used
// to live here asserts Error code 390511 / SSO URL generation failed in External
// browser's SAML Request flow. It is replaced by
// nodejs/tests/e2e/authentication/external-browser.test.ts
// "should fail when authenticator-request reports SSO URL generation failure"
// (WireMock mapping auth/external_browser_authenticator_request_sso_url_error.json).
// Gherkin scenarios 8–9 remain @core_int URL validation and are not this observable.
