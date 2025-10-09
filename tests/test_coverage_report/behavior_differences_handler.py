#!/usr/bin/env python3
"""
Behavior Difference Data Handler Module

Handles Behavior Difference data processing and integration
with the Rust validator output.
"""

from typing import Dict, List
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
        behavior_difference_data = self.validator.get_behavior_difference_data()
        if not behavior_difference_data:
            print(f"Warning: Could not get Behavior Difference test mappings for {driver} from Rust validator")
            return {}
        
        behavior_differences_by_language = behavior_difference_data.get('behavior_differences_by_language', {})
        
        # Convert Rust format to Python format
        result = {}
        for lang, behavior_difference_list in behavior_differences_by_language.items():
            if lang.lower() == driver.lower():
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
                        
                        result[lang][behavior_difference_id].append(converted_impl)
        
        return result
    
    def get_behavior_difference_ids_for_scenario(self, scenario_info: Dict, driver: str, features: Dict = None) -> List[str]:
        """Get all Behavior Difference IDs associated with a scenario by looking up test implementations."""
        scenario_name = scenario_info['name']
        
        # Look up the actual Behavior Difference IDs from test files for this scenario
        behavior_difference_test_mapping = self.get_behavior_difference_test_mappings(driver, features)
        
        # Find all Behavior Difference IDs associated with test methods that match this scenario
        matching_behavior_difference_ids = []
        if driver in behavior_difference_test_mapping:
            for behavior_difference_id, implementations in behavior_difference_test_mapping[driver].items():
                for impl in implementations:
                    if self.feature_parser._method_matches_scenario(impl['test_method'], scenario_name):
                        if behavior_difference_id not in matching_behavior_difference_ids:
                            matching_behavior_difference_ids.append(behavior_difference_id)
        return sorted(matching_behavior_difference_ids)
