use super::base_handler::{BaseDriverHandler, BreakingChangeLocation, TestMethod};
use crate::breaking_changes_utils::parse_breaking_changes_descriptions as parse_breaking_changes_descriptions_util;
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct JdbcHandler {
    workspace_root: PathBuf,
}

impl JdbcHandler {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

impl BaseDriverHandler for JdbcHandler {
    fn supports_breaking_changes(&self) -> bool {
        true
    }

    fn get_breaking_changes_file_path(&self) -> PathBuf {
        self.workspace_root.join("jdbc").join("BreakingChanges.md")
    }

    fn get_test_directory(&self) -> PathBuf {
        self.workspace_root
            .join("jdbc")
            .join("src")
            .join("test")
            .join("java")
    }

    fn get_test_file_extensions(&self) -> Vec<String> {
        vec!["*.java".to_string()]
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

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Look for Java test methods: @Test followed by public void methodName()
            if trimmed.starts_with("@Test") {
                // Look for the method declaration in the next few lines
                for j in (line_num + 1)..std::cmp::min(line_num + 5, lines.len()) {
                    let method_line = lines[j].trim();
                    if let Some(captures) =
                        Regex::new(r"public\s+void\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
                            .unwrap()
                            .captures(method_line)
                    {
                        if let Some(method_name) = captures.get(1) {
                            methods.push(TestMethod {
                                name: method_name.as_str().to_string(),
                                line: j + 1,
                            });
                            break;
                        }
                    }
                }
            }
        }

        methods
    }

    fn extract_helper_method_calls(&self, content: &str, test_method: &str) -> Vec<String> {
        let mut helper_calls = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut in_method = false;
        let mut brace_count = 0;

        for line in lines {
            let trimmed = line.trim();

            // Check for method start
            if trimmed.contains(&format!("void {test_method}(")) && !in_method {
                in_method = true;
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;
                continue;
            }

            if in_method {
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;

                if brace_count <= 0 && !line.contains('{') {
                    break;
                }

                // Look for method calls: ClassName.methodName() or methodName()
                let method_call_re =
                    Regex::new(r"([A-Za-z_][A-Za-z0-9_]*\.)?([A-Za-z_][A-Za-z0-9_]*)\s*\(")
                        .unwrap();
                for captures in method_call_re.captures_iter(trimmed) {
                    if let Some(method_name) = captures.get(2) {
                        let method = method_name.as_str();
                        // Filter out common Java methods and keywords
                        if ![
                            "println", "print", "equals", "toString", "hashCode", "System",
                            "String",
                        ]
                        .contains(&method)
                        {
                            helper_calls.push(method.to_string());
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
        let mut breaking_changes = HashMap::new();

        // First, look for Breaking Changes directly in the method
        self.find_direct_breaking_changes_in_method(
            content,
            method_name,
            file_path,
            &mut breaking_changes,
        )?;

        // Then, look for helper method calls and search for Breaking Changes in those methods
        let helper_calls = self.extract_helper_method_calls(content, method_name);
        for helper_method in helper_calls {
            // Search for the helper method in the same file first
            self.find_direct_breaking_changes_in_method(
                content,
                &helper_method,
                file_path,
                &mut breaking_changes,
            )?;

            // If not found in same file, search in common helper files
            if !content.contains(&format!("void {helper_method}(")) {
                self.search_breaking_changes_in_helper_files(
                    &helper_method,
                    &mut breaking_changes,
                )?;
            }
        }

        Ok(breaking_changes)
    }

    fn method_matches_scenario(&self, method_name: &str, scenario_name: &str) -> bool {
        // Java method naming: shouldAuthenticateUsingPrivateFileWithPassword
        // Scenario naming: should authenticate using private file with password

        let method_normalized = method_name.to_lowercase().replace("_", "");
        let scenario_normalized = scenario_name
            .to_lowercase()
            .replace(" ", "")
            .replace("_", "")
            .replace("-", "");

        method_normalized == scenario_normalized
    }
}

impl JdbcHandler {
    fn extract_breaking_change_id(&self, breaking_change_reference: &str) -> String {
        // Extract Breaking Change ID from reference (e.g., "BC#1" -> "BC#1")
        breaking_change_reference.to_string()
    }

    fn find_direct_breaking_changes_in_method(
        &self,
        content: &str,
        method_name: &str,
        file_path: &Path,
        breaking_changes: &mut HashMap<String, BreakingChangeLocation>,
    ) -> Result<()> {
        let lines: Vec<&str> = content.lines().collect();
        let mut in_method = false;
        let mut brace_count = 0;

        for (line_num, line) in lines.iter().enumerate() {
            let line = line.trim();

            // Check for method start
            let is_method = line.contains(&format!("void {method_name}("))
                || line.contains(&format!("{method_name}("))
                || line.contains(&format!("static void {method_name}("));

            if !line.starts_with("//") && is_method && !in_method {
                in_method = true;
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;
                continue;
            }

            if in_method {
                brace_count += line.matches('{').count() as i32 - line.matches('}').count() as i32;

                if brace_count <= 0 && !line.contains('{') {
                    break;
                }

                // Look for Breaking Change annotations in Java comments
                let new_driver_re =
                    Regex::new(r#"//\s*NEW_DRIVER_ONLY\s*\(\s*"([^"]+)"\s*\)"#).unwrap();
                let old_driver_re =
                    Regex::new(r#"//\s*OLD_DRIVER_ONLY\s*\(\s*"([^"]+)"\s*\)"#).unwrap();

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

        Ok(())
    }

    fn search_breaking_changes_in_helper_files(
        &self,
        helper_method: &str,
        breaking_changes: &mut HashMap<String, BreakingChangeLocation>,
    ) -> Result<()> {
        // Search for helper method in common directory
        let common_dir = self.get_test_directory().join("common");
        if !common_dir.exists() {
            return Ok(());
        }

        // Search all Java files in common directory
        for entry in WalkDir::new(&common_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "java"))
        {
            let helper_file_path = entry.path();
            if let Ok(helper_content) = fs::read_to_string(helper_file_path) {
                // Search for Breaking Changes in the helper method
                self.find_direct_breaking_changes_in_method(
                    &helper_content,
                    helper_method,
                    helper_file_path,
                    breaking_changes,
                )?;
            }
        }

        Ok(())
    }
}
