#!/usr/bin/env python3
"""
Universal Driver Test Coverage Report Generator

Generates coverage reports showing which scenarios and tests are implemented
across different languages based on e2e feature files.
"""

import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Set, Optional, Tuple
import argparse


class CoverageReportGenerator:
    """Generates test coverage reports for multi-language test suites."""
    
    def __init__(self, workspace_root: str):
        self.workspace_root = Path(workspace_root).resolve()  # Convert to absolute path
        self.validator_path = self.workspace_root / "tests" / "tests_format_validator"
        
    def run_validator(self) -> Dict:
        """Run the tests format validator and parse its output."""
        try:
            # Run the validator from the validator directory with workspace and features parameters
            features_path = str(self.workspace_root / "tests" / "e2e")
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
                features[current_feature] = {
                    'path': feature_path,
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
                        
        # After parsing, mark all scenarios not explicitly failed as passed for languages with overall success
        for feature_name, feature_data in features.items():
            scenarios = self.get_feature_scenarios(feature_data['path'])
            for lang, lang_data in feature_data['languages'].items():
                if lang_data['implemented']:  # If overall language implementation passed
                    # Mark all scenarios as passed (since no failures were found)
                    for scenario in scenarios:
                        if scenario not in lang_data['scenarios']:
                            lang_data['scenarios'][scenario] = True
                else:
                    # For failed languages, mark unspecified scenarios as passed
                    # (only explicitly failed scenarios are marked as False above)
                    for scenario in scenarios:
                        if scenario not in lang_data['scenarios']:
                            lang_data['scenarios'][scenario] = True
        
        return features
    
    def get_feature_scenarios(self, feature_path: str) -> List[str]:
        """Extract scenario names from a feature file."""
        scenarios = []
        try:
            with open(self.workspace_root / feature_path, 'r') as f:
                content = f.read()
                
            lines = content.split('\n')
            for line in lines:
                line = line.strip()
                if line.startswith('Scenario:'):
                    scenario_name = line.replace('Scenario:', '').strip()
                    scenarios.append(scenario_name)
        except Exception as e:
            print(f"Error reading feature file {feature_path}: {e}")
            
        return scenarios
    
    def get_all_languages(self, features: Dict) -> Set[str]:
        """Get all languages mentioned across all features."""
        languages = set()
        for feature_data in features.values():
            languages.update(feature_data['languages'].keys())
        return sorted(languages)
    
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
    
    def extract_test_methods_with_lines(self, file_path: str, scenario_names: List[str]) -> Dict[str, int]:
        """Extract test method line numbers from a test file for given scenarios."""
        if not os.path.exists(file_path):
            return {}
        
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read()
        except Exception:
            return {}
        
        lines = content.split('\n')
        method_lines = {}
        
        # Determine file type and patterns
        if file_path.endswith('.rs'):
            # Rust: #[test] followed by fn method_name
            for i, line in enumerate(lines):
                if line.strip() == '#[test]' and i + 1 < len(lines):
                    fn_match = re.search(r'fn\s+(\w+)', lines[i + 1])
                    if fn_match:
                        method_name = fn_match.group(1)
                        # Check if this method matches any scenario
                        for scenario in scenario_names:
                            if self._method_matches_scenario(method_name, scenario):
                                method_lines[scenario] = i + 2  # +2 for 1-indexed and fn is on next line
                                break
        
        elif file_path.endswith('.java'):
            # Java: @Test followed by method declaration
            for i, line in enumerate(lines):
                if line.strip() == '@Test':
                    # Look ahead for method declaration
                    for j in range(i + 1, min(i + 5, len(lines))):
                        method_match = re.search(r'(?:public\s+)?(?:void\s+)?(\w+)\s*\(', lines[j])
                        if method_match:
                            method_name = method_match.group(1)
                            # Check if this method matches any scenario
                            for scenario in scenario_names:
                                if self._method_matches_scenario(method_name, scenario):
                                    method_lines[scenario] = j + 1  # +1 for 1-indexed
                                    break
                            break
        
        elif file_path.endswith('.cpp'):
            # C++: TEST_CASE("method_name")
            for i, line in enumerate(lines):
                test_case_match = re.search(r'TEST_CASE\s*\(\s*"([^"]+)"', line)
                if test_case_match:
                    method_name = test_case_match.group(1)
                    # Check if this method matches any scenario
                    for scenario in scenario_names:
                        if self._method_matches_scenario(method_name, scenario):
                            method_lines[scenario] = i + 1  # +1 for 1-indexed
                            break
        
        elif file_path.endswith('.py'):
            # Python: def test_method_name
            for i, line in enumerate(lines):
                test_match = re.search(r'def\s+(test_\w+)\s*\(', line)
                if test_match:
                    method_name = test_match.group(1)
                    # Check if this method matches any scenario
                    for scenario in scenario_names:
                        if self._method_matches_scenario(method_name, scenario):
                            method_lines[scenario] = i + 1  # +1 for 1-indexed
                            break
        
        return method_lines
    
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
    
    def generate_coverage_table(self, features: Dict) -> str:
        """Generate a coverage table showing implementation status."""
        if not features:
            return "No features found.\n"
        
        languages = self.get_all_languages(features)
        
        # Group features by folder
        features_by_folder = {}
        for feature_name, feature_data in features.items():
            # Extract folder from path (e.g., "tests/e2e/auth/private_key_auth.feature" -> "auth")
            path_parts = Path(feature_data['path']).parts
            if len(path_parts) >= 3 and path_parts[-3] == 'e2e':
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
                    if lang in feature_data['languages']:
                        status = feature_data['languages'][lang]['status']
                        status_text = "PASS" if status == '✅' else "FAIL"
                    else:
                        status_text = "N/A"
                    row += f" | {status_text:<{lang_width}}"
                table.append(row)
        
        return '\n'.join(table)
    

    
    def _get_html_styles(self) -> str:
        """Get CSS styles for the HTML report."""
        return '''
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
            color: #333;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        h1 {
            color: #2c3e50;
            border-bottom: 3px solid #3498db;
            padding-bottom: 10px;
        }
        h2 {
            color: #34495e;
            margin-top: 30px;
        }
        .summary {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin: 20px 0;
        }
        .language-coverage {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin: 20px 0;
        }
        .summary-card {
            background: #ecf0f1;
            padding: 20px;
            border-radius: 6px;
            text-align: center;
        }
        .summary-card h3 {
            margin: 0 0 10px 0;
            color: #2c3e50;
        }
        .summary-card .value {
            font-size: 2em;
            font-weight: bold;
            color: #3498db;
        }
        .coverage-high { color: #27ae60; }
        .coverage-medium { color: #f39c12; }
        .coverage-low { color: #e74c3c; }
        table {
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
            background: white;
        }
        th, td {
            padding: 12px;
            text-align: left;
            border: 1px solid #bdc3c7;
        }
        th {
            background-color: #34495e;
            color: white;
            font-weight: 600;
        }
        tr:nth-child(even) {
            background-color: #f8f9fa;
        }
        .status-pass {
            background-color: #d4edda;
            color: #155724;
            padding: 4px 8px;
            border-radius: 4px;
            font-weight: bold;
        }
        .tick-icon {
            color: #28a745;
            font-weight: bold;
            font-size: 1.2em;
        }
        .folder-name {
            font-weight: bold;
            font-size: 1.3em;
            color: #1a202c;
            padding: 12px 0;
            background-color: #e2e8f0;
            border-left: 6px solid #4a5568;
            padding-left: 16px;
            margin-left: -12px;
            margin-right: -12px;
            text-transform: uppercase;
            letter-spacing: 0.5px;
            text-align: center;
        }
        .feature-name {
            font-weight: bold;
            font-size: 1.1em;
            color: #2c3e50;
            padding: 8px 0;
            background-color: #f8f9fa;
            border-left: 4px solid #007bff;
            padding-left: 20px;  /* Increased to show indentation under folder */
            margin-left: -12px;
            margin-right: -12px;
        }
        .test-name {
            font-size: 0.95em;
            color: #495057;
            margin-left: 40px;  /* Increased to show indentation under feature */
            display: block;
            padding: 8px 0 8px 15px;
            border-left: 2px solid #dee2e6;
            position: relative;
        }
        .test-name::before {
            content: "├─";
            position: absolute;
            left: -12px;
            color: #6c757d;
            font-weight: normal;
        }
        .test-name.last-test::before {
            content: "└─";
        }
        .test-status {
            font-size: 0.95em;
            padding: 8px 0;
            text-align: center;
        }
        .test-row {
            border-bottom: 1px solid #e9ecef;
        }
        .test-row:last-of-type {
            border-bottom: none;
        }
        .status-fail {
            background-color: #f8d7da;
            color: #721c24;
            padding: 4px 8px;
            border-radius: 4px;
            font-weight: bold;
        }
        .status-na {
            background-color: #e2e3e5;
            color: #6c757d;
            padding: 4px 8px;
            border-radius: 4px;
            font-weight: bold;
        }
        .feature-details {
            margin: 30px 0;
            padding: 20px;
            border: 1px solid #dee2e6;
            border-radius: 6px;
        }
        .feature-title {
            color: #2c3e50;
            margin-bottom: 15px;
        }
        .scenario-list {
            background: #f8f9fa;
            padding: 15px;
            border-radius: 4px;
            margin: 10px 0;
        }
        .implementation-list {
            list-style: none;
            padding: 0;
        }
        .implementation-list li {
            padding: 8px 0;
            border-bottom: 1px solid #dee2e6;
        }
        .implementation-list li:last-child {
            border-bottom: none;
        }
        .lang-badge {
            display: inline-block;
            padding: 4px 8px;
            border-radius: 4px;
            font-weight: bold;
            margin-right: 10px;
        }
        .lang-implemented { background-color: #d4edda; color: #155724; }
        .lang-failed { background-color: #f8d7da; color: #721c24; }
        .path-info {
            font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
            font-size: 0.9em;
            color: #6c757d;
            margin-left: 10px;
        }
        .missing-implementations {
            background: #fff3cd;
            border: 1px solid #ffeaa7;
            padding: 15px;
            border-radius: 6px;
            margin: 20px 0;
        }
        .missing-implementations h3 {
            color: #856404;
            margin-top: 0;
        }
        '''

    def _html_tag(self, tag: str, content: str = "", attributes: str = "", self_closing: bool = False) -> str:
        """Helper method to create HTML tags without manual spacing."""
        if self_closing:
            return f"<{tag}{' ' + attributes if attributes else ''} />"
        
        if content:
            return f"<{tag}{' ' + attributes if attributes else ''}>{content}</{tag}>"
        else:
            return f"<{tag}{' ' + attributes if attributes else ''}></{tag}>"
    
    def _indent_html(self, html_content: str, level: int = 1) -> str:
        """Add proper indentation to HTML content."""
        from textwrap import indent
        return indent(html_content, '    ' * level)

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
        header_cells = ['<th>Feature</th>'] + [f'<th>{lang}</th>' for lang in languages]
        
        # Group features by folder
        features_by_folder = {}
        for feature_name, feature_data in features.items():
            # Extract folder from path (e.g., "tests/e2e/auth/private_key_auth.feature" -> "auth")
            path_parts = Path(feature_data['path']).parts
            if len(path_parts) >= 3 and path_parts[-3] == 'e2e':
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
            folder_cell = f'<td colspan="{colspan}"><div class="folder-name">{folder_display}</div></td>'
            rows.append(f'<tr>{folder_cell}</tr>')
            
            for feature_name, feature_data in folder_features.items():
                formatted_name = self.format_feature_name(feature_name, feature_data['path'])
                scenarios = self.get_feature_scenarios(feature_data['path'])
                
                # Feature header row (indented under folder)
                feature_cells = [f'<td><div class="feature-name">{formatted_name}</div></td>']
                
                # Add status cells for each language at feature level
                for lang in languages:
                    if lang in feature_data['languages']:
                        lang_data = feature_data['languages'][lang]
                        # Feature level status: fail if ANY scenario fails, pass only if ALL scenarios pass
                        feature_has_any_failure = False
                        if 'scenarios' in lang_data:
                            for scenario_status in lang_data['scenarios'].values():
                                if not scenario_status:  # If any scenario failed
                                    feature_has_any_failure = True
                                    break
                        
                        if feature_has_any_failure:
                            feature_cells.append('<td><div class="test-status"><span class="status-fail">✗</span></div></td>')
                        else:
                            # Use overall status if no specific failures found
                            status = lang_data['status']
                            if status == '✅':
                                feature_cells.append('<td><div class="test-status"><span class="tick-icon">✓</span></div></td>')
                            else:
                                feature_cells.append('<td><div class="test-status"><span class="status-fail">✗</span></div></td>')
                    else:
                        feature_cells.append('<td><div class="test-status"><span class="status-na">-</span></div></td>')
                
                rows.append(f'<tr>{"".join(feature_cells)}</tr>')
                
                # Individual test rows
                for i, scenario in enumerate(scenarios):
                    is_last_test = i == len(scenarios) - 1
                    row_class = "test-row" if not is_last_test else "test-row"
                    
                    # Test name cell
                    test_class = "test-name last-test" if is_last_test else "test-name"
                    test_cell = f'<td><div class="{test_class}">• {scenario}</div></td>'
                    
                    # Status cells for each language
                    status_cells = []
                    for lang in languages:
                        if lang in feature_data['languages']:
                            lang_data = feature_data['languages'][lang]
                            # Check individual scenario status if available
                            if 'scenarios' in lang_data and scenario in lang_data['scenarios']:
                                scenario_passed = lang_data['scenarios'][scenario]
                                if scenario_passed:
                                    status_cells.append('<td><div class="test-status"><span class="tick-icon">✓</span></div></td>')
                                else:
                                    status_cells.append('<td><div class="test-status"><span class="status-fail">✗</span></div></td>')
                            else:
                                # Fallback to overall language status if scenario-level info not available
                                status = lang_data['status']
                                if status == '✅':
                                    status_cells.append('<td><div class="test-status"><span class="tick-icon">✓</span></div></td>')
                                else:
                                    status_cells.append('<td><div class="test-status"><span class="status-fail">✗</span></div></td>')
                        else:
                            status_cells.append('<td><div class="test-status"><span class="status-na">-</span></div></td>')
                    
                    rows.append(f'<tr class="{row_class}">{test_cell}{"".join(status_cells)}</tr>')
        
        rows_html = '\n'.join(f'                    {row}' for row in rows)
        
        return dedent(f"""
            <h2>📊 Coverage Overview</h2>
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
        """Generate the language coverage section HTML."""
        from textwrap import dedent
        
        # Calculate total test cases across all features
        total_test_cases_all = sum(len(self.get_feature_scenarios(f['path'])) for f in features.values())
        
        cards = []
        for lang in languages:
            implemented_count = 0
            
            for feature_name, feature_data in features.items():
                scenarios = self.get_feature_scenarios(feature_data['path'])
                if lang in feature_data['languages'] and feature_data['languages'][lang]['implemented']:
                    implemented_count += len(scenarios)
            
            lang_coverage = (implemented_count / total_test_cases_all) * 100 if total_test_cases_all > 0 else 0
            coverage_class = 'coverage-high' if lang_coverage >= 80 else 'coverage-medium' if lang_coverage >= 60 else 'coverage-low'
            
            cards.append(dedent(f"""
                <div class="summary-card">
                    <h3>{lang}</h3>
                    <div class="value {coverage_class}">{lang_coverage:.1f}%</div>
                    <div style="font-size: 0.9em; color: #6c757d;">{implemented_count}/{total_test_cases_all} test cases</div>
                </div>
            """).strip())
        
        cards_html = '\n            '.join(cards)
        
        return dedent(f"""
            <h2>📈 Language Coverage</h2>
            <div class="language-coverage">
                {cards_html}
            </div>
        """).strip()

    def _generate_detailed_breakdown_html(self, features: Dict) -> str:
        """Generate the detailed breakdown section HTML."""
        from textwrap import dedent
        
        feature_sections = []
        for feature_name, feature_data in features.items():
            formatted_name = self.format_feature_name(feature_name, feature_data['path'])
            
            # Build scenarios list
            scenarios = self.get_feature_scenarios(feature_data['path'])
            scenarios_html = ""
            if scenarios:
                scenario_items = '\n'.join(f'<li>{scenario}</li>' for scenario in scenarios)
                scenarios_html = dedent(f"""
                    <div class="scenario-list">
                        <strong>Scenarios:</strong>
                        <ul>
                            {scenario_items}
                        </ul>
                    </div>
                """).strip()
            
            # Build implementation status list
            impl_items = []
            for lang in sorted(feature_data['languages'].keys()):
                lang_data = feature_data['languages'][lang]
                badge_class = 'lang-implemented' if lang_data['implemented'] else 'lang-failed'
                status_text = 'IMPLEMENTED' if lang_data['implemented'] else 'FAILED/MISSING'
                
                # Extract line numbers for implemented tests
                line_info = ""
                if lang_data['implemented'] and 'path' in lang_data:
                    # Convert relative path to absolute path
                    file_path = self.workspace_root / lang_data['path']
                    if file_path.exists():
                        method_lines = self.extract_test_methods_with_lines(str(file_path), scenarios)
                        if method_lines:
                            line_numbers = [f"{scenario}: line {line}" for scenario, line in method_lines.items()]
                            if line_numbers:
                                line_info = f" ({', '.join(line_numbers)})"
                
                impl_items.append(dedent(f"""
                    <li>
                        <span class="lang-badge {badge_class}">{lang}</span>
                        {status_text}
                        <span class="path-info">{lang_data["path"]}{line_info}</span>
                    </li>
                """).strip())
            
            impl_html = '\n'.join(f'                {item}' for item in impl_items)
            
            feature_section = dedent(f"""
                <div class="feature-details">
                    <h3 class="feature-title">🔍 {formatted_name}</h3>
                    <p><strong>Path:</strong> <code>{feature_data["path"]}</code></p>
                    {scenarios_html}
                    <h4>Implementation Status:</h4>
                    <ul class="implementation-list">
{impl_html}
                    </ul>
                </div>
            """).strip()
            
            feature_sections.append(feature_section)
        
        sections_html = '\n        '.join(feature_sections)
        
        return dedent(f"""
            <h2>📋 Detailed Breakdown</h2>
            {sections_html}
        """).strip()

    def _generate_missing_implementations_html(self, features: Dict, languages: List[str]) -> str:
        """Generate the missing implementations section HTML."""
        from textwrap import dedent
        
        # Find missing implementations
        missing_implementations = []
        for feature_name, feature_data in features.items():
            for lang in languages:
                if lang not in feature_data['languages']:
                    missing_implementations.append((feature_name, lang))
                elif not feature_data['languages'][lang]['implemented']:
                    missing_implementations.append((feature_name, lang))
        
        if not missing_implementations:
            return ''
        
        # Build list items
        items = []
        for feature_name, lang in missing_implementations:
            feature_path = features.get(feature_name, {}).get('path', '')
            formatted_name = self.format_feature_name(feature_name, feature_path)
            items.append(f'<li><strong>{formatted_name}</strong> in <strong>{lang}</strong></li>')
        
        items_html = '\n            '.join(items)
        
        return dedent(f"""
            <div class="missing-implementations">
                <h3>⚠️ Missing Implementations</h3>
                <ul>
                    {items_html}
                </ul>
            </div>
        """).strip()

    def generate_html_report(self, features: Dict) -> str:
        """Generate an HTML coverage report."""
        from textwrap import dedent
        
        if not features:
            return "<html><body><h1>No features found</h1></body></html>"
        
        languages = self.get_all_languages(features)
        
        # Generate each section
        sections = [
            '<h1>Universal Driver Test Coverage Report</h1>',
            self._generate_coverage_table_html(features, languages),
            self._generate_language_coverage_html(features, languages),
            self._generate_detailed_breakdown_html(features)
        ]
        
        # Add missing implementations if any
        missing_impl_html = self._generate_missing_implementations_html(features, languages)
        if missing_impl_html:
            sections.append(missing_impl_html)
        
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
    features = generator.run_validator()
    
    if not features:
        print("No features found or validator failed to run.")
        sys.exit(1)
    
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
