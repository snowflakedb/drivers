#!/usr/bin/env python3
"""
Universal Driver Test Coverage Report Generator

Generates coverage reports showing which scenarios and tests are implemented
across different languages based on e2e feature files.
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Set, Optional, Tuple
import argparse

# Import the new modular components
try:
    from .validator_integration import ValidatorIntegration
    from .feature_parser import FeatureParser
    from .breaking_changes_handler import BreakingChangesHandler
    from .html_generator import HTMLGenerator
except ImportError:
    # Handle relative imports when running as script
    sys.path.append(os.path.dirname(__file__))
    from validator_integration import ValidatorIntegration
    from feature_parser import FeatureParser
    from breaking_changes_handler import BreakingChangesHandler
    from html_generator import HTMLGenerator



class CoverageReportGenerator:
    """Generates test coverage reports for multi-language test suites."""
    
    def __init__(self, workspace_root: str):
        self.workspace_root = Path(workspace_root).resolve()  # Convert to absolute path
        self.validator_path = self.workspace_root / "tests" / "tests_format_validator"
        self._validator_breaking_change_cache = None  # Cache for validator JSON output
        self._css_cache = None  # Cache for CSS content
        
        # Initialize modular components
        self.validator = ValidatorIntegration(self.workspace_root)
        self.feature_parser = FeatureParser(self.workspace_root)
        self.breaking_change_handler = BreakingChangesHandler(self.validator, self.feature_parser)
        self.html_generator = HTMLGenerator(self.workspace_root)
        
    def get_breaking_change_data_from_validator(self) -> Dict:
        """Get Breaking Change data from the Rust validator's JSON output."""
        return self.validator.get_breaking_change_data()
    
    def run_validator(self) -> Dict:
        """Run the tests format validator and parse its output."""
        try:
            # Run the validator from the validator directory with workspace and features parameters
            features_path = str(self.workspace_root / "tests" / "definitions")
            workspace_path = str(self.workspace_root)
            result = subprocess.run(
                ["cargo", "run", "--bin", "tests_format_validator", "--", 
                 "--workspace", workspace_path, 
                 "--features", features_path,
                 "--verbose"],
                cwd=self.validator_path,
                capture_output=True,
                text=True
            )
            
            if result.returncode not in [0, 1]:  # 1 is expected when orphaned tests are found
                print(f"Validator failed with return code {result.returncode}")
                print(f"stderr: {result.stderr}")
                print(f"stdout: {result.stdout}")
                return {}
            
            return self._parse_validator_output(result.stdout)
        except Exception as e:
            print(f"Error running validator: {e}")
            return {}
    
    def _extract_feature_tags(self, feature_path: str) -> List[str]:
        """Extract feature-level tags from a feature file."""
        try:
            with open(feature_path, 'r', encoding='utf-8') as f:
                content = f.read()
            
            lines = content.split('\n')
            feature_tags = []
            
            for line in lines:
                line = line.strip()
                
                # Collect tags (annotations starting with @) before Feature: line
                if line.startswith('@'):
                    tags = [tag.strip() for tag in line.split() if tag.startswith('@')]
                    feature_tags.extend(tags)
                elif line.startswith('Feature:'):
                    # Stop collecting once we hit the Feature: line
                    break
            
            return feature_tags
        except Exception as e:
            print(f"Error reading feature file {feature_path}: {e}")
            return []
    
    def _parse_validator_output(self, output: str) -> Dict:
        """Parse the validator text output into structured data."""
        features = {}
        current_feature = None
        current_language = None
        in_missing_steps_section = False
        
        lines = output.split('\n')
        for i, line in enumerate(lines):
            original_line = line
            line = line.strip()
            
            if line.startswith('📋 Feature:'):
                # Extract feature path
                feature_path = line.split('📋 Feature: ')[1]
                feature_name = Path(feature_path).stem
                current_feature = feature_name
                
                # Extract feature tags from the feature file
                feature_tags = self._extract_feature_tags(feature_path)
                
                features[current_feature] = {
                    'path': feature_path,
                    'tags': feature_tags,
                    'languages': {}
                }
                current_language = None
                in_missing_steps_section = False
                
            elif line.startswith('✅') or line.startswith('❌'):
                if current_feature:
                    # Parse language validation result
                    # Format: "  ✅ Language: path" or "  ❌ Language: path (validation failed)"
                    parts = line.split(': ', 1)
                    if len(parts) >= 2:
                        status_and_lang = parts[0].strip()
                        path_info = parts[1].strip()
                        
                        # Extract language
                        lang_part = status_and_lang.split(' ', 1)
                        if len(lang_part) >= 2:
                            status = '✅' if line.startswith('✅') else '❌'
                            language = lang_part[1]
                            current_language = language
                            
                            # Clean up path info (remove validation failed message)
                            test_path = path_info.split(' (validation failed)')[0]
                            
                            features[current_feature]['languages'][language] = {
                                'status': status,
                                'path': test_path,
                                'implemented': status == '✅',
                                'scenarios': {}  # Will store individual scenario status
                            }
                            in_missing_steps_section = False
                            
            elif line.startswith('⚠️  Missing steps by method:'):
                in_missing_steps_section = True
                
            elif in_missing_steps_section and current_feature and current_language:
                # Parse missing steps information
                # Look for lines like: "In method 'method_name' at line X (scenario: scenario_name):"
                if line.startswith('In method ') and ' (scenario: ' in line:
                    # Extract scenario name from the line
                    try:
                        scenario_part = line.split(' (scenario: ')[1]
                        scenario_name = scenario_part.split('):')[0]
                        
                        # Mark this scenario as failed for the current language
                        if current_language in features[current_feature]['languages']:
                            features[current_feature]['languages'][current_language]['scenarios'][scenario_name] = False
                    except (IndexError, ValueError):
                        pass  # Skip malformed lines
                        
        # After parsing, mark scenarios as implemented only if they have the appropriate driver tag
        for feature_name, feature_data in features.items():
            scenarios_with_annotations = self.get_feature_scenarios_with_annotations(feature_data['path'])
            for lang, lang_data in feature_data['languages'].items():
                if lang_data['implemented']:  # If overall language implementation passed
                    # Only mark scenarios as passed if they have the driver tag
                    for scenario_info in scenarios_with_annotations:
                        scenario_name = scenario_info['name']
                        scenario_tags = scenario_info['tags']
                        
                        # Check if this scenario has the driver tag
                        lang_lower = lang.lower()
                        # Handle core -> rust mapping
                        if lang_lower == 'rust':
                            tag_to_check = '@core'
                        else:
                            tag_to_check = f'@{lang_lower}'
                        
                        if tag_to_check in [tag.lower() for tag in scenario_tags]:
                            # Scenario has the driver tag, mark as implemented if not already set
                            if scenario_name not in lang_data['scenarios']:
                                lang_data['scenarios'][scenario_name] = True
                        # If scenario doesn't have the driver tag, don't mark it as implemented
                else:
                    # For failed languages, only mark scenarios with driver tags as failed
                    for scenario_info in scenarios_with_annotations:
                        scenario_name = scenario_info['name']
                        scenario_tags = scenario_info['tags']
                        
                        # Check if this scenario has the driver tag
                        lang_lower = lang.lower()
                        # Handle core -> rust mapping
                        if lang_lower == 'rust':
                            tag_to_check = '@core'
                        else:
                            tag_to_check = f'@{lang_lower}'
                        
                        if tag_to_check in [tag.lower() for tag in scenario_tags]:
                            # Scenario has the driver tag but language failed, mark as failed if not already set
                            if scenario_name not in lang_data['scenarios']:
                                lang_data['scenarios'][scenario_name] = False
        
        return features
    
    def get_feature_scenarios(self, feature_path: str) -> List[str]:
        """Extract scenario names from a feature file."""
        return self.feature_parser.get_feature_scenarios(feature_path)
    
    def get_feature_scenarios_with_annotations(self, feature_path: str) -> List[Dict[str, any]]:
        """Extract scenario names and their annotations (Breaking Change, expected) from a feature file."""
        return self.feature_parser.get_feature_scenarios_with_annotations(feature_path)

    def parse_breaking_change_descriptions(self, driver: str = 'odbc') -> Dict[str, str]:
        """Parse Breaking Change descriptions from the driver's Breaking Change.md file."""
        return self.breaking_change_handler.get_breaking_change_descriptions(driver)
    
    def extract_breaking_change_from_test_files(self, driver: str = 'odbc', features: Dict = None) -> Dict[str, Dict[str, List[Dict[str, any]]]]:
        """Extract Breaking Change annotations from test files and associate them with test methods.
        Only includes tests that correspond to scenarios defined in feature files.
        """
        return self.breaking_change_handler.get_breaking_change_test_mappings(driver, features)
    
    
    def is_scenario_todo_for_driver(self, scenario_info: Dict, driver: str) -> bool:
        """
        Determine if a scenario should be considered 'TODO' for a driver.
        
        New Logic:
        - If feature has @{driver}_not_needed: NO (all scenarios excluded)
        - If scenario has explicit @{driver}_not_needed: NO (scenario excluded)
        - If scenario has @{driver}: NO (already implemented)
        - If feature has no @{driver} annotation: YES (TODO by default)
        - Otherwise: NO
        """
        driver_lower = driver.lower()
        tags = scenario_info.get('tags', [])
        tag_strings = [tag.lower() for tag in tags]
        feature_tags = scenario_info.get('feature_level_tags', [])
        feature_tag_strings = [tag.lower() for tag in feature_tags]
        
        # Check if feature has @{driver}_not_needed (excludes all scenarios)
        if f'@{driver_lower}_not_needed' in feature_tag_strings:
            return False
        
        # Check if scenario is explicitly excluded with @{driver}_not_needed
        if f'@{driver_lower}_not_needed' in tag_strings:
            return False
        
        # Check if scenario has the driver tag (already implemented)
        # Handle both old format (@driver) and new format (@driver_e2e, @driver_int)
        has_driver_tag = (f'@{driver_lower}' in tag_strings or 
                         f'@{driver_lower}_e2e' in tag_strings or 
                         f'@{driver_lower}_int' in tag_strings)
        if has_driver_tag:
            return False
        
        # Check if feature has the driver tag
        # Handle both old format (@driver) and new format (@driver_e2e, @driver_int)
        feature_has_driver = (f'@{driver_lower}' in feature_tag_strings or 
                             f'@{driver_lower}_e2e' in feature_tag_strings or 
                             f'@{driver_lower}_int' in feature_tag_strings)
        
        # If feature has no driver annotation, it's TODO by default
        if not feature_has_driver:
            return True
        
        # If feature has driver tag but scenario doesn't (and isn't excluded), it's TODO
        if feature_has_driver and not has_driver_tag:
            return True
        
        return False
    
    def _has_driver_tag_in_scenario(self, scenario_info: Dict, driver: str) -> bool:
        """Check if scenario has driver tag in new format (@driver_e2e, @driver_int) or old format (@driver)."""
        tags = scenario_info.get('tags', [])
        return self._has_driver_tag_in_tags(tags, driver)
    
    def _has_driver_tag_in_tags(self, tags: List[str], driver: str) -> bool:
        """Check if tags contain driver tag in new format (@driver_e2e, @driver_int) or old format (@driver)."""
        tag_strings = [tag.lower() for tag in tags]
        driver_lower = driver.lower()
        return (f'@{driver_lower}' in tag_strings or 
                f'@{driver_lower}_e2e' in tag_strings or 
                f'@{driver_lower}_int' in tag_strings)
    
    def is_scenario_excluded_for_driver(self, scenario_info: Dict, driver: str) -> bool:
        """
        Determine if a scenario should be excluded (show as "-") for a driver.
        
        Logic:
        - If feature has @{driver}_not_needed: YES (all scenarios excluded)
        - If scenario has explicit @{driver}_not_needed: YES (scenario excluded)
        - Otherwise: NO
        """
        driver_lower = driver.lower()
        tags = scenario_info.get('tags', [])
        tag_strings = [tag.lower() for tag in tags]
        feature_tags = scenario_info.get('feature_level_tags', [])
        feature_tag_strings = [tag.lower() for tag in feature_tags]
        
        # Check if feature has @{driver}_not_needed (excludes all scenarios)
        if f'@{driver_lower}_not_needed' in feature_tag_strings:
            return True
        
        # Check if scenario is explicitly excluded with @{driver}_not_needed
        if f'@{driver_lower}_not_needed' in tag_strings:
            return True
        
        return False
    
    def get_all_languages(self, features: Dict) -> List[str]:
        """Get all languages mentioned across all features, with core first and rust renamed to core."""
        languages = set()
        for feature_data in features.values():
            languages.update(feature_data['languages'].keys())
        
        # Normalize language names to lowercase and handle special cases
        normalized_languages = set()
        for lang in languages:
            lang_lower = lang.lower()
            if lang_lower == 'rust':
                normalized_languages.add('core')
            else:
                normalized_languages.add(lang_lower)
        
        languages = normalized_languages
        
        # Convert to list and sort, but ensure 'core' comes first
        languages_list = sorted(languages)
        if 'core' in languages_list:
            languages_list.remove('core')
            languages_list.insert(0, 'core')
        
        return languages_list
    
    def get_language_data_key(self, normalized_lang: str) -> str:
        """Convert normalized language name back to the key used in feature data structure."""
        if normalized_lang == 'core':
            return 'Rust'  # Core maps back to Rust in the data structure
        elif normalized_lang == 'odbc':
            return 'Odbc'  # odbc maps back to Odbc in the data structure
        elif normalized_lang == 'python':
            return 'Python'  # python maps back to Python in the data structure
        else:
            return normalized_lang.capitalize()  # Default: capitalize first letter
    
    
    def format_feature_name(self, feature_name: str, feature_path: str = None) -> str:
        """Extract feature name from the feature file's 'Feature:' line."""
        if feature_path:
            try:
                with open(self.workspace_root / feature_path, 'r') as f:
                    content = f.read()
                
                lines = content.split('\n')
                for line in lines:
                    line = line.strip()
                    if line.startswith('Feature:'):
                        # Extract the feature name after "Feature:"
                        feature_title = line.replace('Feature:', '').strip()
                        if feature_title:
                            return feature_title
            except Exception as e:
                print(f"Error reading feature file {feature_path}: {e}")
        
        # Fallback to dynamic formatting if feature file parsing fails
        words = feature_name.replace('_', ' ').split()
        formatted_words = []
        
        for word in words:
            # Handle common abbreviations and special cases
            if word.lower() == 'auth':
                formatted_words.append('Authentication')
            elif word.lower() == 'api':
                formatted_words.append('API')
            elif word.lower() == 'sql':
                formatted_words.append('SQL')
            elif word.lower() == 'http':
                formatted_words.append('HTTP')
            elif word.lower() == 'jwt':
                formatted_words.append('JWT')
            elif word.lower() == 'ssl':
                formatted_words.append('SSL')
            elif word.lower() == 'tls':
                formatted_words.append('TLS')
            elif word.lower() == 'db':
                formatted_words.append('Database')
            else:
                formatted_words.append(word.capitalize())
        
        return ' '.join(formatted_words)
    
    def _convert_validator_data_to_features(self, validator_data: Dict) -> Dict:
        """Convert validator JSON data to the features format expected by the report generator."""
        features = {}
        
        for validation_result in validator_data.get('validation_results', []):
            feature_file_path = validation_result.get('feature_file', '')
            
            # Extract feature name from path
            feature_name = Path(feature_file_path).stem
            
            # Initialize feature data
            features[feature_name] = {
                'path': feature_file_path,
                'languages': {}
            }
            
            # Process each language validation
            for validation in validation_result.get('validations', []):
                language = validation.get('language', '')
                test_file_found = validation.get('test_file_found', False)
                test_file_path = validation.get('test_file_path', '')
                missing_steps = validation.get('missing_steps', [])
                implemented_steps = validation.get('implemented_steps', [])
                
                # Determine implementation status
                is_implemented = test_file_found and len(missing_steps) == 0 and len(implemented_steps) > 0
                status = '✅' if is_implemented else '❌'
                
                # Convert absolute path to relative path
                relative_path = test_file_path
                if test_file_path.startswith('./'):
                    relative_path = test_file_path[2:]  # Remove ./ prefix
                elif test_file_path.startswith(str(self.workspace_root)):
                    relative_path = Path(test_file_path).relative_to(self.workspace_root)
                
                features[feature_name]['languages'][language] = {
                    'implemented': is_implemented,
                    'status': status,
                    'path': str(relative_path),
                    'missing_steps': missing_steps,
                    'implemented_steps': implemented_steps
                }
        
        return features
    
    def _get_integration_test_file_path(self, language: str, feature_file_path: str) -> Path:
        """Get the integration test file path for a given language and feature."""
        from pathlib import Path
        
        # Extract feature info from the feature file path
        feature_path = Path(feature_file_path)
        feature_name = feature_path.stem
        
        # Get the subdirectory (e.g., authentication, query)
        if len(feature_path.parts) >= 2:
            subdir = feature_path.parts[-2]  # Get parent directory name
        else:
            subdir = None
        
        # Map language to integration test paths
        if language.lower() == 'core':
            # Rust integration tests
            if subdir:
                return self.workspace_root / f"sf_core/tests/integration/{subdir}/{feature_name}.rs"
            else:
                return self.workspace_root / f"sf_core/tests/integration/{feature_name}.rs"
        elif language.lower() == 'python':
            # Python integration tests
            if subdir:
                return self.workspace_root / f"pep249_dbapi/tests/integ/{subdir}/test_{feature_name}.py"
            else:
                return self.workspace_root / f"pep249_dbapi/tests/integ/test_{feature_name}.py"
        elif language.lower() == 'odbc':
            # ODBC integration tests
            if subdir:
                return self.workspace_root / f"odbc_tests/tests/integration/{subdir}/{feature_name}.cpp"
            else:
                return self.workspace_root / f"odbc_tests/tests/integration/{feature_name}.cpp"
        else:
            # Fallback - return a non-existent path
            return self.workspace_root / "non_existent_path"
    
    def extract_test_methods_with_lines(self, file_path: str, scenario_names: List[str]) -> Dict[str, int]:
        """Extract test method line numbers from a test file for given scenarios."""
        return self.feature_parser.extract_test_methods_with_lines(file_path, scenario_names)
    
    def _method_matches_scenario(self, method_name: str, scenario_name: str) -> bool:
        """Check if a method name matches a scenario name using similar logic to the Rust validator."""
        # Normalize both names for comparison
        def normalize(s):
            return re.sub(r'[^\w]', '', s.lower())
        
        # Convert scenario to different naming conventions
        scenario_snake = re.sub(r'[^\w\s]', '', scenario_name.lower()).replace(' ', '_')
        scenario_pascal = ''.join(word.capitalize() for word in re.sub(r'[^\w\s]', '', scenario_name).split())
        scenario_test_method = f"test_{scenario_snake}"
        
        method_normalized = normalize(method_name)
        
        return (method_normalized == normalize(scenario_name) or
                method_normalized == normalize(scenario_snake) or
                method_normalized == normalize(scenario_pascal) or
                method_normalized == normalize(scenario_test_method))
    
    def _get_breaking_change_ids_for_scenario(self, scenario_info: Dict, driver: str, features: Dict = None) -> List[str]:
        """Get all Breaking Change IDs associated with a scenario by looking up test implementations."""
        return self.breaking_change_handler.get_breaking_change_ids_for_scenario(scenario_info, driver, features)
    
    def generate_coverage_table(self, features: Dict) -> str:
        """Generate a coverage table showing implementation status."""
        if not features:
            return "No features found.\n"
        
        languages = self.get_all_languages(features)
        
        # Group features by folder
        features_by_folder = {}
        for feature_name, feature_data in features.items():
            # Extract folder from path
            path_parts = Path(feature_data['path']).parts
            if len(path_parts) >= 3 and path_parts[-3] == 'definitions':
                folder = path_parts[-2]  # Get the folder name (auth, query, etc.)
            else:
                folder = 'other'  # Fallback for unexpected paths
            
            if folder not in features_by_folder:
                features_by_folder[folder] = {}
            features_by_folder[folder][feature_name] = feature_data
        
        # Calculate column widths
        all_names = list(features.keys()) + [folder.replace('_', ' ').title() for folder in features_by_folder.keys()]
        feature_width = max(len("Feature"), max(len(name) for name in all_names) if all_names else 0)
        lang_width = max(8, max(len(lang) for lang in languages) if languages else 0)
        
        # Header
        table = []
        header = f"{'Feature':<{feature_width}}"
        for lang in languages:
            header += f" | {lang:<{lang_width}}"
        table.append(header)
        
        # Separator
        separator = "-" * feature_width
        for lang in languages:
            separator += "-|-" + "-" * lang_width
        table.append(separator)
        
        # Feature rows grouped by folder
        for folder, folder_features in sorted(features_by_folder.items()):
            # Folder header - full width with separator
            folder_display = folder.replace('_', ' ').title().upper()
            total_width = feature_width + sum(lang_width + 3 for _ in languages)  # +3 for " | "
            folder_header = f"=== {folder_display} " + "=" * (total_width - len(folder_display) - 4)
            table.append(folder_header)
            
            # Features in this folder
            for feature_name, feature_data in folder_features.items():
                formatted_name = self.format_feature_name(feature_name, feature_data['path'])
                row = f"  {formatted_name:<{feature_width-2}}"  # Indent feature under folder
                for lang in languages:
                    lang_key = self.get_language_data_key(lang)
                    if lang_key in feature_data['languages']:
                        status = feature_data['languages'][lang_key]['status']
                        status_text = "PASS" if status == '✅' else "FAIL"
                    else:
                        status_text = "N/A"
                    row += f" | {status_text:<{lang_width}}"
                table.append(row)
        
        return '\n'.join(table)
    

    
    def _get_html_styles(self) -> str:
        """Get CSS styles for the HTML report."""
        # Use cached CSS if available
        if self._css_cache is not None:
            return self._css_cache
            
        # Read CSS from external file
        css_file = Path(__file__).parent / 'styles.css'
        try:
            with open(css_file, 'r', encoding='utf-8') as f:
                self._css_cache = f.read()
                return self._css_cache
        except FileNotFoundError:
            # Fallback to basic inline styles if CSS file is missing
            return '''
            body { font-family: Arial, sans-serif; margin: 20px; }
            table { width: 100%; border-collapse: collapse; }
            th, td { padding: 8px; border: 1px solid #ddd; text-align: left; }
            th { background-color: #f2f2f2; }
            .tick-icon { color: green; font-weight: bold; }
            .status-na { color: #666; }
            .breaking_change-superscript-link { 
                color: #FFD700; 
                text-decoration: none; 
                font-weight: bold;
            }
            .breaking_change-superscript-link:hover { 
                text-decoration: underline; 
            }
        '''

    def _create_html_document(self, title: str, body_content: str) -> str:
        """Create a complete HTML document with head and body."""
        from textwrap import dedent
        
        # Clean up body content indentation
        body_content = dedent(body_content).strip()
        
        return dedent(f"""
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>{title}</title>
                <style>
                    {self._get_html_styles()}
                </style>
                <script>
                    function showTab(tabId) {{
                        // Hide all tab contents
                        const tabContents = document.querySelectorAll('.tab-content');
                        tabContents.forEach(content => content.classList.remove('active'));
                        
                        // Remove active class from all tab buttons
                        const tabButtons = document.querySelectorAll('.tab-button');
                        tabButtons.forEach(button => button.classList.remove('active'));
                        
                        // Show selected tab content
                        document.getElementById(tabId).classList.add('active');
                        
                        // Add active class to the corresponding button
                        const targetButton = document.querySelector(`[onclick*="showTab('${{tabId}})"]`);
                        if (targetButton) {{
                            targetButton.classList.add('active');
                        }}
                    }}
                    
                    function toggleSection(element) {{
                        const header = element;
                        const content = header.nextElementSibling;
                        const toggle = header.querySelector('.expandable-toggle');
                        
                        // Toggle expanded state
                        const isExpanded = header.classList.contains('expanded');
                        
                        if (isExpanded) {{
                            header.classList.remove('expanded');
                            content.classList.remove('expanded');
                            toggle.classList.remove('expanded');
                        }} else {{
                            header.classList.add('expanded');
                            content.classList.add('expanded');
                            toggle.classList.add('expanded');
                        }}
                    }}
                    
                    function expandSection(element) {{
                        const header = element;
                        const content = header.nextElementSibling;
                        const toggle = header.querySelector('.expandable-toggle');
                        
                        // Expand the section
                        header.classList.add('expanded');
                        content.classList.add('expanded');
                        toggle.classList.add('expanded');
                    }}
                    
                    function expandToFeature(featureId) {{
                        const featureElement = document.getElementById(featureId);
                        if (featureElement) {{
                            // Find and expand the parent category section
                            const categorySection = featureElement.closest('.expandable-content').previousElementSibling;
                            if (categorySection && categorySection.classList.contains('expandable-header')) {{
                                expandSection(categorySection);
                            }}
                            
                            // Expand the feature section itself
                            const featureHeader = featureElement.querySelector('.expandable-header');
                            if (featureHeader) {{
                                expandSection(featureHeader);
                            }}
                        }}
                    }}
                    
                    function navigateToBreakingChange(breaking_changeId) {{
                        // First, close all tabs and open Breaking Change tab
                        showTab('breaking_change-tab');
                        
                        // Collapse all expandable sections in all tabs
                        const allExpandableSections = document.querySelectorAll('.expandable-section');
                        allExpandableSections.forEach(section => {{
                            const header = section.querySelector('.expandable-header');
                            const content = section.querySelector('.expandable-content');
                            const toggle = section.querySelector('.expandable-toggle');
                            if (header && content && toggle) {{
                                header.classList.remove('expanded');
                                content.classList.remove('expanded');
                                toggle.classList.remove('expanded');
                            }}
                        }});
                        
                        const breaking_changeElement = document.getElementById(breaking_changeId);
                        if (breaking_changeElement) {{
                            // Find and expand the parent driver section
                            const driverSection = breaking_changeElement.closest('.expandable-content').previousElementSibling;
                            if (driverSection && driverSection.classList.contains('expandable-header')) {{
                                expandSection(driverSection);
                            }}
                            
                            // Expand the specific Breaking Change section
                            const breaking_changeHeader = breaking_changeElement.querySelector('.expandable-header');
                            if (breaking_changeHeader) {{
                                expandSection(breaking_changeHeader);
                            }}
                            
                            // Scroll to the Breaking Change section
                            setTimeout(() => breaking_changeElement.scrollIntoView({{behavior: 'smooth'}}), 100);
                        }}
                    }}
                    
                    function toggleBreakingChangePopup(popupId) {{
                        // Hide all other Breaking Change popups first
                        const allPopups = document.querySelectorAll('.breaking_change-popup');
                        allPopups.forEach(popup => {{
                            if (popup.id !== popupId) {{
                                popup.classList.remove('show');
                            }}
                        }});
                        
                        // Toggle the clicked popup
                        const popup = document.getElementById(popupId);
                        if (popup) {{
                            popup.classList.toggle('show');
                        }}
                    }}
                    
                    function hideBreakingChangePopup(popupId) {{
                        const popup = document.getElementById(popupId);
                        if (popup) {{
                            popup.classList.remove('show');
                        }}
                    }}
                    
                    // Close Breaking Change popups when clicking outside
                    document.addEventListener('click', function(event) {{
                        if (!event.target.closest('.breaking_change-popup-container')) {{
                            const allPopups = document.querySelectorAll('.breaking_change-popup');
                            allPopups.forEach(popup => {{
                                popup.classList.remove('show');
                            }});
                        }}
                    }});
                    
                    // Show first tab by default when page loads
                    document.addEventListener('DOMContentLoaded', function() {{
                        const firstTab = document.querySelector('.tab-button');
                        const firstTabContent = document.querySelector('.tab-content');
                        if (firstTab && firstTabContent) {{
                            firstTab.classList.add('active');
                            firstTabContent.classList.add('active');
                        }}
                    }});
                    
                    // Expand/Collapse functionality
                    function toggleCategory(categoryId) {{
                        const categoryHeader = document.querySelector(`[onclick*="${{categoryId}}"]`);
                        const categoryRows = document.querySelectorAll(`[data-category="${{categoryId}}"]`);
                        
                        if (categoryHeader.classList.contains('collapsed')) {{
                            // Expand
                            categoryHeader.classList.remove('collapsed');
                            categoryRows.forEach(row => {{
                                row.style.display = '';
                                row.classList.remove('collapsed');
                            }});
                        }} else {{
                            // Collapse
                            categoryHeader.classList.add('collapsed');
                            categoryRows.forEach(row => {{
                                row.style.display = 'none';
                                row.classList.add('collapsed');
                            }});
                        }}
                    }}
                    
                    function toggleFeature(featureId) {{
                        const featureHeader = document.querySelector(`[onclick*="${{featureId}}"]`);
                        const featureRows = document.querySelectorAll(`[data-feature="${{featureId}}"]`);
                        
                        if (featureHeader.classList.contains('collapsed')) {{
                            // Expand
                            featureHeader.classList.remove('collapsed');
                            featureRows.forEach(row => {{
                                row.style.display = '';
                                row.classList.remove('collapsed');
                            }});
                        }} else {{
                            // Collapse
                            featureHeader.classList.add('collapsed');
                            featureRows.forEach(row => {{
                                row.style.display = 'none';
                                row.classList.add('collapsed');
                            }});
                        }}
                    }}
                    
                    function expandAll() {{
                        // Expand all categories
                        const categoryHeaders = document.querySelectorAll('.folder-name');
                        categoryHeaders.forEach(header => {{
                            header.classList.remove('collapsed');
                        }});
                        
                        // Expand all features
                        const featureHeaders = document.querySelectorAll('.feature-name');
                        featureHeaders.forEach(header => {{
                            header.classList.remove('collapsed');
                        }});
                        
                        // Show all rows
                        const allCollapsibleRows = document.querySelectorAll('[data-category], [data-feature]');
                        allCollapsibleRows.forEach(row => {{
                            row.style.display = '';
                            row.classList.remove('collapsed');
                        }});
                    }}
                    
                    function collapseAll() {{
                        // Collapse all categories
                        const categoryHeaders = document.querySelectorAll('.folder-name');
                        categoryHeaders.forEach(header => {{
                            header.classList.add('collapsed');
                        }});
                        
                        // Collapse all features
                        const featureHeaders = document.querySelectorAll('.feature-name');
                        featureHeaders.forEach(header => {{
                            header.classList.add('collapsed');
                        }});
                        
                        // Hide all rows
                        const allCollapsibleRows = document.querySelectorAll('[data-category], [data-feature]');
                        allCollapsibleRows.forEach(row => {{
                            row.style.display = 'none';
                            row.classList.add('collapsed');
                        }});
                    }}
                </script>
            </head>
            <body>
                <div class="container">
                    {body_content}
                </div>
            </body>
            </html>
        """).strip()

    def _generate_coverage_table_html(self, features: Dict, languages: List[str]) -> str:
        """Generate the coverage overview table HTML."""
        from textwrap import dedent
        
        # Build table header
        header_cells = ['<th>Feature</th>'] + [f'<th>{lang.title()}</th>' for lang in languages]
        
        # Group features by folder
        features_by_folder = {}
        for feature_name, feature_data in features.items():
            # Extract folder from path
            path_parts = Path(feature_data['path']).parts
            if len(path_parts) >= 3 and path_parts[-3] == 'definitions':
                folder = path_parts[-2]  # Get the folder name (auth, query, etc.)
            else:
                folder = 'other'  # Fallback for unexpected paths
            
            if folder not in features_by_folder:
                features_by_folder[folder] = {}
            features_by_folder[folder][feature_name] = feature_data
        
        # Build table rows grouped by folder
        rows = []
        for folder, folder_features in sorted(features_by_folder.items()):
            # Folder header row - span across all columns
            folder_display = folder.replace('_', ' ').title()
            colspan = len(languages) + 1  # +1 for the feature column
            folder_id = f"category-{folder.lower().replace('_', '-')}"
            folder_cell = f'<td colspan="{colspan}"><div class="folder-name" onclick="toggleCategory(\'{folder_id}\')">{folder_display}</div></td>'
            rows.append(f'<tr>{folder_cell}</tr>')
            
            # Start category content wrapper
            category_rows = []
            
            for feature_name, feature_data in folder_features.items():
                formatted_name = self.format_feature_name(feature_name, feature_data['path'])
                scenarios_with_annotations = self.get_feature_scenarios_with_annotations(feature_data['path'])
                scenarios = [s['name'] for s in scenarios_with_annotations] if scenarios_with_annotations else self.get_feature_scenarios(feature_data['path'])
                
                # Generate unique ID for this feature (same as in detailed breakdown)
                feature_id = f"feature-{feature_name.replace('_', '-').replace(' ', '-').lower()}"
                
                # Feature header row with collapsible functionality
                feature_cells = [f'<td><div class="feature-name" onclick="toggleFeature(\'{feature_id}\')">{formatted_name}</div></td>']
                
                # Add status cells for each language at feature level
                for lang in languages:
                    data_key = self.get_language_data_key(lang)
                    
                    # Check if any scenarios in this feature are expected (in progress) for this driver
                    has_expected_scenarios = False
                    has_breaking_change_scenarios = False
                    has_implemented_scenarios = False
                    has_failed_scenarios = False
                    has_relevant_scenarios = False
                    
                    for scenario_info in scenarios_with_annotations:
                        is_todo_for_driver = self.is_scenario_todo_for_driver(scenario_info, lang)
                        breaking_change_driver_check = self.get_language_data_key(lang).lower() if lang == 'core' else lang.lower()
                        
                        # Check if there are actual Breaking Change implementations for this driver and scenario
                        breaking_change_ids_for_this_driver = self._get_breaking_change_ids_for_scenario(scenario_info, breaking_change_driver_check, features)
                        is_breaking_change_for_this_driver = bool(breaking_change_ids_for_this_driver)
                        has_driver_tag = self._has_driver_tag_in_scenario(scenario_info, lang.lower())
                        
                        if is_todo_for_driver or is_breaking_change_for_this_driver or has_driver_tag:
                            has_relevant_scenarios = True
                            
                            if is_todo_for_driver:
                                has_expected_scenarios = True
                            elif is_breaking_change_for_this_driver:
                                has_breaking_change_scenarios = True
                            elif has_driver_tag:
                                # Check if implemented
                                scenario_implemented = False
                                if data_key in feature_data['languages']:
                                    lang_data = feature_data['languages'][data_key]
                                    scenario_name = scenario_info['name']
                                    if 'scenarios' in lang_data and scenario_name in lang_data['scenarios']:
                                        scenario_implemented = lang_data['scenarios'][scenario_name]
                                    elif lang_data['status'] == '✅' and self._has_driver_tag_in_scenario(scenario_info, lang.lower()):
                                        # Only consider implemented if the scenario has the driver tag AND the feature is implemented
                                        scenario_implemented = True
                                
                                if scenario_implemented:
                                    has_implemented_scenarios = True
                                else:
                                    has_failed_scenarios = True
                    
                    # Determine feature-level status based on scenario analysis
                    if not has_relevant_scenarios:
                        # No scenarios relevant for this driver
                        feature_cells.append('<td><div class="test-status"><span class="status-na">-</span></div></td>')
                    elif has_expected_scenarios:
                        # Has expected (in progress) scenarios
                        feature_cells.append('<td><div class="test-status"><span class="status-in-progress">TODO</span></div></td>')
                    elif has_failed_scenarios:
                        # Has failed scenarios
                        feature_cells.append('<td><div class="test-status"><span class="status-fail">✗</span></div></td>')
                    elif has_implemented_scenarios or has_breaking_change_scenarios:
                        # All scenarios are implemented or Breaking Change
                        feature_cells.append('<td><div class="test-status"><span class="tick-icon">✓</span></div></td>')
                    else:
                        # Fallback to not applicable
                        feature_cells.append('<td><div class="test-status"><span class="status-na">-</span></div></td>')
                
                category_rows.append(f'<tr class="feature-row">{"".join(feature_cells)}</tr>')
                
                # Individual test rows - collect in feature content
                feature_test_rows = []
                for i, scenario_info in enumerate(scenarios_with_annotations if scenarios_with_annotations else [{'name': s, 'breaking_change_info': None, 'expected_drivers': [], 'tags': []} for s in scenarios]):
                    scenario = scenario_info['name'] if isinstance(scenario_info, dict) else scenario_info
                    breaking_change_info = scenario_info.get('breaking_change_info') if isinstance(scenario_info, dict) else None
                    expected_drivers = scenario_info.get('expected_drivers', []) if isinstance(scenario_info, dict) else []
                    tags = scenario_info.get('tags', []) if isinstance(scenario_info, dict) else []
                    
                    is_last_test = i == len(scenarios) - 1
                    row_class = "test-row" if not is_last_test else "test-row"
                    
                    # Test name cell with link to detailed breakdown
                    test_class = "test-name last-test" if is_last_test else "test-name"
                    
                    # Determine test level for inline label
                    has_int_tag = any(tag.endswith('_int') for tag in tags)
                    test_level_label = '<span class="test-level-integration">Integration</span>' if has_int_tag else '<span class="test-level-e2e">E2E</span>'
                    
                    # Create unique scenario ID for navigation (same logic as detailed breakdown)
                    scenario_clean = scenario.lower().replace(' ', '-').replace('(', '').replace(')', '').replace("'", '').replace('"', '')
                    scenario_id = f"scenario-{feature_id}-{scenario_clean}"
                    
                    test_cell = f'<td><div class="{test_class}">• <a href="#" onclick="showTab(\'details-tab\'); expandToFeature(\'{feature_id}\'); setTimeout(() => document.getElementById(\'{scenario_id}\').scrollIntoView({{behavior: \'smooth\', block: \'center\'}}), 200); return false;">{scenario} {test_level_label}</a></div></td>'
                    
                    # Status cells for each language
                    status_cells = []
                    for lang in languages:
                        data_key = self.get_language_data_key(lang)
                        if data_key in feature_data['languages']:
                            lang_data = feature_data['languages'][data_key]
                            
                            # Check scenario implementation status
                            scenario_implemented = False
                            # Only consider a scenario implemented if it has the driver tag
                            if self._has_driver_tag_in_tags(tags, lang.lower()):
                                if 'scenarios' in lang_data and scenario in lang_data['scenarios']:
                                    # Use specific scenario data if available and scenario has the tag
                                    scenario_implemented = lang_data['scenarios'][scenario]
                                else:
                                    # If scenario has the driver tag, use the overall feature status
                                    scenario_implemented = lang_data['status'] == '✅'
                            # If scenario doesn't have the driver tag, it's not applicable (scenario_implemented stays False)
                            
                            # Check if this scenario is expected (in progress) for this driver
                            is_todo_for_driver = self.is_scenario_todo_for_driver(scenario_info, lang)
                            
                            # Check if this is a Breaking Change for this driver by looking at actual Breaking Change implementations
                            breaking_change_driver_check = self.get_language_data_key(lang).lower() if lang == 'core' else lang.lower()
                            
                            # Check if there are actual Breaking Change implementations for this driver and scenario
                            breaking_change_ids_for_link = self._get_breaking_change_ids_for_scenario(scenario_info, breaking_change_driver_check, features)
                            is_breaking_change_for_this_driver = bool(breaking_change_ids_for_link)
                            
                            # Determine status based on annotations and implementation
                            # Prioritize expected status over Breaking Change status
                            if is_todo_for_driver:
                                # This scenario is expected (in progress) for this driver - prioritize this over Breaking Change
                                status_cells.append('<td><div class="test-status"><span class="status-in-progress">TODO</span></div></td>')
                            elif is_breaking_change_for_this_driver:
                                # Breaking Change scenario for this specific driver - show Breaking Change popup with list
                                if breaking_change_ids_for_link:
                                    # Generate unique popup ID for this scenario
                                    scenario_clean = scenario.lower().replace(" ", "-").replace("'", "")
                                    popup_id = f'breaking_change-popup-{scenario_clean}-{lang.lower()}'
                                    
                                    # Create Breaking Change list items
                                    breaking_change_items = []
                                    for breaking_change_id in breaking_change_ids_for_link:
                                        breaking_change_section_id = f'breaking_change-{breaking_change_driver_check}-{breaking_change_id.lower().replace("#", "")}'
                                        breaking_change_items.append(f'<li><a href="#" onclick="navigateToBreakingChange(\'{breaking_change_section_id}\'); hideBreakingChangePopup(\'{popup_id}\'); return false;" class="breaking_change-popup-link">{breaking_change_id}</a></li>')
                                    
                                    # Create clickable Breaking Change numbers for superscript display
                                    breaking_change_links = []
                                    for breaking_change_id in breaking_change_ids_for_link:
                                        # Extract number from BC#1 format
                                        if breaking_change_id.startswith('BC#'):
                                            number = breaking_change_id[3:]  # Remove 'BC#' prefix
                                            breaking_change_section_id = f'breaking_change-{breaking_change_driver_check}-{breaking_change_id.lower().replace("#", "")}'
                                            breaking_change_links.append(f'<a href="#" onclick="navigateToBreakingChange(\'{breaking_change_section_id}\'); return false;" class="breaking_change-superscript-link">{number}</a>')
                                    
                                    superscript_links = ','.join(breaking_change_links)
                                    
                                    # Create green checkmark with clickable superscript Breaking Change numbers
                                    status_cells.append(f'''
                                        <td><div class="test-status">
                                            <span class="tick-icon">✓<sup>{superscript_links}</sup></span>
                                        </div></td>
                                    ''')
                                else:
                                    # Scenario has no actual Breaking Change implementations
                                    status_cells.append('<td><div class="test-status"><span class="status-na">-</span></div></td>')
                            elif scenario_implemented:
                                # Regular implemented scenario
                                status_cells.append('<td><div class="test-status"><span class="tick-icon">✓</span></div></td>')
                            elif self._has_driver_tag_in_tags(tags, lang.lower()):
                                # Has driver tag but not implemented
                                status_cells.append('<td><div class="test-status"><span class="status-fail">✗</span></div></td>')
                            else:
                                # Check if scenario is excluded or TODO for this driver
                                is_excluded = self.is_scenario_excluded_for_driver(scenario_info, lang)
                                is_todo = self.is_scenario_todo_for_driver(scenario_info, lang)
                                
                                if is_excluded:
                                    status_cells.append('<td><div class="test-status"><span class="status-na">-</span></div></td>')
                                elif is_todo:
                                    status_cells.append('<td><div class="test-status"><span class="status-in-progress">TODO</span></div></td>')
                                else:
                                    # Not applicable for this driver
                                    status_cells.append('<td><div class="test-status"><span class="status-na">-</span></div></td>')
                        else:
                            # Language not present for this feature
                            # Check if this scenario is expected (in progress) for this driver
                            is_todo_for_driver = self.is_scenario_todo_for_driver(scenario_info, lang)
                            
                            # Check if this is a Breaking Change for this driver by looking at actual Breaking Change implementations
                            breaking_change_driver_check = self.get_language_data_key(lang).lower() if lang == 'core' else lang.lower()
                            
                            # Check if there are actual Breaking Change implementations for this driver and scenario
                            breaking_change_ids_for_link = self._get_breaking_change_ids_for_scenario(scenario_info, breaking_change_driver_check, features)
                            is_breaking_change_for_this_driver = bool(breaking_change_ids_for_link)
                            
                            # Prioritize expected status over Breaking Change status
                            if is_todo_for_driver:
                                # This scenario is expected (in progress) for this driver - prioritize this over Breaking Change
                                status_cells.append('<td><div class="test-status"><span class="status-in-progress">TODO</span></div></td>')
                            elif is_breaking_change_for_this_driver:
                                # Breaking Change scenario for this specific driver (even if not implemented) - show Breaking Change popup with list
                                if breaking_change_ids_for_link:
                                    # Generate unique popup ID for this scenario
                                    scenario_clean = scenario.lower().replace(" ", "-").replace("'", "")
                                    popup_id = f'breaking_change-popup-{scenario_clean}-{lang.lower()}'
                                    
                                    # Create Breaking Change list items
                                    breaking_change_items = []
                                    for breaking_change_id in breaking_change_ids_for_link:
                                        breaking_change_section_id = f'breaking_change-{breaking_change_driver_check}-{breaking_change_id.lower().replace("#", "")}'
                                        breaking_change_items.append(f'<li><a href="#" onclick="navigateToBreakingChange(\'{breaking_change_section_id}\'); hideBreakingChangePopup(\'{popup_id}\'); return false;" class="breaking_change-popup-link">{breaking_change_id}</a></li>')
                                    
                                    # Create clickable Breaking Change numbers for superscript display
                                    breaking_change_links = []
                                    for breaking_change_id in breaking_change_ids_for_link:
                                        # Extract number from BC#1 format
                                        if breaking_change_id.startswith('BC#'):
                                            number = breaking_change_id[3:]  # Remove 'BC#' prefix
                                            breaking_change_section_id = f'breaking_change-{breaking_change_driver_check}-{breaking_change_id.lower().replace("#", "")}'
                                            breaking_change_links.append(f'<a href="#" onclick="navigateToBreakingChange(\'{breaking_change_section_id}\'); return false;" class="breaking_change-superscript-link">{number}</a>')
                                    
                                    superscript_links = ','.join(breaking_change_links)
                                    
                                    # Create green checkmark with clickable superscript Breaking Change numbers
                                    status_cells.append(f'''
                                        <td><div class="test-status">
                                            <span class="tick-icon">✓<sup>{superscript_links}</sup></span>
                                        </div></td>
                                    ''')
                                else:
                                    # Scenario has no actual Breaking Change implementations
                                    status_cells.append('<td><div class="test-status"><span class="status-na">-</span></div></td>')
                            else:
                                # Check if scenario is excluded or TODO for this driver
                                is_excluded = self.is_scenario_excluded_for_driver(scenario_info, lang)
                                is_todo = self.is_scenario_todo_for_driver(scenario_info, lang)
                                
                                if is_excluded:
                                    status_cells.append('<td><div class="test-status"><span class="status-na">-</span></div></td>')
                                elif is_todo:
                                    status_cells.append('<td><div class="test-status"><span class="status-in-progress">TODO</span></div></td>')
                                else:
                                    # Not applicable for this driver
                                    status_cells.append('<td><div class="test-status"><span class="status-na">-</span></div></td>')
                    
                    category_rows.append(f'<tr class="{row_class} feature-content" data-feature="{feature_id}">{test_cell}{"".join(status_cells)}</tr>')
            
            # Add all category rows with proper data attributes
            for row in category_rows:
                if 'feature-row' in row:
                    rows.append(row.replace('<tr class="feature-row">', f'<tr class="feature-row" data-category="{folder_id}">'))
                else:
                    rows.append(row.replace('<tr class="', f'<tr class="').replace('">', f'" data-category="{folder_id}">'))
        
        rows_html = '\n'.join(f'                    {row}' for row in rows)
        
        return dedent(f"""
            <h2>📊 Coverage Overview</h2>
            <div class="expand-collapse-controls">
                <span class="expand-collapse-btn" onclick="expandAll()">📖 Expand All</span>
                <span class="expand-collapse-btn" onclick="collapseAll()">📕 Collapse All</span>
            </div>
            <div class="status-legend">
                <h4>📋 Statuses</h4>
                <div class="legend-items">
                    <div class="legend-item">
                        <span class="legend-symbol tick">✓</span>
                        <span>Implemented</span>
                    </div>
                    <div class="legend-item">
                        <span class="legend-symbol cross">✗</span>
                        <span>Test format errors</span>
                    </div>
                    <div class="legend-item">
                        <span class="legend-symbol todo">TODO</span>
                        <span>To be implemented</span>
                    </div>
                    <div class="legend-item">
                        <span class="legend-symbol na">-</span>
                        <span>Not Applicable</span>
                    </div>
                    <div class="legend-item">
                        <span class="legend-symbol bc">✓<sup>1,2</sup></span>
                        <span>Breaking Change</span>
                    </div>
                </div>
            </div>
            <table>
                <thead>
                    <tr>{"".join(header_cells)}</tr>
                </thead>
                <tbody>
                    {rows_html}
                </tbody>
            </table>
        """).strip()

    def _generate_language_coverage_html(self, features: Dict, languages: List[str]) -> str:
        """Generate the language coverage section HTML based only on expected tests."""
        from textwrap import dedent
        
        cards = []
        for lang in languages:
            expected_count = 0
            implemented_count = 0
            
            for feature_name, feature_data in features.items():
                # Get scenarios with annotations for this feature
                scenarios_with_annotations = self.get_feature_scenarios_with_annotations(feature_data['path'])
                
                if not scenarios_with_annotations:
                    continue
                
                for scenario_info in scenarios_with_annotations:
                    scenario = scenario_info['name']
                    expected_drivers = scenario_info['expected_drivers']
                    tags = scenario_info['tags']
                    
                    # Check if this scenario is expected/relevant for this driver
                    # Check if there are actual Breaking Change implementations for this driver and scenario
                    breaking_change_ids_for_this_driver = self._get_breaking_change_ids_for_scenario(scenario_info, lang.lower(), features)
                    is_breaking_change_for_this_driver = bool(breaking_change_ids_for_this_driver)
                    is_todo_for_driver = self.is_scenario_todo_for_driver(scenario_info, lang)
                    has_driver_implementation = self._has_driver_tag_in_tags(tags, lang.lower())
                    
                    # Count as expected if:
                    # 1. Has driver tag (@{lang})
                    # 2. Is Breaking Change for this driver (@{lang}_breaking_change)
                    # 3. Is TODO for this driver
                    is_relevant = has_driver_implementation or is_breaking_change_for_this_driver or is_todo_for_driver
                    
                    if is_relevant:
                        expected_count += 1
                        
                        # Check if it's implemented
                        scenario_implemented = False
                        data_key = self.get_language_data_key(lang)
                        if data_key in feature_data['languages']:
                            lang_data = feature_data['languages'][data_key]
                            # Only consider a scenario implemented if it has the driver tag
                            if self._has_driver_tag_in_scenario(scenario_info, lang.lower()):
                                if 'scenarios' in lang_data and scenario in lang_data['scenarios']:
                                    # Use specific scenario data if available and scenario has the tag
                                    scenario_implemented = lang_data['scenarios'][scenario]
                                else:
                                    # If scenario has the driver tag, use the overall feature status
                                    scenario_implemented = lang_data['status'] == '✅'
                            # If scenario doesn't have the driver tag, it's not applicable (scenario_implemented stays False)
                        
                        # Count as implemented if:
                        # 1. Actually implemented OR
                        # 2. Is Breaking Change for this driver (Breaking Change counts as implemented)
                        # Note: Expected (in progress) scenarios are NOT counted as implemented
                        if scenario_implemented or is_breaking_change_for_this_driver:
                            implemented_count += 1
            
            lang_coverage = (implemented_count / expected_count) * 100 if expected_count > 0 else 0
            coverage_class = 'coverage-high' if lang_coverage >= 80 else 'coverage-medium' if lang_coverage >= 60 else 'coverage-low'
            
            # Handle case where no expected tests exist
            if expected_count == 0:
                coverage_display = "N/A"
                test_cases_display = "0/0 expected tests"
                coverage_class = 'coverage-na'
            else:
                coverage_display = f"{lang_coverage:.1f}%"
                test_cases_display = f"{implemented_count}/{expected_count} expected tests"
            
            cards.append(dedent(f"""
                <div class="summary-card">
                    <h3>{lang.title()}</h3>
                    <div class="value {coverage_class}">{coverage_display}</div>
                    <div style="font-size: 0.9em; color: #6c757d;">{test_cases_display}</div>
                </div>
            """).strip())
        
        cards_html = '\n            '.join(cards)
        
        return dedent(f"""
            <h2>📈 Language Coverage for Expected Tests</h2>
            <div class="language-coverage">
                {cards_html}
            </div>
        """).strip()

    def _generate_detailed_breakdown_html(self, features: Dict) -> str:
        """Generate the detailed breakdown section HTML with scenarios and their implementation status."""
        from textwrap import dedent
        from pathlib import Path
        
        # Get all languages for consistent ordering
        languages = self.get_all_languages(features)
        
        # Group features by folder
        features_by_folder = {}
        for feature_name, feature_data in features.items():
            # Extract folder from path
            path_parts = Path(feature_data['path']).parts
            if len(path_parts) >= 3 and path_parts[-3] == 'definitions':
                folder = path_parts[-2]  # Get the folder name (auth, query, etc.)
            else:
                folder = 'other'  # Fallback for unexpected paths
            
            if folder not in features_by_folder:
                features_by_folder[folder] = {}
            features_by_folder[folder][feature_name] = feature_data
        
        # Generate expandable sections for each folder
        category_sections = []
        for folder, folder_features in sorted(features_by_folder.items()):
            folder_display = folder.replace('_', ' ').title()
            
            # Generate feature sections for this folder
            feature_sections = []
            for feature_name, feature_data in folder_features.items():
                formatted_name = self.format_feature_name(feature_name, feature_data['path'])
                
                # Generate unique ID for this feature (needed for scenario navigation)
                feature_id = f"feature-{feature_name.replace('_', '-').replace(' ', '-').lower()}"
                
                # Get scenarios with annotations for this feature
                scenarios_with_annotations = self.get_feature_scenarios_with_annotations(feature_data['path'])
            
                # Group scenarios by test level (e2e vs integration)
                e2e_scenarios = []
                integration_scenarios = []
                
                if scenarios_with_annotations:
                    for scenario_info in scenarios_with_annotations:
                        tags = scenario_info['tags']
                        # Check if scenario has integration tags
                        has_int_tag = any(tag.endswith('_int') for tag in tags)
                        
                        if has_int_tag:
                            integration_scenarios.append(scenario_info)
                        else:
                            e2e_scenarios.append(scenario_info)
                
                # Build scenario list with inline E2E/Integration labels
                scenario_items = []
                
                # Process scenarios if we have any with annotations
                if scenarios_with_annotations:
                    # Process all scenarios (e2e first, then integration)
                    # Use the categorized scenarios only - this ensures each scenario appears once with proper labels
                    for scenario_info in e2e_scenarios + integration_scenarios:
                        scenario = scenario_info['name']
                        breaking_change_info = scenario_info['breaking_change_info']
                        expected_drivers = scenario_info['expected_drivers']
                        tags = scenario_info['tags']
                    
                        # Determine test level for inline label
                        has_int_tag = any(tag.endswith('_int') for tag in tags)
                        test_level_label = ' <span class="test-level-integration">Integration</span>' if has_int_tag else ' <span class="test-level-e2e">E2E</span>'
                        
                        # Build implementation status for this specific scenario
                        impl_items = []
                        display_languages = [lang for lang in languages if self.get_language_data_key(lang) in feature_data['languages']]
                        
                        for lang in sorted(display_languages):
                            data_key = self.get_language_data_key(lang)
                            lang_data = feature_data['languages'][data_key]
                            
                            # Use the overall feature implementation status from Coverage Overview
                            # This ensures consistency with the validator's rstest detection
                            scenario_implemented = lang_data['implemented']
                            line_info = ""
                            
                            # Still determine file path for display purposes
                            actual_file_path = None
                            if has_int_tag:
                                # Integration test - look in integration directories
                                actual_file_path = self._get_integration_test_file_path(lang, feature_data['path'])
                                if actual_file_path and actual_file_path.exists():
                                    # Update the display path for integration tests
                                    relative_path = actual_file_path.relative_to(self.workspace_root)
                                    lang_data['path'] = str(relative_path)
                            else:
                                # E2E test - use the path from validator data
                                if lang_data['implemented'] and 'path' in lang_data:
                                    actual_file_path = self.workspace_root / lang_data['path']
                            
                            # Check if this is a Breaking Change for this driver
                            breaking_change_driver_check = self.get_language_data_key(lang).lower() if lang == 'core' else lang.lower()
                            breaking_change_ids_for_this_driver = self._get_breaking_change_ids_for_scenario(scenario_info, breaking_change_driver_check, features)
                            is_breaking_change_for_this_driver = bool(breaking_change_ids_for_this_driver)
                            
                            # Check if this scenario is expected (in progress) for this driver
                            is_todo_for_driver = self.is_scenario_todo_for_driver(scenario_info, lang)
                            
                            # Determine badge class and status text based on annotations
                            if is_todo_for_driver:
                                badge_class = 'lang-in-progress'
                                status_text = 'TODO'
                            elif is_breaking_change_for_this_driver:
                                badge_class = 'lang-breaking_change'
                                breaking_change_links = []
                                for breaking_change_id in breaking_change_ids_for_this_driver:
                                    if breaking_change_id.startswith('BC#'):
                                        number = breaking_change_id[3:]
                                        breaking_change_section_id = f'breaking_change-{breaking_change_driver_check}-{breaking_change_id.lower().replace("#", "")}'
                                        breaking_change_links.append(f'<a href="#" onclick="navigateToBreakingChange(\'{breaking_change_section_id}\'); return false;" class="breaking_change-superscript-link">{number}</a>')
                                superscript_links = ','.join(breaking_change_links)
                                status_text = f'✓<sup>{superscript_links}</sup>'
                            elif scenario_implemented:
                                badge_class = 'lang-implemented'
                                status_text = '✅'
                            elif self._has_driver_tag_in_tags(tags, lang.lower()):
                                badge_class = 'lang-failed'
                                status_text = '❌ MISSING'
                            else:
                                badge_class = 'lang-na'
                                status_text = '-'
                            
                            # Add this language's implementation status
                            impl_items.append(dedent(f"""
                                <li>
                                    <span class="lang-badge {badge_class}">{lang.title()}</span>
                                    {status_text}
                                    {f'<span class="path-info">{lang_data["path"]}{line_info}</span>' if scenario_implemented else ''}
                                </li>
                            """).strip())
                        
                        impl_html = '\n                        '.join(impl_items)
                        
                        # Create unique scenario ID for navigation
                        scenario_clean = scenario.lower().replace(' ', '-').replace('(', '').replace(')', '').replace("'", '').replace('"', '')
                        scenario_id = f"scenario-{feature_id}-{scenario_clean}"
                        
                        scenario_item = dedent(f"""
                            <div class="scenario-section" id="{scenario_id}">
                                <h5 class="scenario-title">📝 {scenario}{test_level_label}</h5>
                                <ul class="implementation-list">
                                    {impl_html}
                                </ul>
                            </div>
                        """).strip()
                        
                        scenario_items.append(scenario_item)
                else:
                    # Fallback: show overall implementation status if no scenarios found
                    # Use the old method for backward compatibility
                    scenarios = self.get_feature_scenarios(feature_data['path'])
                    if scenarios:
                        # If we have scenarios but couldn't parse Breaking Change info, treat as regular scenarios
                        for scenario in scenarios:
                            impl_items = []
                            for lang in sorted(feature_data['languages'].keys()):
                                lang_data = feature_data['languages'][lang]
                                
                                scenario_implemented = False
                                line_info = ""
                                
                                if lang_data['implemented'] and 'path' in lang_data:
                                    file_path = self.workspace_root / lang_data['path']
                                    if file_path.exists():
                                        method_lines = self.extract_test_methods_with_lines(str(file_path), [scenario])
                                        if method_lines and scenario in method_lines:
                                            scenario_implemented = True
                                            line_info = f" (line {method_lines[scenario]})"
                                
                                badge_class = 'lang-implemented' if scenario_implemented else 'lang-failed'
                                status_text = '✅' if scenario_implemented else '❌ MISSING'
                                
                                impl_items.append(dedent(f"""
                                    <li>
                                        <span class="lang-badge {badge_class}">{lang}</span>
                                        {status_text}
                                        {f'<span class="path-info">{lang_data["path"]}{line_info}</span>' if scenario_implemented else ''}
                                    </li>
                                """).strip())
                            
                            impl_html = '\n                            '.join(impl_items)
                            scenario_items.append(dedent(f"""
                                <div class="scenario-section">
                                    <h5 class="scenario-title">📝 {scenario}</h5>
                                    <ul class="implementation-list">
                                        {impl_html}
                                    </ul>
                                </div>
                            """).strip())
                    else:
                        # No scenarios at all - show overall feature status
                        impl_items = []
                        for lang in sorted(feature_data['languages'].keys()):
                            lang_data = feature_data['languages'][lang]
                            badge_class = 'lang-implemented' if lang_data['implemented'] else 'lang-failed'
                            status_text = '✅' if lang_data['implemented'] else '❌ MISSING'
                            
                            impl_items.append(dedent(f"""
                                <li>
                                    <span class="lang-badge {badge_class}">{lang}</span>
                                    {status_text}
                                    <span class="path-info">{lang_data["path"]}</span>
                                </li>
                            """).strip())
                        
                        impl_html = '\n                        '.join(impl_items)
                        scenario_items.append(dedent(f"""
                            <div class="scenario-section">
                                <h5 class="scenario-title">📝 Overall Feature Implementation</h5>
                                <ul class="implementation-list">
                                    {impl_html}
                                </ul>
                            </div>
                        """).strip())
                
                scenarios_html = '\n                    '.join(scenario_items)
                
                feature_section = dedent(f"""
                    <div class="expandable-section" id="{feature_id}">
                        <div class="expandable-header" onclick="toggleSection(this)">
                            <div class="expandable-title">🔍 {formatted_name}</div>
                            <div class="expandable-toggle">▶</div>
                        </div>
                        <div class="expandable-content">
                            <div class="expandable-inner">
                                <p><strong>Path:</strong> <code>{feature_data["path"]}</code></p>
                                <div class="scenarios-with-implementation">
                                    {scenarios_html}
                                </div>
                            </div>
                        </div>
                    </div>
                """).strip()
                
                feature_sections.append(feature_section)
            
            # Create expandable category section
            features_html = '\n            '.join(feature_sections)
            
            category_section = dedent(f"""
                <div class="expandable-section">
                    <div class="expandable-header" onclick="toggleSection(this)">
                        <div class="expandable-title">📂 {folder_display}</div>
                        <div class="expandable-toggle">▶</div>
                    </div>
                    <div class="expandable-content">
                        <div class="expandable-inner">
                            {features_html}
                        </div>
                    </div>
                </div>
            """).strip()
            
            category_sections.append(category_section)
        
        sections_html = '\n        '.join(category_sections)
        
        return dedent(f"""
            <h2>📋 Detailed Breakdown</h2>
            {sections_html}
        """).strip()

    def _generate_test_level_section(self, section_title: str, scenarios: List, feature_data: Dict, languages: List[str]) -> str:
        """Generate HTML for a test level section (E2E or Integration) with expandable/collapsible functionality."""
        from textwrap import dedent
        
        scenario_items = []
        for scenario_info in scenarios:
            scenario = scenario_info['name']
            breaking_change_info = scenario_info['breaking_change_info']
            expected_drivers = scenario_info['expected_drivers']
            tags = scenario_info['tags']
            
            # Build implementation status for this specific scenario
            impl_items = []
            # Use display languages but map to data keys for lookup
            display_languages = [lang for lang in languages if self.get_language_data_key(lang) in feature_data['languages']]
            for lang in sorted(display_languages):
                data_key = self.get_language_data_key(lang)
                lang_data = feature_data['languages'][data_key]
                
                # Check if this specific scenario is implemented in this language
                scenario_implemented = False
                line_info = ""
                
                if lang_data['implemented'] and 'path' in lang_data:
                    # Convert relative path to absolute path
                    file_path = self.workspace_root / lang_data['path']
                    if file_path.exists():
                        method_lines = self.extract_test_methods_with_lines(str(file_path), [scenario])
                        if method_lines and scenario in method_lines:
                            scenario_implemented = True
                            line_info = f" (line {method_lines[scenario]})"
                
                # Check if this is a Breaking Change for this driver by looking at actual Breaking Change implementations
                breaking_change_driver_check = self.get_language_data_key(lang).lower() if lang == 'core' else lang.lower()
                
                # Check if there are actual Breaking Change implementations for this driver and scenario
                breaking_change_ids_for_this_driver = self._get_breaking_change_ids_for_scenario(scenario_info, breaking_change_driver_check, feature_data)
                is_breaking_change_for_this_driver = bool(breaking_change_ids_for_this_driver)
                
                # Check if this scenario is expected (in progress) for this driver
                is_todo_for_driver = self.is_scenario_todo_for_driver(scenario_info, lang)
                
                # Determine badge class and status text based on annotations
                if is_todo_for_driver:
                    badge_class = 'lang-in-progress'
                    status_text = 'TODO'
                elif is_breaking_change_for_this_driver:
                    badge_class = 'lang-breaking_change'
                    # Create clickable Breaking Change numbers for superscript display
                    breaking_change_driver_check = self.get_language_data_key(lang).lower() if lang == 'core' else lang.lower()
                    breaking_change_links = []
                    for breaking_change_id in breaking_change_ids_for_this_driver:
                        # Extract number from BC#1 format
                        if breaking_change_id.startswith('BC#'):
                            number = breaking_change_id[3:]  # Remove 'BC#' prefix
                            breaking_change_section_id = f'breaking_change-{breaking_change_driver_check}-{breaking_change_id.lower().replace("#", "")}'
                            breaking_change_links.append(f'<a href="#" onclick="navigateToBreakingChange(\'{breaking_change_section_id}\'); return false;" class="breaking_change-superscript-link">{number}</a>')
                    
                    superscript_links = ','.join(breaking_change_links)
                    status_text = f'✓<sup>{superscript_links}</sup>'
                elif scenario_implemented:
                    badge_class = 'lang-implemented'
                    status_text = '✅'
                elif f'@{lang.lower()}' in [tag.lower() for tag in tags]:
                    # Has driver tag but not implemented
                    badge_class = 'lang-failed'
                    status_text = '❌ MISSING'
                else:
                    # Not applicable for this driver
                    badge_class = 'lang-na'
                    status_text = '-'
                
                impl_items.append(dedent(f"""
                    <li>
                        <span class="lang-badge {badge_class}">{lang.title()}</span>
                        {status_text}
                        {f'<span class="path-info">{lang_data["path"]}{line_info}</span>' if scenario_implemented else ''}
                    </li>
                """).strip())
            
            impl_html = '\n                        '.join(impl_items)
            
            scenario_item = dedent(f"""
                <div class="scenario-section">
                    <h5 class="scenario-title">📝 {scenario}</h5>
                    <ul class="implementation-list">
                        {impl_html}
                    </ul>
                </div>
            """).strip()
            
            scenario_items.append(scenario_item)
        
        scenarios_html = '\n                    '.join(scenario_items)
        
        return dedent(f"""
            <div class="test-level-section">
                <div class="expandable-header test-level-header" onclick="toggleSection(this)">
                    <div class="expandable-title">🧪 {section_title}</div>
                    <div class="expandable-toggle">▶</div>
                </div>
                <div class="expandable-content">
                    <div class="expandable-inner">
                        {scenarios_html}
                    </div>
                </div>
            </div>
        """).strip()

    def _generate_missing_implementations_html(self, features: Dict, languages: List[str]) -> str:
        """Generate the missing implementations section HTML grouped by driver."""
        from textwrap import dedent
        from collections import defaultdict
        
        # Find missing expected implementations grouped by language/driver
        missing_by_driver = defaultdict(list)
        
        for feature_name, feature_data in features.items():
            # Get scenarios with annotations for this feature
            scenarios_with_annotations = self.get_feature_scenarios_with_annotations(feature_data['path'])
            
            if not scenarios_with_annotations:
                continue
                
            for scenario_info in scenarios_with_annotations:
                scenario = scenario_info['name']
                
                # Check each language to see if this scenario is expected but missing
                for lang in languages:
                    # Check if this scenario is expected (in progress) for this driver
                    is_todo = self.is_scenario_todo_for_driver(scenario_info, lang)
                    
                    if is_todo:
                        # Check if this expected scenario is implemented
                        scenario_implemented = False
                        data_key = self.get_language_data_key(lang)
                        if data_key in feature_data['languages']:
                            lang_data = feature_data['languages'][data_key]
                            # Only consider a scenario implemented if it has the driver tag
                            if self._has_driver_tag_in_scenario(scenario_info, lang.lower()):
                                if 'scenarios' in lang_data and scenario in lang_data['scenarios']:
                                    # Use specific scenario data if available and scenario has the tag
                                    scenario_implemented = lang_data['scenarios'][scenario]
                                else:
                                    # If scenario has the driver tag, use the overall feature status
                                    scenario_implemented = lang_data['status'] == '✅'
                            # If scenario doesn't have the driver tag, it's not applicable (scenario_implemented stays False)
                        
                        # Also check if it's a Breaking Change scenario (Breaking Change scenarios are considered "implemented")
                        breaking_change_driver_check = self.get_language_data_key(lang).lower() if lang == 'core' else lang.lower()
                        
                        # Check if there are actual Breaking Change implementations for this driver and scenario
                        breaking_change_ids_for_this_driver = self._get_breaking_change_ids_for_scenario(scenario_info, breaking_change_driver_check, features)
                        is_breaking_change_for_this_driver = bool(breaking_change_ids_for_this_driver)
                        
                        # Only add to missing if it's expected but not implemented and not a Breaking Change
                        if not scenario_implemented and not is_breaking_change_for_this_driver:
                            missing_by_driver[lang].append({
                                'feature': feature_name,
                                'scenario': scenario,
                                'formatted_feature': self.format_feature_name(feature_name, feature_data['path'])
                            })
        
        # Remove drivers with no missing expected implementations
        missing_by_driver = {k: v for k, v in missing_by_driver.items() if v}
        
        if not missing_by_driver:
            return dedent("""
                <div class="missing-implementations">
                    <h3>⚠️ Missing Expected Implementations</h3>
                    <p>All expected tests are implemented! 🎉</p>
                </div>
            """).strip()
        
        # Generate driver sections
        driver_sections = []
        for driver in sorted(missing_by_driver.keys()):
            missing_items = missing_by_driver[driver]
            
            # Group by feature for better organization
            by_feature = defaultdict(list)
            for item in missing_items:
                by_feature[item['feature']].append(item['scenario'])
            
            # Build feature list for this driver
            feature_items = []
            for feature_name in sorted(by_feature.keys()):
                scenarios = by_feature[feature_name]
                # Find the formatted feature name from any item
                formatted_feature = next(item['formatted_feature'] for item in missing_items if item['feature'] == feature_name)
                
                scenario_list = '\n                        '.join([f'<li>• {scenario}</li>' for scenario in sorted(scenarios)])
                feature_items.append(dedent(f"""
                    <li>
                        <strong>{formatted_feature}</strong>
                        <ul class="scenario-list">
                            {scenario_list}
                        </ul>
                    </li>
                """).strip())
            
            feature_list_html = '\n                '.join(feature_items)
            
            # Create expandable section for each driver
            driver_section = dedent(f"""
                <div class="expandable-section">
                    <div class="expandable-header" onclick="toggleSection(this)">
                        <div class="expandable-title">🔧 {driver.upper()} Driver ({len(missing_items)} missing)</div>
                        <div class="expandable-toggle">▶</div>
                    </div>
                    <div class="expandable-content">
                        <div class="expandable-inner">
                            <ul>
                                {feature_list_html}
                            </ul>
                        </div>
                    </div>
                </div>
            """).strip()
            
            driver_sections.append(driver_section)
        
        sections_html = '\n        '.join(driver_sections)
        
        return dedent(f"""
            <div class="missing-implementations">
                <h3>⚠️ Missing Expected Implementations</h3>
                <p>The following expected tests are missing implementations in specific drivers:</p>
                {sections_html}
            </div>
        """).strip()
    
    def _generate_breaking_change_tab_html(self, features: Dict) -> str:
        """Generate the Breaking Changes tab HTML content showing all Breaking Change implementations by driver."""
        from textwrap import dedent
        from collections import defaultdict
        
        # Get all drivers that have Breaking Changes from the validator data
        drivers_with_breaking_change = set()
        
        # Get Breaking Change data from validator
        breaking_change_data = self.breaking_change_handler.validator.get_breaking_change_data()
        if breaking_change_data:
            breaking_change_by_language = breaking_change_data.get('breaking_changes_by_language', {})
            for language, breaking_change_list in breaking_change_by_language.items():
                if breaking_change_list:  # Only add if there are actual Breaking Changes
                    drivers_with_breaking_change.add(language.lower())
        
        # Also check feature annotations as backup
        for feature_name, feature_data in features.items():
            scenarios_with_annotations = self.get_feature_scenarios_with_annotations(feature_data['path'])
            for scenario_info in scenarios_with_annotations:
                breaking_change_info = scenario_info.get('breaking_change_info')
                if breaking_change_info:
                    drivers_with_breaking_change.add(breaking_change_info['driver'].lower())
        
        if not drivers_with_breaking_change:
            return dedent("""
                <h2>📋 Breaking Changes</h2>
                <p>No Breaking Change annotations found in the test suite.</p>
            """).strip()
        
        # Generate sections for each driver with Breaking Changes
        driver_sections = []
        
        for driver in sorted(drivers_with_breaking_change):
            # Parse Breaking Change descriptions for this driver
            breaking_change_descriptions = self.parse_breaking_change_descriptions(driver)
            
            # Extract Breaking Change test implementations for this driver
            breaking_change_test_mapping = self.extract_breaking_change_from_test_files(driver, features)
            
            # Collect all Breaking Changes mentioned in features and test files
            all_breaking_changes = set()
            
            # Breaking Changes from feature files
            feature_breaking_changes = defaultdict(list)
            for feature_name, feature_data in features.items():
                scenarios_with_annotations = self.get_feature_scenarios_with_annotations(feature_data['path'])
                for scenario_info in scenarios_with_annotations:
                    breaking_change_info = scenario_info.get('breaking_change_info')
                    if breaking_change_info and breaking_change_info['driver'].lower() == driver:
                        # Extract Breaking Change ID from tag (e.g., @odbc_breaking_change -> look for BC# in description)
                        # For now, we'll use a placeholder as we need to correlate with actual Breaking Change IDs
                        feature_breaking_changes['Breaking Change_FROM_FEATURE'].append({
                            'feature': self.format_feature_name(feature_name, feature_data['path']),
                            'scenario': scenario_info['name'],
                            'tag': breaking_change_info['tag']
                        })
            
            # Breaking Changes from test files
            if driver in breaking_change_test_mapping:
                for breaking_change_id in breaking_change_test_mapping[driver].keys():
                    all_breaking_changes.add(breaking_change_id)
            
            # Generate Breaking Change sections
            breaking_change_sections = []
            
            for breaking_change_id in sorted(all_breaking_changes):
                description = breaking_change_descriptions.get(breaking_change_id, 'No description available')
                
                # Create title with driver prefix (without full description in title)
                # Convert BC#1 to DRIVER#1 format
                if breaking_change_id.startswith('BC#'):
                    driver_breaking_change_id = f"{driver.upper()}#{breaking_change_id[3:]}"  # Replace BC# with DRIVER#
                else:
                    driver_breaking_change_id = breaking_change_id
                
                # Extract just the first line for the title
                if description and description != 'No description available':
                    first_line = description.split('\n')[0]
                    breaking_change_title = f"{driver_breaking_change_id}: {first_line}"
                    
                    # Prepare detailed description for expandable section
                    description_lines = description.split('\n')
                    if len(description_lines) > 1:
                        # Multi-line description - show details in expandable section
                        detailed_description = '<br>'.join(description_lines)
                        has_details = True
                    else:
                        # Single line description - no need for separate details section
                        detailed_description = description
                        has_details = False
                else:
                    breaking_change_title = driver_breaking_change_id
                    detailed_description = 'No description available'
                    has_details = False
                
                # Get test implementations for this Breaking Change
                test_implementations = breaking_change_test_mapping.get(driver, {}).get(breaking_change_id, [])
                
                                # Build test implementation list
                impl_items = []
                for impl in test_implementations:
                    test_line_info = f"{impl['test_line']}" if impl.get('test_line') else "unknown"
                    
                    # Build behaviour lines
                    behaviour_lines = []
                    
                    # New behaviour (NEW_DRIVER_ONLY)
                    if impl.get('new_behaviour_file') and impl.get('new_behaviour_line'):
                        new_file = impl['new_behaviour_file']
                        new_line = impl['new_behaviour_line']
                        behaviour_lines.append(f"<strong>New Behaviour:</strong> <code>{new_file}:{new_line}</code>")
                    
                    # Old behaviour (OLD_DRIVER_ONLY)
                    if impl.get('old_behaviour_file') and impl.get('old_behaviour_line'):
                        old_file = impl['old_behaviour_file']
                        old_line = impl['old_behaviour_line']
                        behaviour_lines.append(f"<strong>Old Behaviour:</strong> <code>{old_file}:{old_line}</code>")
                    
                    # If no new/old behaviour found, fall back to legacy assertion field
                    if not behaviour_lines:
                        assertion_file = impl.get('assertion_file', impl['test_file'])
                        assertion_line = impl.get('assertion_line', impl.get('line_number', 'unknown'))
                        behaviour_lines.append(f"<strong>Assertion:</strong> <code>{assertion_file}:{assertion_line}</code>")
                    
                    behaviour_html = "<br>".join(behaviour_lines)
                    
                    impl_items.append(dedent(f"""
                        <li>
                            <strong>Test Method:</strong> {impl['test_method']}<br>
                            <strong>Test:</strong> <code>{impl['test_file']}:{test_line_info}</code><br>
                            {behaviour_html}
                        </li>
                    """).strip())
                
                impl_html = '\n                    '.join(impl_items) if impl_items else '<li><em>No test implementations found</em></li>'
                
                # Build the details section HTML (static, not expandable)
                details_section_html = ""
                if has_details:
                    details_section_html = f"""
                                <div class="breaking_change-details">
                                    <h4>Details</h4>
                                    <p>{detailed_description}</p>
                                </div>
                    """
                
                breaking_change_section = dedent(f"""
                    <div class="expandable-section" id="breaking_change-{driver}-{breaking_change_id.lower().replace('#', '')}">
                        <div class="expandable-header" onclick="toggleSection(this)">
                            <div class="expandable-title">{breaking_change_title}</div>
                            <div class="expandable-toggle">▶</div>
                        </div>
                        <div class="expandable-content">
                            <div class="expandable-inner">{details_section_html}
                                <h4>Test Implementations</h4>
                                <ul class="implementation-list">
                                    {impl_html}
                                </ul>
                            </div>
                        </div>
                    </div>
                """).strip()
                
                breaking_change_sections.append(breaking_change_section)
            
            # Create driver section
            breaking_changes_html = '\n            '.join(breaking_change_sections) if breaking_change_sections else '<p><em>No Breaking Changes found for this driver</em></p>'
            
            driver_section = dedent(f"""
                <div class="expandable-section">
                    <div class="expandable-header" onclick="toggleSection(this)">
                        <div class="expandable-title">📂 {driver.upper()} Driver Breaking Changes ({len(all_breaking_changes)})</div>
                        <div class="expandable-toggle">▶</div>
                    </div>
                    <div class="expandable-content">
                        <div class="expandable-inner">
                            {breaking_changes_html}
                        </div>
                    </div>
                </div>
            """).strip()
            
            driver_sections.append(driver_section)
        
        sections_html = '\n        '.join(driver_sections)
        
        return dedent(f"""
            <h2>📋 Breaking Changes</h2>
            <p>Breaking Changes represent behavior changes between old and new driver implementations. Each Breaking Change is documented with its description and corresponding test implementations.</p>
            {sections_html}
        """).strip()

    def generate_html_report(self, features: Dict) -> str:
        """Generate an HTML coverage report with tabbed interface."""
        from textwrap import dedent
        
        if not features:
            return "<html><body><h1>No features found</h1></body></html>"
        
        languages = self.get_all_languages(features)
        
        # Generate content for each tab
        overview_content = self._generate_coverage_table_html(features, languages) + '\n\n' + self._generate_language_coverage_html(features, languages)
        detailed_content = self._generate_detailed_breakdown_html(features)
        missing_impl_html = self._generate_missing_implementations_html(features, languages)
        
        # Create tab structure
        sections = ['<h1>Universal Driver Test Coverage Report</h1>']
        
        # Generate Breaking Changes tab content
        breaking_change_content = self._generate_breaking_change_tab_html(features)
        
        # Create tabs container
        tabs_html = dedent("""
            <div class="tabs">
                <div class="tab-buttons">
                    <button class="tab-button" onclick="showTab('overview-tab')">📊 Overview</button>
                    <button class="tab-button" onclick="showTab('breaking_change-tab')">📋 Breaking Changes</button>
                    <button class="tab-button" onclick="showTab('details-tab')">📋 Detailed Breakdown</button>
                    <button class="tab-button" onclick="showTab('missing-tab')">⚠️ Missing Implementations</button>
                </div>
                
                <div id="overview-tab" class="tab-content">
                    {overview_content}
                </div>
                
                <div id="breaking_change-tab" class="tab-content">
                    {breaking_change_content}
                </div>
                
                <div id="details-tab" class="tab-content">
                    {detailed_content}
                </div>
                
                <div id="missing-tab" class="tab-content">
                    {missing_content}
                </div>
            </div>
        """).format(
            overview_content=overview_content,
            breaking_change_content=breaking_change_content,
            detailed_content=detailed_content,
            missing_content=missing_impl_html if missing_impl_html else '<p>No missing implementations found! ✅</p>'
        )
        
        sections.append(tabs_html)
        
        # Add footer
        sections.extend([
            '<hr>',
            '<p style="text-align: center; color: #6c757d; margin-top: 30px;">Generated by Test Coverage Report Generator</p>'
        ])
        
        body_content = '\n\n'.join(sections)
        return self._create_html_document("Universal Driver Test Coverage Report", body_content)


def main():
    parser = argparse.ArgumentParser(description='Generate test coverage reports')
    parser.add_argument('--workspace', '-w', default='.', 
                       help='Workspace root directory (default: current directory)')
    parser.add_argument('--format', '-f', choices=['table', 'html'], 
                       default='html', help='Output format (default: html)')
    parser.add_argument('--output', '-o', help='Output file (default: stdout)')
    
    args = parser.parse_args()
    
    generator = CoverageReportGenerator(args.workspace)
    
    print("Generating test coverage report...")
    validator_data = generator.validator.get_validator_data()
    
    if not validator_data or 'validation_results' not in validator_data:
        print("No features found or validator failed to run.")
        sys.exit(1)
    
    # Convert validator data to features format
    features = generator._convert_validator_data_to_features(validator_data)
    
    # Generate report based on format
    if args.format == 'table':
        report = generator.generate_coverage_table(features)
    elif args.format == 'html':
        report = generator.generate_html_report(features)
    else:
        raise ValueError(f"Unsupported format: {args.format}")
    
    # Output report
    if args.format == 'html':
        # For HTML format, default to saving to file
        output_file = args.output or 'universal_driver_e2e_test_coverage.html'
        with open(output_file, 'w') as f:
            f.write(report)
        print(f"HTML report saved to {output_file}")
    else:
        # For table format, default to stdout unless output specified
        if args.output:
            with open(args.output, 'w') as f:
                f.write(report)
            print(f"Report saved to {args.output}")
        else:
            print("\n" + report)


if __name__ == '__main__':
    main()
