use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::behavior_differences_processor::BehaviorDifferencesProcessor;
use crate::feature_parser::Feature;
use crate::step_finder::StepFinder;
use crate::test_discovery::{Language, TestDiscovery};

pub struct GherkinValidator {
    _workspace_root: PathBuf,
    features_dir: PathBuf,
    discovery: TestDiscovery,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub feature_file: PathBuf,
    pub validations: Vec<LanguageValidation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LanguageValidation {
    pub language: Language,
    pub test_file_found: bool,
    pub test_file_path: Option<PathBuf>,
    pub missing_steps: Vec<String>,
    pub implemented_steps: Vec<String>,
    pub warnings: Vec<String>,
    pub missing_steps_by_method: Vec<MethodValidation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodValidation {
    pub method_name: String,
    pub scenario_name: String,
    pub missing_steps: Vec<String>,
    pub line_number: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrphanValidation {
    pub language: Language,
    pub orphaned_files: Vec<OrphanedTestFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrphanedTestFile {
    pub file_path: PathBuf,
    pub orphaned_methods: Vec<String>,
}

// Behavior Differences related structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BehaviorDifferenceInfo {
    pub behavior_difference_id: String,
    pub description: String,
    pub implementations: Vec<BehaviorDifferenceImplementation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BehaviorDifferenceImplementation {
    pub test_method: String,
    pub test_file: String,
    pub test_line: usize,
    pub new_behaviour_file: Option<String>,
    pub new_behaviour_line: Option<usize>,
    pub old_behaviour_file: Option<String>,
    pub old_behaviour_line: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BehaviorDifferencesReport {
    pub behavior_difference_descriptions: HashMap<String, String>,
    pub behavior_differences_by_language: HashMap<String, Vec<BehaviorDifferenceInfo>>,
}

// Enhanced validation result that includes Behavior Differences information
#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedValidationResult {
    pub validation_results: Vec<ValidationResult>,
    pub orphan_results: Vec<OrphanValidation>,
    pub behavior_differences_report: BehaviorDifferencesReport,
}

impl GherkinValidator {
    pub fn new(workspace_root: PathBuf, features_dir: PathBuf) -> Result<Self> {
        let discovery = TestDiscovery::new(workspace_root.clone());

        Ok(Self {
            _workspace_root: workspace_root,
            features_dir,
            discovery,
        })
    }

    pub fn validate_all_features(&self) -> Result<Vec<ValidationResult>> {
        let mut results = Vec::new();

        // Find all .feature files
        for entry in WalkDir::new(&self.features_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "feature"))
        {
            let feature = Feature::parse_from_file(entry.path()).with_context(|| {
                format!("Failed to parse feature file: {}", entry.path().display())
            })?;

            let validation_result = self.validate_feature_with_path(&feature, entry.path())?;
            results.push(validation_result);
        }

        Ok(results)
    }

    /// Find orphaned test files and methods that don't correspond to any feature scenarios
    pub fn find_orphaned_tests(&self) -> Result<Vec<OrphanValidation>> {
        let mut orphan_validations = Vec::new();

        // First, collect all feature scenarios
        let all_scenarios = self.collect_all_scenarios()?;

        // Check each language's test directories
        for language in &[Language::Rust, Language::Jdbc, Language::Odbc] {
            let orphaned_files = self.find_orphaned_files_for_language(language, &all_scenarios)?;
            if !orphaned_files.is_empty() {
                orphan_validations.push(OrphanValidation {
                    language: language.clone(),
                    orphaned_files,
                });
            }
        }

        Ok(orphan_validations)
    }

    fn collect_all_scenarios(&self) -> Result<Vec<(String, String)>> {
        let mut scenarios = Vec::new();

        // Walk through all .feature files
        for entry in WalkDir::new(&self.features_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "feature"))
        {
            let feature_path = entry.path();
            let feature = Feature::parse_from_file(feature_path)?;
            let feature_name = feature_path
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            for scenario in &feature.scenarios {
                scenarios.push((feature_name.clone(), scenario.name.clone()));
            }
        }

        Ok(scenarios)
    }

    fn find_orphaned_files_for_language(
        &self,
        language: &Language,
        all_scenarios: &[(String, String)],
    ) -> Result<Vec<OrphanedTestFile>> {
        let mut orphaned_files = Vec::new();

        let test_dirs = self.get_test_directories_for_language(language);

        for test_dir in test_dirs {
            if !test_dir.exists() {
                continue;
            }

            // Walk through test files
            for entry in WalkDir::new(&test_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter(|e| self.is_test_file_for_language(e.path(), language))
                .filter(|e| !self.is_utility_file(e.path()))
            {
                let test_file_path = entry.path();
                let orphaned_methods =
                    self.find_orphaned_methods_in_file(test_file_path, language, all_scenarios)?;

                let file_name = test_file_path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();

                // Check if the file itself corresponds to any feature
                let file_matches_feature = all_scenarios.iter().any(|(feature_name, _)| {
                    self.file_name_matches_feature(&file_name, feature_name)
                });

                if !file_matches_feature {
                    // File doesn't match any feature - report as orphaned file (no methods)
                    orphaned_files.push(OrphanedTestFile {
                        file_path: test_file_path.to_path_buf(),
                        orphaned_methods: vec![], // Empty for orphaned files
                    });
                } else if !orphaned_methods.is_empty() {
                    // File matches feature but has orphaned methods
                    orphaned_files.push(OrphanedTestFile {
                        file_path: test_file_path.to_path_buf(),
                        orphaned_methods,
                    });
                }
            }
        }

        Ok(orphaned_files)
    }

    fn is_utility_file(&self, file_path: &Path) -> bool {
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip common utility files
        file_name == "mod.rs"
            || file_name.ends_with("_common.rs")
            || file_name.ends_with("_helper.rs")
            || file_name.ends_with("_utils.rs")
            || file_path.to_string_lossy().contains("/common/")
            || file_path.to_string_lossy().contains("/steps/")
            || file_path.to_string_lossy().contains("/utils/")
            || file_path.to_string_lossy().contains("/helpers/")
    }

    fn get_test_directories_for_language(&self, language: &Language) -> Vec<PathBuf> {
        // Only check e2e tests for orphaned tests as per requirements
        match language {
            Language::Rust => vec![self._workspace_root.join("sf_core/tests/e2e")],
            Language::Jdbc => vec![
                self._workspace_root
                    .join("jdbc/src/test/java/com/snowflake/jdbc/e2e"),
            ],
            Language::Odbc => vec![self._workspace_root.join("odbc_tests/tests/e2e")],
            Language::Python => vec![self._workspace_root.join("pep249_dbapi/tests/e2e")],
            _ => vec![],
        }
    }

    fn is_test_file_for_language(&self, file_path: &Path, language: &Language) -> bool {
        if let Some(extension) = file_path.extension() {
            match language {
                Language::Rust => extension == "rs",
                Language::Jdbc => extension == "java",
                Language::Odbc => extension == "cpp",
                _ => false,
            }
        } else {
            false
        }
    }

    fn file_name_matches_feature(&self, file_name: &str, feature_name: &str) -> bool {
        use crate::utils::{strings_match_normalized, to_pascal_case, to_snake_case};

        // Remove common test suffixes
        let clean_file_name = file_name
            .trim_end_matches("Test")
            .trim_end_matches("Tests")
            .trim_end_matches("_test")
            .trim_end_matches("_tests");

        strings_match_normalized(clean_file_name, feature_name)
            || strings_match_normalized(clean_file_name, &to_pascal_case(feature_name))
            || strings_match_normalized(clean_file_name, &to_snake_case(feature_name))
    }

    fn find_orphaned_methods_in_file(
        &self,
        file_path: &Path,
        language: &Language,
        all_scenarios: &[(String, String)],
    ) -> Result<Vec<String>> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read test file: {}", file_path.display()))?;

        let mut orphaned_methods = Vec::new();

        // Get all test methods in this file
        let all_methods = self.get_all_test_methods_in_file(&content, language)?;

        for method_name in all_methods {
            let method_matches_scenario = all_scenarios.iter().any(|(_, scenario_name)| {
                self.method_name_matches_scenario(&method_name, scenario_name)
            });

            if !method_matches_scenario {
                orphaned_methods.push(method_name);
            }
        }

        Ok(orphaned_methods)
    }

    fn get_all_test_methods_in_file(
        &self,
        content: &str,
        language: &Language,
    ) -> Result<Vec<String>> {
        use regex::Regex;
        let mut methods = Vec::new();

        match language {
            Language::Rust => {
                let test_regex = Regex::new(r"#\[test\]\s*(?:\n\s*)*fn\s+(\w+)\s*\(")?;
                for captures in test_regex.captures_iter(content) {
                    methods.push(captures[1].to_string());
                }
            }
            Language::Jdbc => {
                let test_regex =
                    Regex::new(r"@Test\s*(?:\n\s*)?(?:public\s+)?(?:void\s+)?(\w+)\s*\(")?;
                for captures in test_regex.captures_iter(content) {
                    methods.push(captures[1].to_string());
                }
            }
            Language::Odbc => {
                let catch2_regex = Regex::new(r#"TEST_CASE\s*\(\s*"([^"]+)""#)?;
                for captures in catch2_regex.captures_iter(content) {
                    methods.push(captures[1].to_string());
                }
            }
            _ => {}
        }

        Ok(methods)
    }

    fn method_name_matches_scenario(&self, method_name: &str, scenario_name: &str) -> bool {
        use crate::utils::{strings_match_normalized, to_pascal_case, to_snake_case};

        strings_match_normalized(method_name, scenario_name)
            || strings_match_normalized(method_name, &to_pascal_case(scenario_name))
            || strings_match_normalized(method_name, &to_snake_case(scenario_name))
    }

    pub fn validate_feature_with_path(
        &self,
        feature: &Feature,
        feature_path: &Path,
    ) -> Result<ValidationResult> {
        let mut validations = Vec::new();

        // Get target languages from feature-level tags
        let target_languages = TestDiscovery::get_target_languages(&feature.tags);

        for language in target_languages {
            let validation =
                self.validate_language_implementation_with_path(feature, feature_path, language)?;
            validations.push(validation);
        }

        // Also check scenario-level tags for additional languages
        for scenario in &feature.scenarios {
            let scenario_languages = TestDiscovery::get_target_languages(&scenario.tags);
            for language in scenario_languages {
                // Only add if we haven't already validated this language
                if !validations.iter().any(|v| v.language == language) {
                    let validation = self.validate_language_implementation_with_path(
                        feature,
                        feature_path,
                        language,
                    )?;
                    validations.push(validation);
                }
            }
        }

        Ok(ValidationResult {
            feature_file: feature.file_path.clone(),
            validations,
        })
    }

    fn validate_language_implementation_with_path(
        &self,
        feature: &Feature,
        feature_path: &Path,
        language: Language,
    ) -> Result<LanguageValidation> {
        // Find test file using the feature path (includes subdirectory structure)
        let test_file = self
            .discovery
            .find_test_file_with_path(feature_path, &language);

        if let Some(test_file_path) = test_file {
            let step_finder = StepFinder::new(language.clone());

            // Check if we need to validate specific scenarios or the whole file
            let mut all_implemented_steps = Vec::new();
            let mut all_missing_steps = Vec::new();
            let mut warnings = Vec::new();
            let mut missing_steps_by_method = Vec::new();

            // Check if any scenarios have language-specific tags
            let language_specific_scenarios: Vec<_> = feature
                .scenarios
                .iter()
                .filter(|scenario| {
                    TestDiscovery::get_target_languages(&scenario.tags).contains(&language)
                })
                .collect();

            if language_specific_scenarios.is_empty() {
                // No scenario-specific tags, validate all steps in the file
                let implemented_steps = step_finder.find_implemented_steps(&test_file_path)?;
                let feature_steps = feature.get_all_step_texts();

                let missing_steps = self.find_missing_steps(&feature_steps, &implemented_steps);

                all_implemented_steps = implemented_steps;
                all_missing_steps = missing_steps;
            } else {
                // Validate specific scenarios - check test methods FIRST
                for scenario in language_specific_scenarios {
                    // Determine the test level for this scenario
                    let test_level = TestDiscovery::get_test_level(&scenario.tags);

                    // Find the appropriate test file based on test level
                    let scenario_test_file = self.discovery.find_test_file_with_path_and_level(
                        feature_path,
                        &language,
                        test_level,
                    );

                    let actual_test_file_path =
                        scenario_test_file.as_ref().unwrap_or(&test_file_path);

                    // First, check if test method exists for this scenario
                    let test_methods_with_lines = step_finder
                        .find_test_methods_with_lines(actual_test_file_path, &scenario.name)?;

                    if test_methods_with_lines.is_empty() {
                        warnings.push(format!(
                            "No test method found for scenario: {}",
                            scenario.name
                        ));
                        // Don't check steps if no test method exists - skip this scenario entirely
                        continue;
                    }

                    // For each test method found, check if it implements all scenario steps
                    for (method_name, line_number) in test_methods_with_lines {
                        let method_steps = step_finder
                            .find_steps_in_method(actual_test_file_path, &method_name)?;
                        let scenario_steps: Vec<String> = scenario
                            .steps
                            .iter()
                            .map(|step| format!("{:?} {}", step.step_type, step.text))
                            .collect();

                        // Track missing steps for this specific method
                        let mut method_missing_steps = Vec::new();

                        for step_text in &scenario_steps {
                            if !method_steps.contains(step_text) {
                                method_missing_steps.push(step_text.clone());
                                if !all_missing_steps.contains(step_text) {
                                    all_missing_steps.push(step_text.clone());
                                }
                            }
                        }

                        // Add implemented steps to the overall list
                        for step in method_steps {
                            if !all_implemented_steps.contains(&step) {
                                all_implemented_steps.push(step);
                            }
                        }

                        // If there are missing steps in this method, record them
                        if !method_missing_steps.is_empty() {
                            missing_steps_by_method.push(MethodValidation {
                                method_name: method_name.clone(),
                                scenario_name: scenario.name.clone(),
                                missing_steps: method_missing_steps,
                                line_number: Some(line_number),
                            });
                        }
                    }
                }
            }

            Ok(LanguageValidation {
                language,
                test_file_found: true,
                test_file_path: Some(test_file_path.to_path_buf()),
                missing_steps: all_missing_steps,
                implemented_steps: all_implemented_steps,
                warnings,
                missing_steps_by_method,
            })
        } else {
            Ok(LanguageValidation {
                language,
                test_file_found: false,
                test_file_path: None,
                missing_steps: feature.get_all_step_texts(),
                implemented_steps: Vec::new(),
                warnings: vec![format!("No test file found for feature: {}", feature.name)],
                missing_steps_by_method: Vec::new(),
            })
        }
    }

    fn find_missing_steps(
        &self,
        feature_steps: &[String],
        implemented_steps: &[String],
    ) -> Vec<String> {
        feature_steps
            .iter()
            .filter(|feature_step| {
                !implemented_steps
                    .iter()
                    .any(|impl_step| self.steps_match(impl_step, feature_step))
            })
            .cloned()
            .collect()
    }

    fn steps_match(&self, implemented_step: &str, feature_step: &str) -> bool {
        // Normalize both steps for comparison - only remove punctuation, keep all words
        let normalize = |s: &str| {
            s.to_lowercase()
                .replace("\"", "")
                .replace("'", "")
                .replace(",", "")
                .replace(".", "")
                .replace(":", "")
                .replace(";", "")
                .replace("!", "")
                .replace("?", "")
                .replace("(", "")
                .replace(")", "")
                .trim()
                .to_string()
        };

        let norm_impl = normalize(implemented_step);
        let norm_feature = normalize(feature_step);

        // Require exact match after normalization - no partial matches allowed
        norm_impl == norm_feature
    }

    pub fn validate_all_with_breaking_changes(&self) -> Result<EnhancedValidationResult> {
        let validation_results = self.validate_all_features()?;
        let orphan_results = self.find_orphaned_tests()?;

        // Create feature info map from parsed features
        let mut features = HashMap::new();

        // Parse all feature files
        for entry in WalkDir::new(&self.features_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "feature"))
        {
            if let Ok(feature) = Feature::parse_from_file(entry.path()) {
                // Extract Behavior Difference scenarios (scenarios with @{driver}_behavior_difference annotations)
                // Include scenarios with driver tags that might have Behavior Difference implementations
                // We'll check for actual Behavior Difference implementations during processing
                let behavior_difference_scenarios = feature
                    .scenarios
                    .iter()
                    .filter(|scenario| {
                        scenario.tags.iter().any(|tag| {
                            let tag_str = tag.as_str();
                            matches!(
                                tag_str,
                                "odbc"
                                    | "jdbc"
                                    | "python"
                                    | "pep249"
                                    | "core"
                                    | "csharp"
                                    | "dotnet"
                                    | "javascript"
                                    | "nodejs"
                                    | "js"
                            ) || tag_str.starts_with("odbc_")
                                || tag_str.starts_with("jdbc_")
                                || tag_str.starts_with("python_")
                                || tag_str.starts_with("core_")
                                || tag_str.starts_with("csharp_")
                                || tag_str.starts_with("dotnet_")
                                || tag_str.starts_with("javascript_")
                                || tag_str.starts_with("nodejs_")
                                || tag_str.starts_with("js_")
                        })
                    })
                    .map(|s| s.name.clone())
                    .collect();

                features.insert(
                    feature.name.clone(),
                    crate::behavior_differences_processor::FeatureInfo {
                        behavior_difference_scenarios,
                    },
                );
            }
        }

        // Process Behavior Differences
        let behavior_differences_processor =
            BehaviorDifferencesProcessor::new(self._workspace_root.clone());
        let behavior_differences_report =
            behavior_differences_processor.process_behavior_differences(&features)?;

        Ok(EnhancedValidationResult {
            validation_results,
            orphan_results,
            behavior_differences_report,
        })
    }
}
