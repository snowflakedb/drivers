#!/usr/bin/env python3
"""
Feature File Parser Module

Handles parsing of Gherkin feature files to extract scenarios,
annotations, and test method mappings.
"""

import re
from pathlib import Path
from typing import Dict, List, Optional

# Pre-compile regex patterns for better performance
TEST_CASE_PATTERN = re.compile(r'TEST_CASE\s*\(\s*"([^"]+)"')
CPP_METHOD_PATTERN = re.compile(r'void\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(')
RUST_FUNCTION_PATTERN = re.compile(r'fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(')
PYTHON_TEST_PATTERN = re.compile(r'def\s+(test_[a-zA-Z_][a-zA-Z0-9_]*)\s*\(')


class FeatureParser:
    """Parses Gherkin feature files and extracts test information."""
    
    def __init__(self, workspace_root: Path):
        self.workspace_root = workspace_root
        self._file_cache = {}  # Cache for file contents to avoid re-reading
    
    def clear_cache(self):
        """Clear the file cache to free memory."""
        self._file_cache.clear()
    
    def get_feature_scenarios(self, feature_path: str) -> List[str]:
        """Extract scenario names from a feature file."""
        try:
            full_path = self.workspace_root / feature_path
            # Use cached content if available
            if str(full_path) in self._file_cache:
                content = self._file_cache[str(full_path)]
            else:
                with open(full_path, 'r', encoding='utf-8') as f:
                    content = f.read()
                self._file_cache[str(full_path)] = content
            
            scenarios = []
            for line in content.split('\n'):
                line = line.strip()
                if line.startswith('Scenario:'):
                    scenario_name = line.replace('Scenario:', '').strip()
                    scenarios.append(scenario_name)
            
            return scenarios
        except Exception as e:
            print(f"Error reading feature file {feature_path}: {e}")
            return []
    
    def get_feature_scenarios_with_annotations(self, feature_path: str) -> List[Dict[str, any]]:
        """Extract scenario names and their annotations from a feature file."""
        scenarios = []
        feature_level_expected_drivers = []
        feature_level_tags = []
        
        try:
            full_path = self.workspace_root / feature_path
            # Use cached content if available
            if str(full_path) in self._file_cache:
                content = self._file_cache[str(full_path)]
            else:
                with open(full_path, 'r', encoding='utf-8') as f:
                    content = f.read()
                self._file_cache[str(full_path)] = content
            
            lines = content.split('\n')
            current_tags = []
            in_feature_header = True
            
            for i, line in enumerate(lines):
                line = line.strip()
                
                # Collect tags (annotations starting with @)
                if line.startswith('@'):
                    tags = [tag.strip() for tag in line.split() if tag.startswith('@')]
                    current_tags.extend(tags)
                    continue
                elif line.startswith('Feature:'):
                    # Store feature-level tags
                    feature_level_tags = current_tags.copy()
                    
                    # Process feature-level expected tags (for backward compatibility)
                    for tag in current_tags:
                        if '_expected' in tag.lower():
                            # Extract driver name from expected tag (e.g., @python_expected -> python)
                            driver_name = tag.replace('@', '').replace('_expected', '').replace('_EXPECTED', '')
                            feature_level_expected_drivers.append(driver_name.lower())
                    
                    # Reset tags for scenarios
                    current_tags = []
                    in_feature_header = False
                    continue
                
                # Process scenario
                if line.startswith('Scenario:'):
                    scenario_name = line.replace('Scenario:', '').strip()
                    
                    # Breaking Changes detection is now handled by the Rust validator
                    breaking_change_info = self._extract_breaking_change_info(current_tags)
                    
                    # Check for scenario-level expected tags
                    scenario_expected_drivers = []
                    for tag in current_tags:
                        if '_expected' in tag.lower():
                            # Extract driver name from expected tag (e.g., @odbc_expected -> odbc)
                            driver_name = tag.replace('@', '').replace('_expected', '').replace('_EXPECTED', '')
                            scenario_expected_drivers.append(driver_name.lower())
                    
                    scenario_info = {
                        'name': scenario_name,
                        'tags': current_tags.copy(),
                        'breaking_change_info': breaking_change_info,
                        'expected_drivers': scenario_expected_drivers,
                        'feature_level_expected_drivers': feature_level_expected_drivers,
                        'feature_level_tags': feature_level_tags
                    }
                    
                    scenarios.append(scenario_info)
                    current_tags = []  # Reset tags after processing scenario
                    continue
                
                # Reset tags if we hit a non-tag, non-scenario line
                if line and not line.startswith('#') and not line.startswith('Background:') and not in_feature_header:
                    # Reset tags if we encounter non-tag, non-scenario content
                    if not any(keyword in line for keyword in ['Given', 'When', 'Then', 'And', 'But']):
                        current_tags = []
                        
        except Exception as e:
            print(f"Error reading feature file {feature_path}: {e}")
        
        return scenarios
    
    def format_feature_name(self, feature_name: str, feature_path: str = None) -> str:
        """Extract feature name from the feature file's 'Feature:' line."""
        if feature_path:
            try:
                full_path = self.workspace_root / feature_path
                with open(full_path, 'r') as f:
                    for line in f:
                        if line.strip().startswith('Feature:'):
                            return line.replace('Feature:', '').strip()
            except Exception:
                pass
        
        # Fallback to formatted filename
        return feature_name.replace('_', ' ').title()
    
    def extract_test_methods_with_lines(self, file_path: str, scenario_names: List[str]) -> Dict[str, int]:
        """Extract test method names and their line numbers from a test file."""
        methods = {}
        
        try:
            # Use cached content if available to avoid re-reading files
            if file_path in self._file_cache:
                content = self._file_cache[file_path]
            else:
                with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                    content = f.read()
                self._file_cache[file_path] = content
            
            lines = content.split('\n')
            
            for i, line in enumerate(lines, 1):
                line_stripped = line.strip()
                
                # Look for TEST_CASE definitions (C++)
                test_case_match = TEST_CASE_PATTERN.search(line_stripped)
                if test_case_match:
                    test_name = test_case_match.group(1)
                    # Check if this test matches any of our scenarios
                    for scenario in scenario_names:
                        if self._method_matches_scenario(test_name, scenario):
                            methods[scenario] = i
                            break
                
                # Look for C++ method definitions
                method_match = CPP_METHOD_PATTERN.search(line_stripped)
                if method_match:
                    method_name = method_match.group(1)
                    for scenario in scenario_names:
                        if self._method_matches_scenario(method_name, scenario):
                            methods[scenario] = i
                            break
                
                # Look for Rust test functions (fn test_name() after #[test])
                rust_test_match = RUST_FUNCTION_PATTERN.search(line_stripped)
                if rust_test_match:
                    method_name = rust_test_match.group(1)
                    # Check if previous line has #[test] or #[rstest] attribute (look back a few lines)
                    is_test_function = False
                    for j in range(max(0, i-5), i):  # Look back up to 5 lines
                        if j < len(lines) and ('#[test]' in lines[j] or '#[rstest]' in lines[j]):
                            is_test_function = True
                            break
                    
                    if is_test_function:
                        for scenario in scenario_names:
                            if self._method_matches_scenario(method_name, scenario):
                                methods[scenario] = i
                                break
                
                # Look for Python test functions (def test_name():)
                python_test_match = PYTHON_TEST_PATTERN.search(line_stripped)
                if python_test_match:
                    method_name = python_test_match.group(1)
                    for scenario in scenario_names:
                        if self._method_matches_scenario(method_name, scenario):
                            methods[scenario] = i
                            break
        
        except Exception as e:
            print(f"Error reading test file {file_path}: {e}")
        
        return methods
    
    def _extract_breaking_change_info(self, tags: List[str]) -> Optional[Dict[str, str]]:
        """Breaking Changes information is now extracted from actual implementations via Rust validator.
        This method is kept for compatibility but returns None."""
        return None
    
    def _method_matches_scenario(self, method_name: str, scenario_name: str) -> bool:
        """Check if a test method name matches a scenario name using various naming conventions."""
        def normalize(s):
            return re.sub(r'[^a-zA-Z0-9]', '', s).lower()
        
        # Convert scenario to different naming conventions
        scenario_snake = re.sub(r'[^\w\s]', '', scenario_name).replace(' ', '_').lower()
        scenario_pascal = ''.join(word.capitalize() for word in re.sub(r'[^\w\s]', '', scenario_name).split())
        scenario_test_method = f"test_{scenario_snake}"
        
        method_normalized = normalize(method_name)
        
        return (method_normalized == normalize(scenario_name) or
                method_normalized == normalize(scenario_snake) or
                method_normalized == normalize(scenario_pascal) or
                method_normalized == normalize(scenario_test_method))
