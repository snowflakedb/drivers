#!/usr/bin/env python3

import unittest
from pathlib import Path
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parent))

from coverage_report import CoverageReportGenerator
from feature_parser import FeatureParser
from html_generator import HTMLGenerator


class NodejsLanguageMappingTest(unittest.TestCase):
    def setUp(self):
        self.features = {
            "authentication/external_browser": {
                "languages": {
                    "Rust": {},
                    "JavaScript": {},
                }
            }
        }

    def test_coverage_report_maps_javascript_to_nodejs(self):
        report = CoverageReportGenerator(".")

        self.assertEqual(report.get_all_languages(self.features), ["core", "nodejs"])
        self.assertEqual(report.get_language_data_key("nodejs"), "JavaScript")

    def test_html_generator_maps_javascript_to_nodejs(self):
        report = HTMLGenerator(".")

        self.assertEqual(report._get_all_languages(self.features), ["core", "nodejs"])
        self.assertEqual(report._get_language_data_key("nodejs"), "JavaScript")

    def test_feature_parser_extracts_vitest_methods(self):
        scenarios = [
            "active scenario",
            "todo scenario",
            "conditional scenario",
        ]
        with tempfile.NamedTemporaryFile(mode="w", suffix=".test.ts") as test_file:
            test_file.write(
                "it('active scenario', () => {});\n"
                "it.todo('todo scenario');\n"
                "it.skipIf(false)('conditional scenario', () => {});\n"
            )
            test_file.flush()

            methods = FeatureParser(Path(".")).extract_test_methods_with_lines(
                test_file.name,
                scenarios,
            )

        self.assertEqual(
            methods,
            {
                "active scenario": 1,
                "todo scenario": 2,
                "conditional scenario": 3,
            },
        )

    def test_coverage_report_resolves_nodejs_integration_path(self):
        report = CoverageReportGenerator(".")

        self.assertEqual(
            report._get_integration_test_file_path(
                "nodejs",
                "tests/definitions/shared/authentication/external_browser.feature",
            ),
            report.workspace_root
            / "nodejs/tests/integ/authentication/external-browser.test.ts",
        )


if __name__ == "__main__":
    unittest.main()
