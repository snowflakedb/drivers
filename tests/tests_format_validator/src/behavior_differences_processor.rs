use anyhow::Result;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::driver_handlers::{BaseDriverHandler, DriverHandlerFactory};
use crate::test_discovery::Language;
use crate::validator::{
    BehaviorDifferenceImplementation, BehaviorDifferenceInfo, BehaviorDifferencesReport,
};

#[derive(Debug, Clone)]
pub struct FeatureInfo {
    pub behavior_difference_scenarios: Vec<String>, // Only scenarios with behavior differences
}

pub struct BehaviorDifferencesProcessor {
    workspace_root: PathBuf,
}

impl BehaviorDifferencesProcessor {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub fn process_behavior_differences(
        &self,
        features: &HashMap<String, FeatureInfo>,
    ) -> Result<BehaviorDifferencesReport> {
        let mut behavior_difference_descriptions = HashMap::new();
        let mut behavior_differences_by_language = HashMap::new();

        // Behavior Differences processing started

        let factory = DriverHandlerFactory::new(self.workspace_root.clone());

        // Process each language that supports Behavior Differences
        for language in &[
            Language::Odbc,
            Language::Jdbc,
            Language::Python,
            Language::Rust,
        ] {
            let handler = factory.create_handler(language);

            if !handler.supports_behavior_differences() {
                continue;
            }

            // Parse Behavior Differences descriptions for this language
            let descriptions = handler
                .parse_behavior_differences_descriptions()
                .unwrap_or_default();

            // Extract Behavior Differences from test files
            if let Ok(mut behavior_differences) =
                self.extract_behavior_differences_from_test_files(language, features, &*handler)
            {
                // Populate descriptions for each Behavior Difference
                for behavior_difference in &mut behavior_differences {
                    if let Some(description) =
                        descriptions.get(&behavior_difference.behavior_difference_id)
                    {
                        behavior_difference.description = description.clone();
                    }
                }

                let language_key = format!("{:?}", language).to_lowercase();
                behavior_differences_by_language.insert(language_key, behavior_differences);

                // Also add to global descriptions for backward compatibility
                behavior_difference_descriptions.extend(descriptions);
            }
        }

        Ok(BehaviorDifferencesReport {
            behavior_difference_descriptions,
            behavior_differences_by_language,
        })
    }

    /// Extract Behavior Difference annotations from test files following Python logic exactly
    fn extract_behavior_differences_from_test_files(
        &self,
        _language: &Language,
        features: &HashMap<String, FeatureInfo>,
        handler: &dyn BaseDriverHandler,
    ) -> Result<Vec<BehaviorDifferenceInfo>> {
        let mut behavior_difference_test_mapping: HashMap<String, BehaviorDifferenceInfo> =
            HashMap::new();

        // Step 1: Get Behavior Difference scenario names from feature files to filter test methods
        let mut behavior_difference_scenarios = HashSet::new();
        for feature_info in features.values() {
            for scenario in &feature_info.behavior_difference_scenarios {
                behavior_difference_scenarios.insert(scenario.clone());
            }
        }

        let test_dir = handler.get_test_directory();
        if !test_dir.exists() {
            return Ok(vec![]);
        }

        // Step 2: Find all test files recursively
        let mut test_files = Vec::new();
        for ext in handler.get_test_file_extensions() {
            for entry in WalkDir::new(&test_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_file() && e.path().to_string_lossy().ends_with(&ext[1..]) // Remove * from *.cpp
                })
            {
                test_files.push(entry.path().to_path_buf());
            }
        }

        // Step 3: Process each test file - First pass: direct Behavior Difference annotations
        for test_file in &test_files {
            if let Ok(content) = fs::read_to_string(test_file) {
                let test_methods = handler.extract_test_methods(&content);

                // Look for direct Behavior Difference annotations in test methods
                for test_method in &test_methods {
                    // Check if this test method matches any Behavior Difference scenario
                    let matches_behavior_difference_scenario =
                        behavior_difference_scenarios.iter().any(|scenario| {
                            handler.method_matches_scenario(&test_method.name, scenario)
                        });

                    // Only process Behavior Differences for test methods that match Behavior Difference scenarios
                    if matches_behavior_difference_scenario {
                        if let Ok(method_behavior_differences) = handler
                            .find_behavior_differences_in_method(
                                &content,
                                &test_method.name,
                                test_file,
                            )
                        {
                            for (behavior_difference_id, behavior_difference_location) in
                                method_behavior_differences
                            {
                                let behavior_difference_info = behavior_difference_test_mapping
                                    .entry(behavior_difference_id.clone())
                                    .or_insert_with(|| BehaviorDifferenceInfo {
                                        behavior_difference_id: behavior_difference_id.clone(),
                                        description: String::new(),
                                        implementations: Vec::new(),
                                    });

                                behavior_difference_info.implementations.push(
                                    BehaviorDifferenceImplementation {
                                        test_method: test_method.name.clone(),
                                        test_file: test_file
                                            .strip_prefix(&self.workspace_root)
                                            .unwrap_or(test_file)
                                            .to_string_lossy()
                                            .to_string(),
                                        test_line: test_method.line,
                                        new_behaviour_file: behavior_difference_location
                                            .new_behaviour_file,
                                        new_behaviour_line: behavior_difference_location
                                            .new_behaviour_line,
                                        old_behaviour_file: behavior_difference_location
                                            .old_behaviour_file,
                                        old_behaviour_line: behavior_difference_location
                                            .old_behaviour_line,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        // Step 4: Second pass - Process test methods that match Behavior Difference scenarios but don't have direct Behavior Differences
        self.find_behavior_differences_in_scenario_test_methods(
            &mut behavior_difference_test_mapping,
            &behavior_difference_scenarios,
            handler,
            &test_files,
        )?;

        // Step 5: Third pass - Look for Behavior Difference assertions in cross-file helper methods
        self.find_cross_file_behavior_difference_assertions(
            &mut behavior_difference_test_mapping,
            handler,
            &test_files,
        )?;

        Ok(behavior_difference_test_mapping.into_values().collect())
    }

    /// Find Behavior Differences in helper methods called by test methods that match Behavior Difference scenarios
    fn find_behavior_differences_in_scenario_test_methods(
        &self,
        behavior_difference_test_mapping: &mut HashMap<String, BehaviorDifferenceInfo>,
        behavior_difference_scenarios: &HashSet<String>,
        handler: &dyn BaseDriverHandler,
        test_files: &[PathBuf],
    ) -> Result<()> {
        for test_file in test_files {
            if let Ok(content) = fs::read_to_string(test_file) {
                let test_methods = handler.extract_test_methods(&content);

                // Check each test method to see if it matches a Breaking Change scenario
                for test_method in &test_methods {
                    // Check if this test method matches any Behavior Difference scenario
                    let matches_behavior_difference_scenario =
                        behavior_difference_scenarios.iter().any(|scenario| {
                            handler.method_matches_scenario(&test_method.name, scenario)
                        });

                    if matches_behavior_difference_scenario {
                        // Extract helper methods called by this test method
                        let helper_methods =
                            handler.extract_helper_method_calls(&content, &test_method.name);

                        // Search for Behavior Difference assertions in the called helper methods
                        let additional_behavior_differences = self
                            .find_all_behavior_differences_in_helper_methods(
                                &content,
                                &helper_methods,
                                test_file,
                                handler,
                            )?;

                        // Add any Behavior Differences found to the test's Behavior Difference mapping
                        for (behavior_difference_id, behavior_difference_location) in
                            additional_behavior_differences
                        {
                            let behavior_difference_info = behavior_difference_test_mapping
                                .entry(behavior_difference_id.clone())
                                .or_insert_with(|| BehaviorDifferenceInfo {
                                    behavior_difference_id: behavior_difference_id.clone(),
                                    description: String::new(),
                                    implementations: Vec::new(),
                                });

                            // Check if this test is already in the mapping for this Behavior Difference
                            let already_exists = behavior_difference_info
                                .implementations
                                .iter()
                                .any(|impl_| {
                                    impl_.test_method == test_method.name
                                        && impl_.test_file
                                            == test_file
                                                .strip_prefix(&self.workspace_root)
                                                .unwrap_or(test_file)
                                                .to_string_lossy()
                                });

                            if !already_exists {
                                behavior_difference_info.implementations.push(
                                    BehaviorDifferenceImplementation {
                                        test_method: test_method.name.clone(),
                                        test_file: test_file
                                            .strip_prefix(&self.workspace_root)
                                            .unwrap_or(test_file)
                                            .to_string_lossy()
                                            .to_string(),
                                        test_line: test_method.line,
                                        new_behaviour_file: behavior_difference_location
                                            .new_behaviour_file,
                                        new_behaviour_line: behavior_difference_location
                                            .new_behaviour_line,
                                        old_behaviour_file: behavior_difference_location
                                            .old_behaviour_file,
                                        old_behaviour_line: behavior_difference_location
                                            .old_behaviour_line,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Find all Behavior Differences in helper methods (including cross-file and nested calls)
    fn find_all_behavior_differences_in_helper_methods(
        &self,
        content: &str,
        helper_methods: &[String],
        test_file: &Path,
        handler: &dyn BaseDriverHandler,
    ) -> Result<HashMap<String, crate::driver_handlers::base_handler::BehaviorDifferenceLocation>>
    {
        let mut all_behavior_differences = HashMap::new();
        let mut processed_methods = HashSet::new();

        // Recursively process helper methods to find nested calls
        self.find_behavior_differences_in_helper_methods_recursive(
            content,
            helper_methods,
            test_file,
            handler,
            &mut all_behavior_differences,
            &mut processed_methods,
        )?;

        // Also check cross-file helper methods for direct calls
        self.find_behavior_differences_in_cross_file_methods(
            helper_methods,
            handler,
            &mut all_behavior_differences,
            &mut processed_methods,
        )?;

        Ok(all_behavior_differences)
    }

    /// Recursively find Behavior Differences in helper methods and their nested calls
    fn find_behavior_differences_in_helper_methods_recursive(
        &self,
        content: &str,
        helper_methods: &[String],
        test_file: &Path,
        handler: &dyn BaseDriverHandler,
        all_behavior_differences: &mut HashMap<
            String,
            crate::driver_handlers::base_handler::BehaviorDifferenceLocation,
        >,
        processed_methods: &mut HashSet<String>,
    ) -> Result<()> {
        // First, look for Behavior Differences in helper methods within the same file
        for helper_method in helper_methods {
            if processed_methods.contains(helper_method) {
                continue; // Avoid infinite recursion
            }
            processed_methods.insert(helper_method.clone());

            // Find Breaking Changes directly in this helper method (class methods)
            if let Ok(method_behavior_differences) =
                handler.find_behavior_differences_in_method(content, helper_method, test_file)
            {
                all_behavior_differences.extend(method_behavior_differences);
            }

            // Also check for Breaking Changes in standalone functions (for Python)
            if let Ok(function_behavior_differences) =
                handler.find_behavior_differences_in_function(content, helper_method, test_file)
            {
                all_behavior_differences.extend(function_behavior_differences);
            }

            // Find nested helper method calls within this helper method
            let nested_helper_methods = handler.extract_helper_method_calls(content, helper_method);
            if !nested_helper_methods.is_empty() {
                // Recursively process nested helper methods
                self.find_behavior_differences_in_helper_methods_recursive(
                    content,
                    &nested_helper_methods,
                    test_file,
                    handler,
                    all_behavior_differences,
                    processed_methods,
                )?;
                // Also check cross-file for nested helper methods
                self.find_behavior_differences_in_cross_file_methods(
                    &nested_helper_methods,
                    handler,
                    all_behavior_differences,
                    processed_methods,
                )?;
            }
        }

        // Then, look for Breaking Changes in cross-file helper methods (e.g., common library)
        if handler
            .get_test_file_extensions()
            .contains(&"*.py".to_string())
        {
            // For Python, follow imports to find helper functions
            self.find_python_imported_helpers(
                content,
                helper_methods,
                test_file,
                handler,
                all_behavior_differences,
                processed_methods,
            )?;
        } else {
            self.find_odbc_included_helpers(
                content,
                helper_methods,
                test_file,
                handler,
                all_behavior_differences,
                processed_methods,
            )?;
        }

        Ok(())
    }

    /// Find Breaking Changes in cross-file helper methods (e.g., common library)
    fn find_behavior_differences_in_cross_file_methods(
        &self,
        helper_methods: &[String],
        handler: &dyn BaseDriverHandler,
        all_behavior_differences: &mut HashMap<
            String,
            crate::driver_handlers::base_handler::BehaviorDifferenceLocation,
        >,
        processed_methods: &mut HashSet<String>,
    ) -> Result<()> {
        if handler
            .get_test_file_extensions()
            .contains(&"*.py".to_string())
        {
            Ok(())
        } else {
            self.find_odbc_cross_file_methods(
                helper_methods,
                handler,
                all_behavior_differences,
                processed_methods,
            )
        }
    }

    fn find_odbc_cross_file_methods(
        &self,
        helper_methods: &[String],
        handler: &dyn BaseDriverHandler,
        all_behavior_differences: &mut HashMap<
            String,
            crate::driver_handlers::base_handler::BehaviorDifferenceLocation,
        >,
        processed_methods: &mut HashSet<String>,
    ) -> Result<()> {
        let common_dir = self
            .workspace_root
            .join("odbc_tests")
            .join("common")
            .join("src");
        if common_dir.exists() {
            // Use separate tracking for cross-file methods to avoid skipping methods that don't exist in main file
            let mut cross_file_processed = HashSet::new();

            for helper_method in helper_methods {
                if cross_file_processed.contains(helper_method) {
                    continue; // Already processed in cross-file context
                }
                // Check if this helper method exists in common library
                for entry in WalkDir::new(&common_dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().is_file()
                            && e.path()
                                .extension()
                                .map_or(false, |ext| ext == "cpp" || ext == "c")
                    })
                {
                    if let Ok(common_content) = fs::read_to_string(entry.path()) {
                        // Check if the helper method is defined in this file
                        if self.method_exists_in_content(&common_content, helper_method) {
                            cross_file_processed.insert(helper_method.clone());
                            processed_methods.insert(helper_method.clone()); // Also mark in main processed set

                            // Find Breaking Changes in this cross-file method
                            if let Ok(method_behavior_differences) = handler
                                .find_behavior_differences_in_method(
                                    &common_content,
                                    helper_method,
                                    entry.path(),
                                )
                            {
                                all_behavior_differences.extend(method_behavior_differences);
                            }

                            // Also check for nested calls within this cross-file method
                            let nested_helper_methods =
                                handler.extract_helper_method_calls(&common_content, helper_method);
                            if !nested_helper_methods.is_empty() {
                                // Recursively process nested helper methods in cross-file context
                                self.find_behavior_differences_in_cross_file_methods(
                                    &nested_helper_methods,
                                    handler,
                                    all_behavior_differences,
                                    processed_methods,
                                )?;
                            }

                            break; // Found the method, no need to check other files
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Find Breaking Changes in Python helper functions by following imports
    fn find_python_imported_helpers(
        &self,
        test_file_content: &str,
        helper_methods: &[String],
        test_file: &Path,
        handler: &dyn BaseDriverHandler,
        all_behavior_differences: &mut HashMap<
            String,
            crate::driver_handlers::base_handler::BehaviorDifferenceLocation,
        >,
        processed_methods: &mut HashSet<String>,
    ) -> Result<()> {
        // Parse import statements from the test file to find where helper functions come from
        let import_map = self.parse_python_imports(test_file_content, test_file)?;

        // Use separate tracking for cross-file methods
        let mut cross_file_processed = HashSet::new();

        for helper_method in helper_methods {
            if cross_file_processed.contains(helper_method) {
                continue;
            }

            // Check if this helper method is imported from another file
            if let Some(helper_file_path) = import_map.get(helper_method) {
                if helper_file_path.exists() {
                    if let Ok(helper_content) = fs::read_to_string(helper_file_path) {
                        // Check if the helper method is defined in this file
                        if helper_content.contains(&format!("def {}(", helper_method)) {
                            cross_file_processed.insert(helper_method.clone());
                            processed_methods.insert(helper_method.clone());

                            // Find Breaking Changes in this imported helper function
                            if let Ok(method_behavior_differences) = handler
                                .find_behavior_differences_in_function(
                                    &helper_content,
                                    helper_method,
                                    helper_file_path,
                                )
                            {
                                all_behavior_differences.extend(method_behavior_differences);
                            }

                            // Also check for nested calls within this helper function
                            let nested_helper_methods =
                                handler.extract_helper_method_calls(&helper_content, helper_method);
                            if !nested_helper_methods.is_empty() {
                                // Recursively process nested helper methods
                                self.find_python_imported_helpers(
                                    &helper_content,
                                    &nested_helper_methods,
                                    helper_file_path,
                                    handler,
                                    all_behavior_differences,
                                    processed_methods,
                                )?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse Python import statements and return a map of function name -> file path
    fn parse_python_imports(
        &self,
        content: &str,
        test_file: &Path,
    ) -> Result<HashMap<String, PathBuf>> {
        let mut import_map = HashMap::new();

        if let Some(test_dir) = test_file.parent() {
            for line in content.lines() {
                let trimmed = line.trim();

                // Handle "from .module import function1, function2" patterns
                if let Some(captures) = Regex::new(r"from\s+(\.[\w.]*)\s+import\s+(.+)")
                    .unwrap()
                    .captures(trimmed)
                {
                    if let (Some(module_path), Some(imports)) = (captures.get(1), captures.get(2)) {
                        let module_str = module_path.as_str();

                        // Convert relative import to file path
                        let helper_file = if module_str.starts_with('.') {
                            // Relative import like ".auth_helpers"
                            let module_name = module_str.trim_start_matches('.');
                            if module_name.is_empty() {
                                continue; // Skip malformed imports
                            }
                            test_dir.join(format!("{}.py", module_name))
                        } else {
                            continue; // Skip absolute imports for now
                        };

                        // Parse imported function names
                        for func in imports.as_str().split(',') {
                            let func_name = func.trim();
                            if !func_name.is_empty() && helper_file.exists() {
                                import_map.insert(func_name.to_string(), helper_file.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(import_map)
    }

    /// Find Breaking Changes in ODBC helper functions by following includes (same pattern as Python imports)
    fn find_odbc_included_helpers(
        &self,
        test_file_content: &str,
        helper_methods: &[String],
        test_file: &Path,
        handler: &dyn BaseDriverHandler,
        all_behavior_differences: &mut HashMap<
            String,
            crate::driver_handlers::base_handler::BehaviorDifferenceLocation,
        >,
        processed_methods: &mut HashSet<String>,
    ) -> Result<()> {
        // Parse include statements from the test file to find where helper functions come from
        let include_map = self.parse_odbc_includes(test_file_content, test_file)?;

        // Use separate tracking for cross-file methods
        let mut cross_file_processed = HashSet::new();

        for helper_method in helper_methods {
            if cross_file_processed.contains(helper_method) {
                continue;
            }

            // Check if this helper method is included from another file
            if let Some(helper_file_path) = include_map.get(helper_method) {
                if helper_file_path.exists() {
                    if let Ok(helper_content) = fs::read_to_string(helper_file_path) {
                        // Check if the helper method is defined in this file
                        if self.method_exists_in_content(&helper_content, helper_method) {
                            cross_file_processed.insert(helper_method.clone());
                            processed_methods.insert(helper_method.clone());

                            // Find Breaking Changes in this included helper function
                            if let Ok(method_behavior_differences) = handler
                                .find_behavior_differences_in_method(
                                    &helper_content,
                                    helper_method,
                                    helper_file_path,
                                )
                            {
                                all_behavior_differences.extend(method_behavior_differences);
                            }

                            // Also check for nested calls within this helper function
                            let nested_helper_methods =
                                handler.extract_helper_method_calls(&helper_content, helper_method);
                            if !nested_helper_methods.is_empty() {
                                // Recursively process nested helper methods
                                self.find_odbc_included_helpers(
                                    &helper_content,
                                    &nested_helper_methods,
                                    helper_file_path,
                                    handler,
                                    all_behavior_differences,
                                    processed_methods,
                                )?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse ODBC include statements and return a map of function name -> file path
    fn parse_odbc_includes(
        &self,
        content: &str,
        _test_file: &Path,
    ) -> Result<HashMap<String, PathBuf>> {
        let mut include_map = HashMap::new();

        // ODBC common directories
        let common_include_dir = self
            .workspace_root
            .join("odbc_tests")
            .join("common")
            .join("include");
        let common_src_dir = self
            .workspace_root
            .join("odbc_tests")
            .join("common")
            .join("src");

        for line in content.lines() {
            let trimmed = line.trim();

            // Handle #include "header.hpp" patterns
            if let Some(captures) = Regex::new(r#"#include\s+"([^"]+\.hpp?)""#)
                .unwrap()
                .captures(trimmed)
            {
                if let Some(header_name) = captures.get(1) {
                    let header_file = header_name.as_str();

                    // Look for the header in common/include
                    let header_path = common_include_dir.join(header_file);
                    if header_path.exists() {
                        // Find corresponding .cpp file in common/src
                        let cpp_name = header_file.replace(".hpp", ".cpp").replace(".h", ".cpp");
                        let cpp_path = common_src_dir.join(cpp_name);

                        if cpp_path.exists() {
                            // Read the header to find function declarations
                            if let Ok(header_content) = fs::read_to_string(&header_path) {
                                // Extract function names from header declarations
                                // Look for function declarations like: void functionName( or std::vector<Type> functionName(
                                let function_regex = Regex::new(r"(?:void|int|bool|std::\w+(?:<[^>]+>)?|SQLRETURN|[\w:]+(?:<[^>]+>)?)\s+(\w+)\s*\(").unwrap();
                                for func_captures in function_regex.captures_iter(&header_content) {
                                    if let Some(function_name) = func_captures.get(1) {
                                        let func_name = function_name.as_str();
                                        // Map function name to the .cpp implementation file
                                        include_map.insert(func_name.to_string(), cpp_path.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(include_map)
    }

    /// Check if a method exists in the given content
    fn method_exists_in_content(&self, content: &str, method_name: &str) -> bool {
        content.contains(&format!("void {method_name}("))
            || content.contains(&format!("{method_name}("))
            || content.contains(&format!("void {method_name}()"))
            || content.contains(&format!("{method_name}()"))
    }

    /// Look for Breaking Change assertions that might be in other files for existing Breaking Changes
    fn find_cross_file_behavior_difference_assertions(
        &self,
        _behavior_difference_test_mapping: &mut HashMap<String, BehaviorDifferenceInfo>,
        _handler: &dyn BaseDriverHandler,
        _test_files: &[PathBuf],
    ) -> Result<()> {
        // This method can be implemented later for more complex cross-file scenarios
        // For now, the logic in find_all_behavior_differences_in_helper_methods handles the main cases
        Ok(())
    }
}
