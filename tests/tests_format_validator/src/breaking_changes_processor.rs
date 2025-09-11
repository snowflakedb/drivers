use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::driver_handlers::{BaseDriverHandler, DriverHandlerFactory};
use crate::test_discovery::Language;
use crate::validator::{BreakingChangeImplementation, BreakingChangeInfo, BreakingChangesReport};

#[derive(Debug, Clone)]
pub struct FeatureInfo {
    pub breaking_change_scenarios: Vec<String>, // Only scenarios with breaking changes
}

pub struct BreakingChangesProcessor {
    workspace_root: PathBuf,
}

impl BreakingChangesProcessor {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub fn process_breaking_changes(
        &self,
        features: &HashMap<String, FeatureInfo>,
    ) -> Result<BreakingChangesReport> {
        let mut breaking_change_descriptions = HashMap::new();
        let mut breaking_changes_by_language = HashMap::new();

        // Breaking Changes processing started

        let factory = DriverHandlerFactory::new(self.workspace_root.clone());

        // Process each language that supports Breaking Changes
        for language in &[
            Language::Odbc,
            Language::Jdbc,
            Language::Python,
            Language::Rust,
        ] {
            let handler = factory.create_handler(language);

            if !handler.supports_breaking_changes() {
                continue;
            }

            // Parse Breaking Changes descriptions for this language
            let descriptions = handler
                .parse_breaking_changes_descriptions()
                .unwrap_or_default();

            // Extract Breaking Changes from test files
            if let Ok(mut breaking_changes) =
                self.extract_breaking_changes_from_test_files(language, features, &*handler)
            {
                // Populate descriptions for each Breaking Change
                for breaking_change in &mut breaking_changes {
                    if let Some(description) = descriptions.get(&breaking_change.breaking_change_id)
                    {
                        breaking_change.description = description.clone();
                    }
                }

                let language_key = format!("{:?}", language).to_lowercase();
                breaking_changes_by_language.insert(language_key, breaking_changes);

                // Also add to global descriptions for backward compatibility
                breaking_change_descriptions.extend(descriptions);
            }
        }

        Ok(BreakingChangesReport {
            breaking_change_descriptions,
            breaking_changes_by_language,
        })
    }

    /// Extract Breaking Change annotations from test files following Python logic exactly
    fn extract_breaking_changes_from_test_files(
        &self,
        _language: &Language,
        features: &HashMap<String, FeatureInfo>,
        handler: &dyn BaseDriverHandler,
    ) -> Result<Vec<BreakingChangeInfo>> {
        let mut breaking_change_test_mapping: HashMap<String, BreakingChangeInfo> = HashMap::new();

        // Step 1: Get Breaking Change scenario names from feature files to filter test methods
        let mut breaking_change_scenarios = HashSet::new();
        for feature_info in features.values() {
            for scenario in &feature_info.breaking_change_scenarios {
                breaking_change_scenarios.insert(scenario.clone());
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

        // Step 3: Process each test file - First pass: direct Breaking Change annotations
        for test_file in &test_files {
            if let Ok(content) = fs::read_to_string(test_file) {
                let test_methods = handler.extract_test_methods(&content);

                // Look for direct Breaking Change annotations in test methods
                for test_method in &test_methods {
                    // Check if this test method matches any Breaking Change scenario
                    let matches_breaking_change_scenario =
                        breaking_change_scenarios.iter().any(|scenario| {
                            handler.method_matches_scenario(&test_method.name, scenario)
                        });

                    // Only process Breaking Changes for test methods that match Breaking Change scenarios
                    if matches_breaking_change_scenario {
                        if let Ok(method_breaking_changes) = handler
                            .find_breaking_changes_in_method(&content, &test_method.name, test_file)
                        {
                            for (breaking_change_id, breaking_change_location) in
                                method_breaking_changes
                            {
                                let breaking_change_info = breaking_change_test_mapping
                                    .entry(breaking_change_id.clone())
                                    .or_insert_with(|| BreakingChangeInfo {
                                        breaking_change_id: breaking_change_id.clone(),
                                        description: String::new(),
                                        implementations: Vec::new(),
                                    });

                                breaking_change_info.implementations.push(
                                    BreakingChangeImplementation {
                                        test_method: test_method.name.clone(),
                                        test_file: test_file
                                            .strip_prefix(&self.workspace_root)
                                            .unwrap_or(test_file)
                                            .to_string_lossy()
                                            .to_string(),
                                        test_line: test_method.line,
                                        new_behaviour_file: breaking_change_location
                                            .new_behaviour_file,
                                        new_behaviour_line: breaking_change_location
                                            .new_behaviour_line,
                                        old_behaviour_file: breaking_change_location
                                            .old_behaviour_file,
                                        old_behaviour_line: breaking_change_location
                                            .old_behaviour_line,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        // Step 4: Second pass - Process test methods that match Breaking Change scenarios but don't have direct Breaking Changes
        self.find_breaking_changes_in_scenario_test_methods(
            &mut breaking_change_test_mapping,
            &breaking_change_scenarios,
            handler,
            &test_files,
        )?;

        // Step 5: Third pass - Look for Breaking Change assertions in cross-file helper methods
        self.find_cross_file_breaking_change_assertions(
            &mut breaking_change_test_mapping,
            handler,
            &test_files,
        )?;

        Ok(breaking_change_test_mapping.into_values().collect())
    }

    /// Find Breaking Changes in helper methods called by test methods that match Breaking Change scenarios
    fn find_breaking_changes_in_scenario_test_methods(
        &self,
        breaking_change_test_mapping: &mut HashMap<String, BreakingChangeInfo>,
        breaking_change_scenarios: &HashSet<String>,
        handler: &dyn BaseDriverHandler,
        test_files: &[PathBuf],
    ) -> Result<()> {
        for test_file in test_files {
            if let Ok(content) = fs::read_to_string(test_file) {
                let test_methods = handler.extract_test_methods(&content);

                // Check each test method to see if it matches a Breaking Change scenario
                for test_method in &test_methods {
                    // Check if this test method matches any Breaking Change scenario
                    let matches_breaking_change_scenario =
                        breaking_change_scenarios.iter().any(|scenario| {
                            handler.method_matches_scenario(&test_method.name, scenario)
                        });

                    if matches_breaking_change_scenario {
                        // Extract helper methods called by this test method
                        let helper_methods =
                            handler.extract_helper_method_calls(&content, &test_method.name);

                        // Search for Breaking Change assertions in the called helper methods
                        let additional_breaking_changes = self
                            .find_all_breaking_changes_in_helper_methods(
                                &content,
                                &helper_methods,
                                test_file,
                                handler,
                            )?;

                        // Add any Breaking Changes found to the test's Breaking Change mapping
                        for (breaking_change_id, breaking_change_location) in
                            additional_breaking_changes
                        {
                            let breaking_change_info = breaking_change_test_mapping
                                .entry(breaking_change_id.clone())
                                .or_insert_with(|| BreakingChangeInfo {
                                    breaking_change_id: breaking_change_id.clone(),
                                    description: String::new(),
                                    implementations: Vec::new(),
                                });

                            // Check if this test is already in the mapping for this Breaking Change
                            let already_exists =
                                breaking_change_info.implementations.iter().any(|impl_| {
                                    impl_.test_method == test_method.name
                                        && impl_.test_file
                                            == test_file
                                                .strip_prefix(&self.workspace_root)
                                                .unwrap_or(test_file)
                                                .to_string_lossy()
                                });

                            if !already_exists {
                                breaking_change_info.implementations.push(
                                    BreakingChangeImplementation {
                                        test_method: test_method.name.clone(),
                                        test_file: test_file
                                            .strip_prefix(&self.workspace_root)
                                            .unwrap_or(test_file)
                                            .to_string_lossy()
                                            .to_string(),
                                        test_line: test_method.line,
                                        new_behaviour_file: breaking_change_location
                                            .new_behaviour_file,
                                        new_behaviour_line: breaking_change_location
                                            .new_behaviour_line,
                                        old_behaviour_file: breaking_change_location
                                            .old_behaviour_file,
                                        old_behaviour_line: breaking_change_location
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

    /// Find all Breaking Changes in helper methods (including cross-file and nested calls)
    fn find_all_breaking_changes_in_helper_methods(
        &self,
        content: &str,
        helper_methods: &[String],
        test_file: &Path,
        handler: &dyn BaseDriverHandler,
    ) -> Result<HashMap<String, crate::driver_handlers::base_handler::BreakingChangeLocation>> {
        let mut all_breaking_changes = HashMap::new();
        let mut processed_methods = HashSet::new();

        // Recursively process helper methods to find nested calls
        self.find_breaking_changes_in_helper_methods_recursive(
            content,
            helper_methods,
            test_file,
            handler,
            &mut all_breaking_changes,
            &mut processed_methods,
        )?;

        Ok(all_breaking_changes)
    }

    /// Recursively find Breaking Changes in helper methods and their nested calls
    fn find_breaking_changes_in_helper_methods_recursive(
        &self,
        content: &str,
        helper_methods: &[String],
        test_file: &Path,
        handler: &dyn BaseDriverHandler,
        all_breaking_changes: &mut HashMap<
            String,
            crate::driver_handlers::base_handler::BreakingChangeLocation,
        >,
        processed_methods: &mut HashSet<String>,
    ) -> Result<()> {
        // First, look for Breaking Changes in helper methods within the same file
        for helper_method in helper_methods {
            if processed_methods.contains(helper_method) {
                continue; // Avoid infinite recursion
            }
            processed_methods.insert(helper_method.clone());

            // Find Breaking Changes directly in this helper method
            if let Ok(method_breaking_changes) =
                handler.find_breaking_changes_in_method(content, helper_method, test_file)
            {
                all_breaking_changes.extend(method_breaking_changes);
            }

            // Find nested helper method calls within this helper method
            let nested_helper_methods = handler.extract_helper_method_calls(content, helper_method);
            if !nested_helper_methods.is_empty() {
                // Recursively process nested helper methods
                self.find_breaking_changes_in_helper_methods_recursive(
                    content,
                    &nested_helper_methods,
                    test_file,
                    handler,
                    all_breaking_changes,
                    processed_methods,
                )?;
                // Also check cross-file for nested helper methods
                self.find_breaking_changes_in_cross_file_methods(
                    &nested_helper_methods,
                    handler,
                    all_breaking_changes,
                    processed_methods,
                )?;
            }
        }

        // Then, look for Breaking Changes in cross-file helper methods (e.g., common library)
        self.find_breaking_changes_in_cross_file_methods(
            helper_methods,
            handler,
            all_breaking_changes,
            processed_methods,
        )?;

        Ok(())
    }

    /// Find Breaking Changes in cross-file helper methods (e.g., common library)
    fn find_breaking_changes_in_cross_file_methods(
        &self,
        helper_methods: &[String],
        handler: &dyn BaseDriverHandler,
        all_breaking_changes: &mut HashMap<
            String,
            crate::driver_handlers::base_handler::BreakingChangeLocation,
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
                            if let Ok(method_breaking_changes) = handler
                                .find_breaking_changes_in_method(
                                    &common_content,
                                    helper_method,
                                    entry.path(),
                                )
                            {
                                all_breaking_changes.extend(method_breaking_changes);
                            }

                            // Also check for nested calls within this cross-file method
                            let nested_helper_methods =
                                handler.extract_helper_method_calls(&common_content, helper_method);
                            if !nested_helper_methods.is_empty() {
                                // Recursively process nested helper methods in cross-file context
                                self.find_breaking_changes_in_cross_file_methods(
                                    &nested_helper_methods,
                                    handler,
                                    all_breaking_changes,
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

    /// Check if a method exists in the given content
    fn method_exists_in_content(&self, content: &str, method_name: &str) -> bool {
        content.contains(&format!("void {method_name}("))
            || content.contains(&format!("{method_name}("))
            || content.contains(&format!("void {method_name}()"))
            || content.contains(&format!("{method_name}()"))
    }

    /// Look for Breaking Change assertions that might be in other files for existing Breaking Changes
    fn find_cross_file_breaking_change_assertions(
        &self,
        _breaking_change_test_mapping: &mut HashMap<String, BreakingChangeInfo>,
        _handler: &dyn BaseDriverHandler,
        _test_files: &[PathBuf],
    ) -> Result<()> {
        // This method can be implemented later for more complex cross-file scenarios
        // For now, the logic in find_all_breaking_changes_in_helper_methods handles the main cases
        Ok(())
    }
}
