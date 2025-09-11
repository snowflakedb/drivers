#!/usr/bin/env python3
"""
Breaking Change Data Handler Module

Handles Business Change Request (Breaking Change) data processing and integration
with the Rust validator output.
"""

from typing import Dict, List
try:
    from .validator_integration import ValidatorIntegration
    from .feature_parser import FeatureParser
except ImportError:
    from validator_integration import ValidatorIntegration
    from feature_parser import FeatureParser


class BreakingChangesHandler:
    """Handles Breaking Change data processing and scenario matching."""
    
    def __init__(self, validator: ValidatorIntegration, feature_parser: FeatureParser):
        self.validator = validator
        self.feature_parser = feature_parser
    
    def get_breaking_change_descriptions(self, driver: str = 'odbc') -> Dict[str, str]:
        """Get Breaking Change descriptions from the validator for a specific driver."""
        breaking_change_data = self.validator.get_breaking_change_data()
        if not breaking_change_data:
            print(f"Warning: Could not get Breaking Change descriptions for {driver} from Rust validator")
            return {}
        
        # Extract descriptions from the specific language's Breaking Changes
        breaking_changes_by_language = breaking_change_data.get('breaking_changes_by_language', {})
        descriptions = {}
        
        for lang, breaking_change_list in breaking_changes_by_language.items():
            if lang.lower() == driver.lower():
                for breaking_change_info in breaking_change_list:
                    breaking_change_id = breaking_change_info['breaking_change_id']
                    description = breaking_change_info.get('description', '')
                    if description:
                        descriptions[breaking_change_id] = description
                break
        
        return descriptions
    
    def get_breaking_change_test_mappings(self, driver: str = 'odbc', features: Dict = None) -> Dict[str, Dict[str, List[Dict[str, any]]]]:
        """Get Breaking Change test mappings from the validator in Python format."""
        breaking_change_data = self.validator.get_breaking_change_data()
        if not breaking_change_data:
            print(f"Warning: Could not get Breaking Change test mappings for {driver} from Rust validator")
            return {}
        
        breaking_changes_by_language = breaking_change_data.get('breaking_changes_by_language', {})
        
        # Convert Rust format to Python format
        result = {}
        for lang, breaking_change_list in breaking_changes_by_language.items():
            if lang.lower() == driver.lower():
                result[lang] = {}
                for breaking_change_info in breaking_change_list:
                    breaking_change_id = breaking_change_info['breaking_change_id']
                    result[lang][breaking_change_id] = []
                    
                    for impl in breaking_change_info['implementations']:
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
                        
                        result[lang][breaking_change_id].append(converted_impl)
        
        return result
    
    def get_breaking_change_ids_for_scenario(self, scenario_info: Dict, driver: str, features: Dict = None) -> List[str]:
        """Get all Breaking Change IDs associated with a scenario by looking up test implementations."""
        scenario_name = scenario_info['name']
        
        # Look up the actual Breaking Change IDs from test files for this scenario
        breaking_change_test_mapping = self.get_breaking_change_test_mappings(driver, features)
        
        # Find all Breaking Change IDs associated with test methods that match this scenario
        matching_breaking_change_ids = []
        if driver in breaking_change_test_mapping:
            for breaking_change_id, implementations in breaking_change_test_mapping[driver].items():
                for impl in implementations:
                    if self.feature_parser._method_matches_scenario(impl['test_method'], scenario_name):
                        if breaking_change_id not in matching_breaking_change_ids:
                            matching_breaking_change_ids.append(breaking_change_id)
        return sorted(matching_breaking_change_ids)
