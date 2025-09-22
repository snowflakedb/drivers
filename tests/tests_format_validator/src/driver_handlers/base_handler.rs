use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Abstract base trait for driver-specific Breaking Changes processing
pub trait BaseDriverHandler {
    /// Check if this driver supports Breaking Changes processing
    fn supports_breaking_changes(&self) -> bool;

    /// Get the path to the Breaking Changes description file for this driver
    fn get_breaking_changes_file_path(&self) -> PathBuf;

    /// Get the root test directory for this driver
    fn get_test_directory(&self) -> PathBuf;

    /// Get the file extensions for test files in this driver
    fn get_test_file_extensions(&self) -> Vec<String>;

    /// Parse Breaking Changes descriptions from the driver's Breaking Changes file
    fn parse_breaking_changes_descriptions(&self) -> Result<HashMap<String, String>>;

    /// Extract test method information from file content
    fn extract_test_methods(&self, content: &str) -> Vec<TestMethod>;

    /// Extract helper method calls from a specific test method
    fn extract_helper_method_calls(&self, content: &str, test_method: &str) -> Vec<String>;

    /// Find all Breaking Changes within a specific method
    fn find_breaking_changes_in_method(
        &self,
        content: &str,
        method_name: &str,
        file_path: &Path,
    ) -> Result<HashMap<String, BreakingChangeLocation>>;

    /// Find all Breaking Changes within a standalone function (for Python)
    fn find_breaking_changes_in_function(
        &self,
        _content: &str,
        _function_name: &str,
        _file_path: &Path,
    ) -> Result<HashMap<String, BreakingChangeLocation>> {
        // Default implementation does nothing (only Python needs this)
        Ok(HashMap::new())
    }

    /// Check if a test method name matches a scenario name
    fn method_matches_scenario(&self, method_name: &str, scenario_name: &str) -> bool {
        let method_normalized = method_name
            .to_lowercase()
            .replace('_', " ")
            .replace('-', " ");
        let scenario_normalized = scenario_name
            .to_lowercase()
            .replace('_', " ")
            .replace('-', " ");

        let method_words: Vec<&str> = method_normalized.split_whitespace().collect();
        let scenario_words: Vec<&str> = scenario_normalized.split_whitespace().collect();

        // Check if all significant scenario words are present as complete words in the method
        scenario_words.iter().all(|scenario_word| {
            scenario_word.len() <= 2
                || method_words
                    .iter()
                    .any(|method_word| method_word == scenario_word)
        })
    }
}

#[derive(Debug, Clone)]
pub struct TestMethod {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct BreakingChangeLocation {
    pub new_behaviour_file: Option<String>,
    pub new_behaviour_line: Option<usize>,
    pub old_behaviour_file: Option<String>,
    pub old_behaviour_line: Option<usize>,
}
