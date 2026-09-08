"""Unit tests for auth-browser **diagnostics** (no live credentials, no job gating)."""

from __future__ import annotations

import unittest

from auth_browser_param_diagnostics import KEYS, describe, format_diagnostics_report


class AuthBrowserParamDiagnosticsTests(unittest.TestCase):
    def test_should_label_strings_present_or_empty(self) -> None:
        self.assertEqual(describe("okta-user"), "present")
        self.assertEqual(describe(""), "empty")
        self.assertEqual(describe(["a"]), "present")
        self.assertEqual(describe([]), "empty")
        self.assertEqual(describe(1), "int")

    def test_should_report_present_empty_and_absent_without_values(self) -> None:
        report = format_diagnostics_report(
            {
                "testconnection": {
                    "SNOWFLAKE_TEST_HOST": "example.snowflakecomputing.com",
                    "SNOWFLAKE_TEST_OKTA_USER": "secret-user",
                    "SNOWFLAKE_TEST_OKTA_PASSWORD": "",
                },
                "testconnection-nodejs": {
                    "SNOWFLAKE_TEST_OKTA_USER": "other-secret",
                },
            },
            keys=(
                "SNOWFLAKE_TEST_HOST",
                "SNOWFLAKE_TEST_OKTA_USER",
                "SNOWFLAKE_TEST_OKTA_PASSWORD",
                "MISSING",
            ),
        )
        self.assertIn("testconnection: 3 keys", report)
        self.assertIn("testconnection-nodejs: 1 keys", report)
        self.assertIn("SNOWFLAKE_TEST_HOST: testconnection=present", report)
        self.assertIn(
            "SNOWFLAKE_TEST_OKTA_USER: testconnection=present, testconnection-nodejs=present",
            report,
        )
        self.assertIn("SNOWFLAKE_TEST_OKTA_PASSWORD: testconnection=empty", report)
        self.assertIn("MISSING: absent", report)
        self.assertNotIn("example.snowflakecomputing.com", report)
        self.assertNotIn("secret-user", report)
        self.assertNotIn("other-secret", report)

    def test_should_keep_auth_browser_key_list_non_empty(self) -> None:
        self.assertGreater(len(KEYS), 0)
        self.assertIn("SNOWFLAKE_TEST_OKTA_USER", KEYS)


if __name__ == "__main__":
    unittest.main()
