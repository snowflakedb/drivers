use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::base_handler::{BaseDriverHandler, BreakingChangeLocation, TestMethod};
use crate::breaking_changes_utils::parse_breaking_changes_descriptions as parse_breaking_changes_descriptions_util;

pub struct OdbcHandler {
    workspace_root: PathBuf,
}

impl OdbcHandler {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn extract_breaking_change_id(&self, breaking_change_reference: &str) -> String {
        let breaking_change_re = Regex::new(r"(BC#\d+)").unwrap();
        if let Some(captures) = breaking_change_re.captures(breaking_change_reference) {
            if let Some(breaking_change_id) = captures.get(1) {
                return breaking_change_id.as_str().to_string();
            }
        }
        breaking_change_reference.to_string() // Fallback
    }
}

impl BaseDriverHandler for OdbcHandler {
    fn supports_breaking_changes(&self) -> bool {
        true
    }

    fn get_breaking_changes_file_path(&self) -> PathBuf {
        self.workspace_root
            .join("odbc_tests")
            .join("BreakingChanges.md")
    }

    fn get_test_directory(&self) -> PathBuf {
        self.workspace_root.join("odbc_tests").join("tests")
    }

    fn get_test_file_extensions(&self) -> Vec<String> {
        vec!["*.cpp".to_string(), "*.c".to_string()]
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
        let lines: Vec<&str> = content.lines().collect();
        let test_name_re = Regex::new(r#""([^"]+)""#).unwrap();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Look for TEST_CASE start
            if line.starts_with("TEST_CASE(") {
                let start_line = i;
                let mut test_case_content = String::new();
                let mut found_closing_paren = false;

                // Collect the TEST_CASE declaration (might be multi-line)
                let mut j = i;
                while j < lines.len() && !found_closing_paren {
                    let current_line = lines[j].trim();
                    test_case_content.push_str(current_line);
                    test_case_content.push(' ');

                    if current_line.contains(')') && current_line.contains('{') {
                        found_closing_paren = true;
                    }
                    j += 1;
                }

                // Extract test name from the collected content
                if let Some(captures) = test_name_re.captures(&test_case_content) {
                    if let Some(test_name) = captures.get(1) {
                        methods.push(TestMethod {
                            name: test_name.as_str().to_string(),
                            line: start_line + 1,
                        });
                    }
                }

                i = j;
            } else {
                i += 1;
            }
        }

        methods
    }

    fn extract_helper_method_calls(&self, content: &str, method_name: &str) -> Vec<String> {
        let mut helper_methods = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Find the method and extract calls within it
        let mut in_method = false;
        let mut brace_count = 0;

        for line in &lines {
            let line = line.trim();

            if line.contains(&format!("TEST_CASE(\"{method_name}\"")) && !in_method {
                in_method = true;
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;
                continue;
            }

            // Look for method definitions (not calls)
            let is_method_definition = (line.contains(&format!("void {method_name}("))
                || line.contains(&format!("void {method_name}()")))
                && !line.starts_with("//");

            if !in_method && is_method_definition {
                in_method = true;
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;
                continue;
            }

            if in_method {
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;

                if brace_count <= 0 && !line.contains('{') {
                    break;
                }

                // Look for function calls
                let call_re = Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").unwrap();
                for captures in call_re.captures_iter(line) {
                    if let Some(func_name) = captures.get(1) {
                        let name = func_name.as_str();
                        if name != "CHECK"
                            && name != "REQUIRE"
                            && name != "INFO"
                            && !name.starts_with("SQL")
                            && name.len() > 3
                        {
                            helper_methods.push(name.to_string());
                        }
                    }
                }
            }
        }

        helper_methods
    }

    fn find_breaking_changes_in_method(
        &self,
        content: &str,
        method_name: &str,
        file_path: &Path,
    ) -> Result<HashMap<String, BreakingChangeLocation>> {
        let mut breaking_changes = HashMap::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut in_method = false;
        let mut brace_count = 0;
        let mut in_test_case_declaration = false;

        for (line_num, line) in lines.iter().enumerate() {
            let line = line.trim();

            // Check for TEST_CASE start (might be multi-line)
            if line.starts_with("TEST_CASE(") {
                if line.contains(&format!("\"{method_name}\"")) {
                    // Method name is on the same line
                    in_method = true;
                } else {
                    // Method name might be on next line
                    in_test_case_declaration = true;
                }
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;
                continue;
            }

            // Handle multi-line TEST_CASE declaration
            if in_test_case_declaration {
                if line.contains(&format!("\"{method_name}\"")) {
                    in_method = true;
                }
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;
                if line.contains('{') {
                    in_test_case_declaration = false;
                }
                continue;
            }

            // Check for regular void method start
            let is_void_method = line.contains(&format!("void {method_name}("))
                || line.contains(&format!("void {method_name}()"));

            if !line.starts_with("//") && is_void_method && !in_method {
                in_method = true;
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;
                continue;
            }

            if in_method {
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;

                if brace_count <= 0 && !line.contains('{') {
                    break;
                }

                // Look for Breaking Change annotations
                let new_driver_re = Regex::new(r#"NEW_DRIVER_ONLY\s*\(\s*"([^"]+)"\s*\)"#).unwrap();
                let old_driver_re = Regex::new(r#"OLD_DRIVER_ONLY\s*\(\s*"([^"]+)"\s*\)"#).unwrap();

                if let Some(captures) = new_driver_re.captures(line) {
                    if let Some(breaking_change_reference) = captures.get(1) {
                        let breaking_change_reference = breaking_change_reference.as_str();
                        let breaking_change_id =
                            self.extract_breaking_change_id(breaking_change_reference);

                        let breaking_change_location = breaking_changes
                            .entry(breaking_change_id)
                            .or_insert_with(|| BreakingChangeLocation {
                                new_behaviour_file: None,
                                new_behaviour_line: None,
                                old_behaviour_file: None,
                                old_behaviour_line: None,
                            });

                        breaking_change_location.new_behaviour_file = Some(
                            file_path
                                .strip_prefix(&self.workspace_root)
                                .unwrap_or(file_path)
                                .to_string_lossy()
                                .to_string(),
                        );
                        breaking_change_location.new_behaviour_line = Some(line_num + 1);
                    }
                }

                if let Some(captures) = old_driver_re.captures(line) {
                    if let Some(breaking_change_reference) = captures.get(1) {
                        let breaking_change_reference = breaking_change_reference.as_str();
                        let breaking_change_id =
                            self.extract_breaking_change_id(breaking_change_reference);

                        let breaking_change_location = breaking_changes
                            .entry(breaking_change_id)
                            .or_insert_with(|| BreakingChangeLocation {
                                new_behaviour_file: None,
                                new_behaviour_line: None,
                                old_behaviour_file: None,
                                old_behaviour_line: None,
                            });

                        breaking_change_location.old_behaviour_file = Some(
                            file_path
                                .strip_prefix(&self.workspace_root)
                                .unwrap_or(file_path)
                                .to_string_lossy()
                                .to_string(),
                        );
                        breaking_change_location.old_behaviour_line = Some(line_num + 1);
                    }
                }
            }
        }

        Ok(breaking_changes)
    }
}
