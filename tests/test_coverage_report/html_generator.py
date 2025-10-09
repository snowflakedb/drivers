#!/usr/bin/env python3
"""
HTML Generator Module

Handles generation of HTML reports including coverage tables,
detailed breakdowns, and Behavior Difference tabs.
"""

from pathlib import Path
from typing import Dict, List
from collections import defaultdict
from textwrap import dedent


class HTMLGenerator:
    """Generates HTML reports from coverage and Behavior Difference data."""
    
    def __init__(self, workspace_root: Path):
        self.workspace_root = workspace_root
    
    def generate_html_report(self, features: Dict, breaking_change_handler, feature_parser) -> str:
        """Generate a complete HTML coverage report with tabbed interface."""
        if not features:
            return "<html><body><h1>No features found</h1></body></html>"
        
        languages = self._get_all_languages(features)
        
        # Generate content for each tab
        overview_content = self._generate_coverage_table_html(features, languages, breaking_change_handler, feature_parser) + '\n\n' + self._generate_language_coverage_html(features, languages)
        detailed_content = self._generate_detailed_breakdown_html(features, feature_parser)
        missing_impl_html = self._generate_missing_implementations_html(features, languages, feature_parser)
        breaking_change_content = self._generate_breaking_change_tab_html(features, breaking_change_handler)
        
        # Generate complete HTML document
        return self._create_html_document(
            "Test Coverage Report",
            self._generate_tabbed_interface(overview_content, detailed_content, missing_impl_html, breaking_change_content)
        )
    
    def generate_coverage_table(self, features: Dict) -> str:
        """Generate a text-based coverage table showing implementation status."""
        if not features:
            return "No features found.\n"
        
        languages = self._get_all_languages(features)
        
        # Group features by folder
        features_by_folder = {}
        for feature_name, feature_data in features.items():
            # Extract folder from path
            path_parts = Path(feature_data['path']).parts
            if len(path_parts) > 3:  # tests/e2e/folder/feature.feature
                folder = path_parts[2]
            else:
                folder = "root"
            
            if folder not in features_by_folder:
                features_by_folder[folder] = {}
            features_by_folder[folder][feature_name] = feature_data
        
        # Calculate column widths
        feature_width = max(
            max(len(self._format_feature_name_simple(name)) for name in features.keys()),
            20  # minimum width
        ) + 4  # padding
        lang_width = 15  # Fixed width for language columns
        
        # Build table
        table = []
        
        # Header
        header = f"{'Feature':<{feature_width}}"
        for lang in languages:
            header += f" | {lang.title():<{lang_width}}"
        table.append(header)
        table.append("-" * len(header))
        
        # Content by folder
        for folder, folder_features in sorted(features_by_folder.items()):
            # Folder header
            table.append(f"\n{folder.title()}:")
            
            # Features in this folder
            for feature_name, feature_data in folder_features.items():
                formatted_name = self._format_feature_name_simple(feature_name)
                row = f"  {formatted_name:<{feature_width-2}}"  # Indent feature under folder
                for lang in languages:
                    lang_key = self._get_language_data_key(lang)
                    if lang_key in feature_data['languages']:
                        status = feature_data['languages'][lang_key]['status']
                        status_text = "PASS" if status == '✅' else "FAIL"
                    else:
                        status_text = "N/A"
                    row += f" | {status_text:<{lang_width}}"
                table.append(row)
        
        return '\n'.join(table)
    
    def _get_all_languages(self, features: Dict) -> List[str]:
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
    
    def _get_language_data_key(self, normalized_lang: str) -> str:
        """Convert normalized language name back to the key used in feature data structure."""
        if normalized_lang == 'core':
            return 'Rust'  # Core maps back to Rust in the data structure
        elif normalized_lang == 'odbc':
            return 'Odbc'  # odbc maps back to Odbc in the data structure
        elif normalized_lang == 'python':
            return 'Python'  # python maps back to Python in the data structure
        else:
            return normalized_lang.capitalize()  # Default: capitalize first letter
    
    def _format_feature_name_simple(self, feature_name: str) -> str:
        """Simple feature name formatting for text table."""
        return feature_name.replace('_', ' ').title()
    
    def _get_html_styles(self) -> str:
        """Get CSS styles for the HTML report."""
        # Read CSS from external file
        css_file = Path(__file__).parent / 'styles.css'
        try:
            with open(css_file, 'r', encoding='utf-8') as f:
                return f.read()
        except FileNotFoundError:
            # Fallback to basic inline styles if CSS file is missing
            return '''
            body { font-family: Arial, sans-serif; margin: 20px; }
            table { width: 100%; border-collapse: collapse; }
            th, td { padding: 8px; border: 1px solid #ddd; text-align: left; }
            th { background-color: #f2f2f2; }
            .tick-icon { color: green; font-weight: bold; }
            .cross-icon { color: red; font-weight: bold; }
            .status-na { color: #666; }
            '''
    
    def _create_html_document(self, title: str, body_content: str) -> str:
        """Create a complete HTML document with head and body."""
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
                    <h1>{title}</h1>
                    {body_content}
                </div>
            </body>
            </html>
        """).strip()
    
    def _generate_tabbed_interface(self, overview_content: str, detailed_content: str, missing_content: str, breaking_change_content: str) -> str:
        """Generate the tabbed interface HTML."""
        return dedent(f"""
            <div class="tabs">
                <div class="tab-buttons">
                    <button class="tab-button" onclick="showTab('overview-tab')">📊 Overview</button>
                    <button class="tab-button" onclick="showTab('breaking_change-tab')">📋 Behavior Differences</button>
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
                    {missing_content if missing_content else '<p>No missing implementations found! ✅</p>'}
                </div>
            </div>
            
            <hr>
            <p style="text-align: center; color: #6c757d; margin-top: 30px;">Generated by Test Coverage Report Generator</p>
            
            <script>
                function showTab(tabId) {{
                    // Hide all tab contents
                    const tabs = document.querySelectorAll('.tab-content');
                    tabs.forEach(tab => tab.style.display = 'none');
                    
                    // Remove active class from all buttons
                    const buttons = document.querySelectorAll('.tab-button');
                    buttons.forEach(button => button.classList.remove('active'));
                    
                    // Show selected tab and mark button as active
                    document.getElementById(tabId).style.display = 'block';
                    event.target.classList.add('active');
                }}
                
                // Show first tab by default
                document.addEventListener('DOMContentLoaded', function() {{
                    showTab('overview-tab');
                    document.querySelector('.tab-button').classList.add('active');
                }});
                
                // Behavior Difference popup functionality
                function toggleBreaking ChangePopup(popupId) {{
                    const popup = document.getElementById(popupId);
                    popup.style.display = popup.style.display === 'block' ? 'none' : 'block';
                }}
                
                function hideBreaking ChangePopup(popupId) {{
                    document.getElementById(popupId).style.display = 'none';
                }}
                
                // Behavior Difference expansion functionality
                function expandToBreaking Change(breaking_changeId) {{
                    const breaking_changeElement = document.getElementById(breaking_changeId);
                    if (breaking_changeElement) {{
                        // Collapse all other Behavior Difference sections
                        const allBreakingChangeSections = document.querySelectorAll('.breaking_change-section');
                        allBreakingChangeSections.forEach(section => {{
                            if (section.id !== breaking_changeId) {{
                                section.classList.remove('expanded');
                            }}
                        }});
                        
                        // Expand the target Behavior Difference section
                        breaking_changeElement.classList.add('expanded');
                        setTimeout(() => breaking_changeElement.scrollIntoView({{behavior: 'smooth'}}), 100);
                    }}
                }}
                
                // Feature expansion functionality
                function expandToFeature(featureId) {{
                    const featureElement = document.getElementById(featureId);
                    if (featureElement) {{
                        featureElement.classList.add('expanded');
                        setTimeout(() => featureElement.scrollIntoView({{behavior: 'smooth'}}), 100);
                    }}
                }}
            </script>
        """)
    
    # Placeholder methods - these would need to be implemented with the actual logic
    # from the original coverage_report.py file
    
    def _generate_coverage_table_html(self, features: Dict, languages: List[str], breaking_change_handler, feature_parser) -> str:
        """Generate the coverage table HTML (placeholder - needs implementation)."""
        return "<p>Coverage table HTML generation not yet implemented</p>"
    
    def _generate_language_coverage_html(self, features: Dict, languages: List[str]) -> str:
        """Generate the language coverage HTML (placeholder - needs implementation)."""
        return "<p>Language coverage HTML generation not yet implemented</p>"
    
    def _generate_detailed_breakdown_html(self, features: Dict, feature_parser) -> str:
        """Generate the detailed breakdown HTML (placeholder - needs implementation)."""
        return "<p>Detailed breakdown HTML generation not yet implemented</p>"
    
    def _generate_missing_implementations_html(self, features: Dict, languages: List[str], feature_parser) -> str:
        """Generate the missing implementations HTML (placeholder - needs implementation)."""
        return "<p>Missing implementations HTML generation not yet implemented</p>"
    
    def _generate_breaking_change_tab_html(self, features: Dict, breaking_change_handler) -> str:
        """Generate the Behavior Difference tab HTML (placeholder - needs implementation)."""
        return "<p>Behavior Difference tab HTML generation not yet implemented</p>"
