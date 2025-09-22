use super::base_handler::{BaseDriverHandler, BreakingChangeLocation, TestMethod};
use crate::breaking_changes_utils::parse_breaking_changes_descriptions as parse_breaking_changes_descriptions_util;
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct PythonHandler {
    workspace_root: PathBuf,
}

impl PythonHandler {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

impl BaseDriverHandler for PythonHandler {
    fn supports_breaking_changes(&self) -> bool {
        true
    }

    fn get_breaking_changes_file_path(&self) -> PathBuf {
        self.workspace_root
            .join("pep249_dbapi")
            .join("BreakingChanges.md")
    }

    fn get_test_directory(&self) -> PathBuf {
        self.workspace_root.join("pep249_dbapi/tests")
    }

    fn get_test_file_extensions(&self) -> Vec<String> {
        vec!["*.py".to_string()]
    }

    fn parse_breaking_changes_descriptions(&self) -> Result<HashMap<String, String>> {
        let breaking_change_file_path = self.get_breaking_changes_file_path();
        if !breaking_change_file_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&breaking_change_file_path).with_context(|| {
            format!(
                "Failed to read Breaking Change file: {}",
                breaking_change_file_path.display()
            )
        })?;

        parse_breaking_changes_descriptions_util(&content)
    }

    fn extract_test_methods(&self, content: &str) -> Vec<TestMethod> {
        let mut methods = Vec::new();
        // Match Python test methods: def test_method_name(
        let test_method_re = Regex::new(r"def\s+(test_\w+)\s*\(").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(captures) = test_method_re.captures(line) {
                if let Some(method_name) = captures.get(1) {
                    methods.push(TestMethod {
                        name: method_name.as_str().to_string(),
                        line: line_num + 1,
                    });
                }
            }
        }

        methods
    }

    fn extract_helper_method_calls(&self, content: &str, test_method: &str) -> Vec<String> {
        let mut helper_calls = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_test_method = false;
        let mut _brace_level = 0;

        for line in &lines {
            let trimmed = line.trim();

            // Check if we're entering the test method
            if trimmed.starts_with(&format!("def {}(", test_method)) {
                in_test_method = true;
                continue;
            }

            if in_test_method {
                // Track indentation level (Python uses indentation instead of braces)
                if !trimmed.is_empty() {
                    let indent_level = line.len() - line.trim_start().len();

                    // If we hit a line with same or less indentation that starts with def/class, we're out of the method
                    if indent_level <= 4
                        && (trimmed.starts_with("def ") || trimmed.starts_with("class "))
                        && !trimmed.starts_with(&format!("def {}(", test_method))
                    {
                        break;
                    }

                    // Look for method calls like self._method_name()
                    if let Some(method_call) = self.extract_method_call_from_line(trimmed) {
                        if !helper_calls.contains(&method_call) {
                            helper_calls.push(method_call);
                        }
                    }
                }
            }
        }

        helper_calls
    }

    fn find_breaking_changes_in_method(
        &self,
        content: &str,
        method_name: &str,
        file_path: &Path,
    ) -> Result<HashMap<String, BreakingChangeLocation>> {
        let mut breaking_changes: HashMap<String, BreakingChangeLocation> = HashMap::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_method = false;

        // Find the method start
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with(&format!("def {}(", method_name)) {
                in_method = true;
                continue;
            }

            if in_method {
                let indent_level = line.len() - line.trim_start().len();

                // If we hit a line with same or less indentation that starts with def/class, we're out of the method
                if indent_level <= 4
                    && (trimmed.starts_with("def ") || trimmed.starts_with("class "))
                    && !trimmed.starts_with(&format!("def {}(", method_name))
                {
                    break;
                }

                // Look for Breaking Change patterns in Python: NEW_DRIVER_ONLY("BC#X") or OLD_DRIVER_ONLY("BC#X")
                if let Some(breaking_change_id) = self.extract_breaking_change_from_line(trimmed) {
                    // Determine if this is NEW or OLD driver behavior
                    let is_new_driver = trimmed.contains("NEW_DRIVER_ONLY");

                    let location = if is_new_driver {
                        BreakingChangeLocation {
                            new_behaviour_file: Some(
                                file_path
                                    .strip_prefix(&self.workspace_root)
                                    .unwrap_or(file_path)
                                    .to_string_lossy()
                                    .to_string(),
                            ),
                            new_behaviour_line: Some(line_num + 1),
                            old_behaviour_file: None,
                            old_behaviour_line: None,
                        }
                    } else {
                        BreakingChangeLocation {
                            new_behaviour_file: None,
                            new_behaviour_line: None,
                            old_behaviour_file: Some(
                                file_path
                                    .strip_prefix(&self.workspace_root)
                                    .unwrap_or(file_path)
                                    .to_string_lossy()
                                    .to_string(),
                            ),
                            old_behaviour_line: Some(line_num + 1),
                        }
                    };

                    // If we already have this Breaking Change, merge the locations
                    if let Some(existing) = breaking_changes.get_mut(&breaking_change_id) {
                        if is_new_driver {
                            existing.new_behaviour_file = location.new_behaviour_file;
                            existing.new_behaviour_line = location.new_behaviour_line;
                        } else {
                            existing.old_behaviour_file = location.old_behaviour_file;
                            existing.old_behaviour_line = location.old_behaviour_line;
                        }
                    } else {
                        breaking_changes.insert(breaking_change_id, location);
                    }
                }
            }
        }

        Ok(breaking_changes)
    }

    fn method_matches_scenario(&self, method_name: &str, scenario_name: &str) -> bool {
        // Convert Python test method name to words (remove test_ prefix and split by underscores)
        let method_words: Vec<&str> = method_name
            .strip_prefix("test_")
            .unwrap_or(method_name)
            .split('_')
            .filter(|word| word.len() > 2) // Only consider significant words
            .collect();

        // Convert scenario name to words
        let scenario_lower = scenario_name.to_lowercase();
        let scenario_words: Vec<&str> = scenario_lower
            .split_whitespace()
            .filter(|word| word.len() > 2) // Only consider significant words
            .collect();

        // Check if significant words from method name appear in scenario name
        let matching_words = method_words
            .iter()
            .filter(|method_word| {
                scenario_words
                    .iter()
                    .any(|scenario_word| *method_word == scenario_word)
            })
            .count();

        // Consider it a match if at least 2 significant words match
        matching_words >= 2
    }

    fn find_breaking_changes_in_function(
        &self,
        content: &str,
        function_name: &str,
        file_path: &Path,
    ) -> Result<HashMap<String, BreakingChangeLocation>> {
        // Use the internal implementation for Python standalone functions
        self.find_breaking_changes_in_function_internal(content, function_name, file_path)
    }
}

impl PythonHandler {
    fn find_breaking_changes_in_function_internal(
        &self,
        content: &str,
        function_name: &str,
        file_path: &Path,
    ) -> Result<HashMap<String, BreakingChangeLocation>> {
        let mut breaking_changes: HashMap<String, BreakingChangeLocation> = HashMap::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_function = false;

        // Find the function start
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with(&format!("def {}(", function_name)) {
                in_function = true;
                continue;
            }

            if in_function {
                let indent_level = line.len() - line.trim_start().len();

                // If we hit a line with same or less indentation that starts with def/class, we're out of the function
                if indent_level == 0
                    && (trimmed.starts_with("def ") || trimmed.starts_with("class "))
                    && !trimmed.starts_with(&format!("def {}(", function_name))
                {
                    break;
                }

                // Look for Breaking Change patterns in Python: NEW_DRIVER_ONLY("BC#X") or OLD_DRIVER_ONLY("BC#X")
                if let Some(breaking_change_id) = self.extract_breaking_change_from_line(trimmed) {
                    // Determine if this is NEW or OLD driver behavior
                    let is_new_driver = trimmed.contains("NEW_DRIVER_ONLY");

                    let location = if is_new_driver {
                        BreakingChangeLocation {
                            new_behaviour_file: Some(
                                file_path
                                    .strip_prefix(&self.workspace_root)
                                    .unwrap_or(file_path)
                                    .to_string_lossy()
                                    .to_string(),
                            ),
                            new_behaviour_line: Some(line_num + 1),
                            old_behaviour_file: None,
                            old_behaviour_line: None,
                        }
                    } else {
                        BreakingChangeLocation {
                            new_behaviour_file: None,
                            new_behaviour_line: None,
                            old_behaviour_file: Some(
                                file_path
                                    .strip_prefix(&self.workspace_root)
                                    .unwrap_or(file_path)
                                    .to_string_lossy()
                                    .to_string(),
                            ),
                            old_behaviour_line: Some(line_num + 1),
                        }
                    };

                    // Merge with existing entry if it exists (to handle both NEW and OLD driver patterns)
                    if let Some(existing_location) = breaking_changes.get_mut(&breaking_change_id) {
                        // Merge the new information with the existing entry
                        if is_new_driver {
                            existing_location.new_behaviour_file = location.new_behaviour_file;
                            existing_location.new_behaviour_line = location.new_behaviour_line;
                        } else {
                            existing_location.old_behaviour_file = location.old_behaviour_file;
                            existing_location.old_behaviour_line = location.old_behaviour_line;
                        }
                    } else {
                        // First time seeing this breaking change ID
                        breaking_changes.insert(breaking_change_id, location);
                    }
                }
            }
        }

        Ok(breaking_changes)
    }

    fn extract_method_call_from_line(&self, line: &str) -> Option<String> {
        // Look for self._method_name() calls (class methods)
        let class_method_re = Regex::new(r"self\.(_\w+)\s*\(").unwrap();
        if let Some(captures) = class_method_re.captures(line) {
            if let Some(method_name) = captures.get(1) {
                return Some(method_name.as_str().to_string());
            }
        }

        // Look for standalone function calls like function_name()
        let function_call_re = Regex::new(r"(\w+)\s*\(").unwrap();
        if let Some(captures) = function_call_re.captures(line) {
            if let Some(function_name) = captures.get(1) {
                let func_name = function_name.as_str();
                // Only consider functions that are likely test helpers (not built-ins or common functions)
                if func_name.len() > 3
                    && !matches!(
                        func_name,
                        "print"
                            | "len"
                            | "str"
                            | "int"
                            | "float"
                            | "bool"
                            | "list"
                            | "dict"
                            | "set"
                            | "tuple"
                            | "range"
                            | "enumerate"
                            | "zip"
                            | "map"
                            | "filter"
                            | "sorted"
                            | "reversed"
                            | "max"
                            | "min"
                            | "sum"
                            | "any"
                            | "all"
                            | "open"
                            | "format"
                            | "join"
                            | "split"
                            | "replace"
                            | "strip"
                            | "lower"
                            | "upper"
                            | "startswith"
                            | "endswith"
                            | "find"
                            | "index"
                            | "count"
                            | "append"
                            | "extend"
                            | "insert"
                            | "remove"
                            | "pop"
                            | "clear"
                            | "copy"
                            | "get"
                            | "keys"
                            | "values"
                            | "items"
                            | "update"
                    )
                {
                    return Some(func_name.to_string());
                }
            }
        }

        None
    }

    fn extract_breaking_change_from_line(&self, line: &str) -> Option<String> {
        // Look for NEW_DRIVER_ONLY("BC#X") or OLD_DRIVER_ONLY("BC#X") patterns
        let breaking_change_re =
            Regex::new(r#"(?:NEW_DRIVER_ONLY|OLD_DRIVER_ONLY)\s*\(\s*"(BC#\d+)"\s*\)"#).unwrap();
        if let Some(captures) = breaking_change_re.captures(line) {
            if let Some(breaking_change_id) = captures.get(1) {
                return Some(breaking_change_id.as_str().to_string());
            }
        }
        None
    }
}
