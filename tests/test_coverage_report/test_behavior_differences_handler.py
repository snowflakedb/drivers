#!/usr/bin/env python3
"""Unit tests for BehaviorDifferencesHandler lookup caching."""

import unittest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))

from feature_parser import FeatureParser
from behavior_differences_handler import BehaviorDifferencesHandler


def _impl(test_method: str, test_file: str = 'tests/foo.cpp', test_line: int = 10) -> dict:
    return {
        'test_method': test_method,
        'test_file': test_file,
        'test_line': test_line,
    }


class _FakeValidator:
    def __init__(self, by_language: dict):
        self.by_language = by_language
        self.data_calls = 0

    def get_behavior_difference_data(self) -> dict:
        self.data_calls += 1
        return {'behavior_differences_by_language': self.by_language}


class BehaviorDifferencesHandlerCacheTest(unittest.TestCase):
    def setUp(self):
        self.validator = _FakeValidator({
            'odbc': [
                {
                    'behavior_difference_id': 'BD#1',
                    'description': 'one',
                    'implementations': [_impl('test_should_bind_parameters')],
                },
                {
                    'behavior_difference_id': 'BD#2',
                    'description': 'two',
                    'implementations': [_impl('ShouldFetchNulls')],
                },
            ],
            'python': [
                {
                    'behavior_difference_id': 'BD#9',
                    'description': 'py',
                    'implementations': [_impl('test_should_close_cursor')],
                },
            ],
        })
        self.handler = BehaviorDifferencesHandler(self.validator, FeatureParser(Path('.')))

    def test_should_match_snake_and_pascal_method_names(self):
        self.assertEqual(
            self.handler.get_behavior_difference_ids_for_scenario(
                {'name': 'should bind parameters'}, 'odbc'
            ),
            ['BD#1'],
        )
        self.assertEqual(
            self.handler.get_behavior_difference_ids_for_scenario(
                {'name': 'should fetch nulls'}, 'ODBC'
            ),
            ['BD#2'],
        )

    def test_should_reuse_converted_mappings_across_lookups(self):
        self.handler.get_behavior_difference_ids_for_scenario(
            {'name': 'should bind parameters'}, 'odbc'
        )
        self.handler.get_behavior_difference_ids_for_scenario(
            {'name': 'should bind parameters'}, 'odbc'
        )
        self.handler.get_behavior_difference_ids_for_scenario(
            {'name': 'should fetch nulls'}, 'odbc'
        )
        self.handler.get_behavior_difference_test_mappings('odbc')
        self.assertEqual(self.validator.data_calls, 1)

    def test_should_isolate_indexes_per_driver(self):
        odbc_ids = self.handler.get_behavior_difference_ids_for_scenario(
            {'name': 'should close cursor'}, 'odbc'
        )
        python_ids = self.handler.get_behavior_difference_ids_for_scenario(
            {'name': 'should close cursor'}, 'python'
        )
        self.assertEqual(odbc_ids, [])
        self.assertEqual(python_ids, ['BD#9'])

    def test_should_return_empty_list_when_scenario_has_no_bd(self):
        self.assertEqual(
            self.handler.get_behavior_difference_ids_for_scenario(
                {'name': 'should do something unrelated'}, 'odbc'
            ),
            [],
        )


if __name__ == '__main__':
    unittest.main()
