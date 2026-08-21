#!/usr/bin/env python3
"""
Behavior Difference Data Handler Module

Handles Behavior Difference data processing and integration
with the Rust validator output.
"""

from collections import defaultdict
from typing import Dict, List, Set
try:
    from .validator_integration import ValidatorIntegration
    from .feature_parser import FeatureParser
except ImportError:
    from validator_integration import ValidatorIntegration
    from feature_parser import FeatureParser


class BehaviorDifferencesHandler:
    """Handles Behavior Difference data processing and scenario matching."""
    
    def __init__(self, validator: ValidatorIntegration, feature_parser: FeatureParser):
        self.validator = validator
        self.feature_parser = feature_parser
        # Converted validator mappings, keyed by lowercase driver name.
        self._mappings_by_driver: Dict[str, Dict] = {}
        # normalized test_method -> BD ids, keyed by lowercase driver name.
        self._method_index_by_driver: Dict[str, Dict[str, Set[str]]] = {}
        # (driver, scenario_name) -> sorted BD ids
        self._ids_for_scenario: Dict[tuple, List[str]] = {}
    
    def get_behavior_difference_descriptions(self, driver: str = 'odbc') -> Dict[str, str]:
        """Get Behavior Difference descriptions from the validator for a specific driver."""
        behavior_difference_data = self.validator.get_behavior_difference_data()
        if not behavior_difference_data:
            print(f"Warning: Could not get Behavior Difference descriptions for {driver} from Rust validator")
            return {}
        
        # Extract descriptions from the specific language's Behavior Differences
        behavior_differences_by_language = behavior_difference_data.get('behavior_differences_by_language', {})
        descriptions = {}
        
        for lang, behavior_difference_list in behavior_differences_by_language.items():
            if lang.lower() == driver.lower():
                for behavior_difference_info in behavior_difference_list:
                    behavior_difference_id = behavior_difference_info['behavior_difference_id']
                    description = behavior_difference_info.get('description', '')
                    if description:
                        descriptions[behavior_difference_id] = description
                break
        
        return descriptions
    
    def get_behavior_difference_test_mappings(self, driver: str = 'odbc', features: Dict = None) -> Dict[str, Dict[str, List[Dict[str, any]]]]:
        """Get Behavior Difference test mappings from the validator in Python format."""
        cache_key = driver.lower()
        cached = self._mappings_by_driver.get(cache_key)
        if cached is not None:
            return cached

        behavior_difference_data = self.validator.get_behavior_difference_data()
        if not behavior_difference_data:
            print(f"Warning: Could not get Behavior Difference test mappings for {driver} from Rust validator")
            self._mappings_by_driver[cache_key] = {}
            return {}
        
        behavior_differences_by_language = behavior_difference_data.get('behavior_differences_by_language', {})
        
        # Convert Rust format to Python format
        result = {}
        for lang, behavior_difference_list in behavior_differences_by_language.items():
            if lang.lower() == cache_key:
                result[lang] = {}
                for behavior_difference_info in behavior_difference_list:
                    behavior_difference_id = behavior_difference_info['behavior_difference_id']
                    result[lang][behavior_difference_id] = []
                    
                    for impl in behavior_difference_info['implementations']:
                        # Convert Rust implementation format to Python format
                        converted_impl = {
                            'test_method': impl['test_method'],
                            'test_file': impl['test_file'],
                            'test_line': impl['test_line'],
                            'line_number': impl.get('new_behaviour_line') or impl.get('old_behaviour_line', impl['test_line']),
                            'assertion_file': impl.get('new_behaviour_file') or impl.get('old_behaviour_file', impl['test_file']),
                            'assertion_line': impl.get('new_behaviour_line') or impl.get('old_behaviour_line', impl['test_line'])
                        }
                        
                        if impl.get('new_behaviour_file'):
                            converted_impl['new_behaviour_file'] = impl['new_behaviour_file']
                            converted_impl['new_behaviour_line'] = impl['new_behaviour_line']
                        
                        if impl.get('old_behaviour_file'):
                            converted_impl['old_behaviour_file'] = impl['old_behaviour_file']
                            converted_impl['old_behaviour_line'] = impl['old_behaviour_line']
                        
                        if impl.get('old_driver_skipped'):
                            converted_impl['old_driver_skipped'] = True
                        if impl.get('new_driver_skipped'):
                            converted_impl['new_driver_skipped'] = True
                        
                        result[lang][behavior_difference_id].append(converted_impl)
        
        self._mappings_by_driver[cache_key] = result
        return result

    def _method_index_for_driver(self, driver: str) -> Dict[str, Set[str]]:
        """Build normalized test_method -> BD id sets once per driver."""
        cache_key = driver.lower()
        cached = self._method_index_by_driver.get(cache_key)
        if cached is not None:
            return cached

        index: Dict[str, Set[str]] = defaultdict(set)
        mapping = self.get_behavior_difference_test_mappings(driver)
        for lang, behavior_differences in mapping.items():
            if lang.lower() != cache_key:
                continue
            for behavior_difference_id, implementations in behavior_differences.items():
                for impl in implementations:
                    index[FeatureParser.normalize_identifier(impl['test_method'])].add(
                        behavior_difference_id
                    )

        self._method_index_by_driver[cache_key] = index
        return index
    
    def get_behavior_difference_ids_for_scenario(self, scenario_info: Dict, driver: str, features: Dict = None) -> List[str]:
        """Get all Behavior Difference IDs associated with a scenario by looking up test implementations."""
        scenario_name = scenario_info['name']
        cache_key = (driver.lower(), scenario_name)
        cached = self._ids_for_scenario.get(cache_key)
        if cached is not None:
            return cached

        matching_ids: Set[str] = set()
        method_index = self._method_index_for_driver(driver)
        for match_key in FeatureParser.scenario_match_keys(scenario_name):
            matching_ids.update(method_index.get(match_key, ()))

        result = sorted(matching_ids)
        self._ids_for_scenario[cache_key] = result
        return result
