use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::behavior_differences_processor::BehaviorDifferencesProcessor;
use crate::feature_parser::Feature;
use crate::step_finder::StepFinder;
use crate::test_discovery::{Language, TestDiscovery, TestLevel};

pub struct GherkinValidator {
    _workspace_root: PathBuf,
    features_dir: PathBuf,
    discovery: TestDiscovery,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub feature_file: PathBuf,
    pub validations: Vec<LanguageValidation>,
    pub scenario_structure_errors: Vec<String>,
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
    pub empty_steps: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MethodValidation {
    pub method_name: String,
    pub scenario_name: String,
    pub missing_steps: Vec<String>,
    pub empty_steps: Vec<String>,
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
    pub reason: OrphanReason,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrphanReason {
    NoMatchingFeature,
    LanguageMarkedAsNotNeeded,
    FeatureMissingGenericLanguageTag,
    FeatureExistsButNoScenarioTags,
    MethodsWithoutScenarioTags,
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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub old_driver_skipped: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub new_driver_skipped: bool,
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

        // First, collect all feature scenarios and language requirements
        let (all_scenarios, feature_language_requirements, scenario_language_requirements) =
            self.collect_all_scenarios_and_languages()?;

        // Check each language's test directories
        for language in &[
            Language::Rust,
            Language::Jdbc,
            Language::Odbc,
            Language::Python,
        ] {
            let orphaned_files = self.find_orphaned_files_for_language(
                language,
                &all_scenarios,
                &feature_language_requirements,
                &scenario_language_requirements,
            )?;
            if !orphaned_files.is_empty() {
                orphan_validations.push(OrphanValidation {
                    language: language.clone(),
                    orphaned_files,
                });
            }
        }

        Ok(orphan_validations)
    }

    /// Find features that have no tags at all (TODO items)
    pub fn find_untagged_features(&self) -> Result<Vec<PathBuf>> {
        use walkdir::WalkDir;
        let mut untagged_features = Vec::new();

        for entry in WalkDir::new(&self.features_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "feature"))
        {
            let feature_path = entry.path();
            let feature = Feature::parse_from_file(feature_path)?;

            // Check if feature has no tags and no scenario has tags
            let feature_has_tags = !feature.tags.is_empty();
            let scenarios_have_tags = feature.scenarios.iter().any(|s| !s.tags.is_empty());

            if !feature_has_tags && !scenarios_have_tags {
                untagged_features.push(feature_path.to_path_buf());
            }
        }

        Ok(untagged_features)
    }

    /// Get a unique feature ID that includes the relative path to distinguish
    /// features with the same name in different directories (e.g., shared/session/logout vs core/session/logout)
    fn get_feature_id(&self, feature_path: &Path) -> String {
        // Get path relative to features_dir
        let raw_id = if let Ok(relative) = feature_path.strip_prefix(&self.features_dir) {
            // Remove .feature extension and convert to string (lossy to avoid panics on non-UTF8 paths)
            relative.with_extension("").to_string_lossy().into_owned()
        } else if let Some(stem) = feature_path.file_stem() {
            // Fall back to the file stem if we cannot get a relative path
            stem.to_string_lossy().into_owned()
        } else {
            // As a last resort, use the full path as a string (lossy) to avoid panicking
            feature_path.to_string_lossy().into_owned()
        };

        // Normalize path separators to forward slashes for cross-platform consistency.
        // On Windows, PathBuf::to_str() returns backslashes, but our prefix checks
        // (e.g., starts_with("shared/")) expect forward slashes.
        raw_id.replace('\\', "/")
    }

    /// Extract just the feature name (file stem) from a feature ID
    fn get_feature_name_from_id(feature_id: &str) -> String {
        std::path::Path::new(feature_id)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(feature_id)
            .to_string()
    }

    /// Validate that a feature file is in a valid top-level directory.
    ///
    /// This catches misconfigurations early (e.g., typos like `shares/` instead of `shared/`,
    /// or incorrect paths like `rust/` instead of `core/`). Without this validation,
    /// features in unknown directories would be silently ignored during orphan detection,
    /// leading to false positives.
    ///
    /// Uses `TestDiscovery::get_language_from_path()` as the single source of truth for
    /// valid language-specific folders, plus explicit handling for `shared/`.
    fn validate_feature_prefix(&self, feature_path: &Path, feature_id: &str) -> Result<()> {
        let first_component = feature_id.split('/').next().unwrap_or("");

        // shared/ is valid for all languages
        if first_component == "shared" {
            return Ok(());
        }

        // Check if it's a known language folder using existing detection logic
        // (get_language_from_path returns Some for core/, python/, odbc/, jdbc/, csharp/, javascript/)
        if TestDiscovery::get_language_from_path(feature_path).is_some() {
            return Ok(());
        }

        // Unknown folder - error
        anyhow::bail!(
            "Feature file '{}' is in an invalid directory '{}'. \
             Feature files must be under 'shared/' or a language-specific folder \
             (core/, python/, odbc/, jdbc/, csharp/, javascript/).",
            feature_path.display(),
            first_component,
        );
    }

    fn collect_all_scenarios_and_languages(
        &self,
    ) -> Result<(
        Vec<(String, String)>,
        std::collections::HashMap<String, Vec<Language>>,
        std::collections::HashMap<(String, String), Vec<Language>>,
    )> {
        let mut scenarios = Vec::new();
        let mut feature_language_requirements: std::collections::HashMap<String, Vec<Language>> =
            std::collections::HashMap::new();
        let mut scenario_language_requirements: std::collections::HashMap<
            (String, String),
            Vec<Language>,
        > = std::collections::HashMap::new();

        // Walk through all .feature files
        for entry in WalkDir::new(&self.features_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "feature"))
        {
            let feature_path = entry.path();
            let feature = Feature::parse_from_file(feature_path)?;
            // Use unique feature ID (relative path) instead of just file stem
            let feature_id = self.get_feature_id(feature_path);

            // Validate feature is in a known directory structure
            self.validate_feature_prefix(feature_path, &feature_id)?;

            // Get generic languages declared at feature level
            let mut feature_declared_languages =
                TestDiscovery::get_generic_languages(&feature.tags);
            // Also check if feature is in a language-specific folder
            if let Some(folder_lang) = TestDiscovery::get_language_from_path(feature_path) {
                if !feature_declared_languages.contains(&folder_lang) {
                    feature_declared_languages.push(folder_lang);
                }
            }
            let feature_excluded = TestDiscovery::get_excluded_languages(&feature.tags);
            let mut required_languages = std::collections::HashSet::new();

            for scenario in &feature.scenarios {
                scenarios.push((feature_id.clone(), scenario.name.clone()));

                // Collect languages required by this scenario
                let scenario_excluded = TestDiscovery::get_excluded_languages(&scenario.tags);
                let scenario_languages = TestDiscovery::get_target_languages(&scenario.tags);

                let mut scenario_required_languages = Vec::new();
                for language in scenario_languages {
                    // Language is required if:
                    // 1. Feature has generic tag for this language (e.g., @core, @python)
                    // 2. Not excluded at feature or scenario level
                    if feature_declared_languages.contains(&language)
                        && !feature_excluded.contains(&language)
                        && !scenario_excluded.contains(&language)
                    {
                        required_languages.insert(language.clone());
                        scenario_required_languages.push(language);
                    }
                }

                // Store languages required by this specific scenario
                scenario_language_requirements.insert(
                    (feature_id.clone(), scenario.name.clone()),
                    scenario_required_languages,
                );
            }

            // Store required languages for this feature
            feature_language_requirements
                .insert(feature_id, required_languages.into_iter().collect());
        }

        Ok((
            scenarios,
            feature_language_requirements,
            scenario_language_requirements,
        ))
    }

    fn find_orphaned_files_for_language(
        &self,
        language: &Language,
        all_scenarios: &[(String, String)],
        feature_language_requirements: &std::collections::HashMap<String, Vec<Language>>,
        scenario_language_requirements: &std::collections::HashMap<(String, String), Vec<Language>>,
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
                let orphaned_methods = self.find_orphaned_methods_in_file(
                    test_file_path,
                    language,
                    all_scenarios,
                    scenario_language_requirements,
                )?;

                let file_name = test_file_path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();

                // Find ALL features that match this test file name
                // (language relevance is determined by tags in feature_language_requirements)
                let mut matching_feature_ids: Vec<&String> = all_scenarios
                    .iter()
                    .filter(|(feature_id, _)| {
                        let feature_name = Self::get_feature_name_from_id(feature_id);
                        self.file_name_matches_feature(&file_name, &feature_name)
                    })
                    .map(|(feature_id, _)| feature_id)
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                // Sort for deterministic ordering: prefer language-specific folders over shared/
                // (non-shared sorts before shared alphabetically, then by full path)
                matching_feature_ids.sort_by(|a, b| {
                    let a_shared = a.starts_with("shared/");
                    let b_shared = b.starts_with("shared/");
                    match (a_shared, b_shared) {
                        (false, true) => std::cmp::Ordering::Less,
                        (true, false) => std::cmp::Ordering::Greater,
                        _ => a.cmp(b),
                    }
                });

                if !matching_feature_ids.is_empty() {
                    // Check if ANY of the matching features require this language
                    let any_feature_requires_language = matching_feature_ids.iter().any(|fid| {
                        feature_language_requirements
                            .get(*fid)
                            .map(|langs| langs.contains(language))
                            .unwrap_or(false)
                    });

                    if !any_feature_requires_language {
                        // No matching feature requires this language - determine why
                        // Use the first matching feature (language-specific preferred over shared)
                        let reason =
                            self.determine_orphan_reason(matching_feature_ids[0], language)?;

                        orphaned_files.push(OrphanedTestFile {
                            file_path: test_file_path.to_path_buf(),
                            orphaned_methods: vec![],
                            reason,
                        });
                    } else if !orphaned_methods.is_empty() {
                        // File matches feature AND feature requires language, but has orphaned methods
                        orphaned_files.push(OrphanedTestFile {
                            file_path: test_file_path.to_path_buf(),
                            orphaned_methods,
                            reason: OrphanReason::MethodsWithoutScenarioTags,
                        });
                    }
                } else {
                    // File doesn't match any relevant feature
                    orphaned_files.push(OrphanedTestFile {
                        file_path: test_file_path.to_path_buf(),
                        orphaned_methods: vec![],
                        reason: OrphanReason::NoMatchingFeature,
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
            || file_name == "__init__.py"
            || file_name == "conftest.py"
            || file_name.ends_with("_common.rs")
            || file_name.ends_with("_helper.rs")
            || file_name.ends_with("_helpers.rs")
            || file_name.ends_with("_utils.rs")
            || file_name.ends_with("_common.py")
            || file_name.ends_with("_helper.py")
            || file_name.ends_with("_helpers.py")
            || file_name.ends_with("_utils.py")
            || file_name == "utils.py"
            || file_name == "compatibility.py"
            || file_name == "connector_factory.py"
            || file_name == "connector_types.py"
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
                    .join("jdbc/src/test/java/net/snowflake/jdbc/e2e"),
            ],
            Language::Odbc => vec![self._workspace_root.join("odbc_tests/tests/e2e")],
            Language::Python => vec![self._workspace_root.join("python/tests/e2e")],
            _ => vec![],
        }
    }

    fn is_test_file_for_language(&self, file_path: &Path, language: &Language) -> bool {
        if let Some(extension) = file_path.extension() {
            match language {
                Language::Rust => extension == "rs",
                Language::Jdbc => extension == "java",
                Language::Odbc => extension == "cpp",
                Language::Python => extension == "py",
                _ => false,
            }
        } else {
            false
        }
    }

    fn file_name_matches_feature(&self, file_name: &str, feature_name: &str) -> bool {
        use crate::utils::{strings_match_normalized, to_pascal_case, to_snake_case};

        // Remove common test prefixes and suffixes
        let clean_file_name = file_name
            .trim_start_matches("test_") // Python: test_feature_name.py
            .trim_end_matches("Test") // JDBC: FeatureNameTest.java
            .trim_end_matches("Tests") // JDBC: FeatureNameTests.java
            .trim_end_matches("_test") // Rust: feature_name_test.rs
            .trim_end_matches("_tests"); // Rust: feature_name_tests.rs

        strings_match_normalized(clean_file_name, feature_name)
            || strings_match_normalized(clean_file_name, &to_pascal_case(feature_name))
            || strings_match_normalized(clean_file_name, &to_snake_case(feature_name))
    }

    fn find_orphaned_methods_in_file(
        &self,
        file_path: &Path,
        language: &Language,
        all_scenarios: &[(String, String)],
        scenario_language_requirements: &std::collections::HashMap<(String, String), Vec<Language>>,
    ) -> Result<Vec<String>> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read test file: {}", file_path.display()))?;

        let mut orphaned_methods = Vec::new();

        // Get all test methods in this file
        let all_methods = self.get_all_test_methods_in_file(&content, language)?;

        // Determine which feature this test file corresponds to
        let file_name = file_path.file_stem().unwrap().to_str().unwrap().to_string();

        // Find ALL matching features by name
        // (language relevance is determined by tags in scenario_language_requirements)
        let matching_feature_ids: Vec<&String> = all_scenarios
            .iter()
            .filter(|(feature_id, _)| {
                let feature_name = Self::get_feature_name_from_id(feature_id);
                self.file_name_matches_feature(&file_name, &feature_name)
            })
            .map(|(feature_id, _)| feature_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        for method_name in all_methods {
            // Check if method matches a scenario in ANY of the matching features that requires this language
            let method_matches_valid_scenario = matching_feature_ids.iter().any(|feature_id| {
                all_scenarios
                    .iter()
                    .filter(|(fid, _)| fid == *feature_id)
                    .any(|(fid, scenario_name)| {
                        if self.method_name_matches_scenario(&method_name, scenario_name) {
                            // Method name matches, check if scenario requires this language
                            scenario_language_requirements
                                .get(&(fid.clone(), scenario_name.clone()))
                                .map(|langs| langs.contains(language))
                                .unwrap_or(false)
                        } else {
                            false
                        }
                    })
            });

            if !method_matches_valid_scenario {
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
                let test_regex = Regex::new(
                    r"@(?:Test|ParameterizedTest)\b(?:\s*\n\s*@\w+(?:\([^)]*\))?)*\s*\n\s*(?:public|protected|private)?\s*(?:static\s+)?(?:void|Task(?:<[^>]+>)?)\s+(\w+)\s*\(",
                )?;
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
            Language::Python => {
                // Match pytest test functions: def test_something(...):
                let test_regex = Regex::new(r"def\s+(test_\w+)\s*\(")?;
                for captures in test_regex.captures_iter(content) {
                    methods.push(captures[1].to_string());
                }
            }
            _ => {}
        }

        // Remove duplicates (e.g., if file has duplicate test method names)
        methods.sort();
        methods.dedup();

        Ok(methods)
    }

    fn method_name_matches_scenario(&self, method_name: &str, scenario_name: &str) -> bool {
        use crate::utils::{
            clean_method_name, strings_match_normalized, to_pascal_case, to_snake_case,
        };

        let clean = clean_method_name(method_name);

        strings_match_normalized(clean, scenario_name)
            || strings_match_normalized(clean, &to_pascal_case(scenario_name))
            || strings_match_normalized(clean, &to_snake_case(scenario_name))
    }

    fn determine_orphan_reason(
        &self,
        feature_id: &str,
        language: &Language,
    ) -> Result<OrphanReason> {
        // Find the feature file using the feature ID (relative path)
        let feature_path = self.find_feature_file_by_id(feature_id)?;
        let feature = Feature::parse_from_file(&feature_path)?;

        // Check if feature has generic language tag OR is in a language-specific folder for this language
        let feature_generic_languages = TestDiscovery::get_generic_languages(&feature.tags);
        let folder_language = TestDiscovery::get_language_from_path(&feature_path);
        let has_generic_tag = feature_generic_languages.contains(language)
            || folder_language.as_ref() == Some(language);

        // Check if language is explicitly excluded (e.g., @python_not_needed)
        let feature_excluded = TestDiscovery::get_excluded_languages(&feature.tags);
        let is_excluded = feature_excluded.contains(language);

        // Check if ANY scenario has level tags for this language
        let mut scenarios_have_level_tags = false;
        for scenario in &feature.scenarios {
            let scenario_languages = TestDiscovery::get_target_languages(&scenario.tags);
            if scenario_languages.contains(language) {
                scenarios_have_level_tags = true;
                break;
            }
        }

        Ok(if is_excluded {
            // Language is explicitly marked as not needed
            OrphanReason::LanguageMarkedAsNotNeeded
        } else if scenarios_have_level_tags && !has_generic_tag {
            // Scenarios have @core_e2e but feature is missing @core
            OrphanReason::FeatureMissingGenericLanguageTag
        } else {
            // Feature exists but scenarios don't have level tags
            OrphanReason::FeatureExistsButNoScenarioTags
        })
    }

    /// Find a feature file by its ID (relative path without .feature extension)
    fn find_feature_file_by_id(&self, feature_id: &str) -> Result<PathBuf> {
        // Feature ID is the relative path, so we can reconstruct the full path
        let feature_path = self.features_dir.join(format!("{}.feature", feature_id));
        if feature_path.exists() {
            return Ok(feature_path);
        }

        // Fallback: search by file stem (for backward compatibility)
        let feature_name = Self::get_feature_name_from_id(feature_id);
        self.find_feature_file(&feature_name)
    }

    fn find_feature_file(&self, feature_name: &str) -> Result<PathBuf> {
        use walkdir::WalkDir;

        for entry in WalkDir::new(&self.features_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "feature"))
        {
            let path = entry.path();
            let file_stem = path.file_stem().unwrap().to_str().unwrap();
            if file_stem == feature_name {
                return Ok(path.to_path_buf());
            }
        }

        Err(anyhow::anyhow!("Feature file not found: {}", feature_name))
    }

    pub fn validate_feature_with_path(
        &self,
        feature: &Feature,
        feature_path: &Path,
    ) -> Result<ValidationResult> {
        // Feature-level tags can be:
        // 1. Generic language tags (e.g., @core, @python) - indicate planned implementations
        // 2. Exclusion tags (e.g., @core_not_needed) - exclude languages
        // BUT NOT level-specific tags (e.g., @core_e2e, @core_int) - those must be at scenario level

        let mut tag_errors = Vec::new();

        // Check if feature is in a language-specific folder
        let folder_language = TestDiscovery::get_language_from_path(feature_path);

        if let Some(only_lang) = &folder_language {
            // This is a language-specific feature - validate that all tags match the folder language
            let only_lang_name = match only_lang {
                Language::Rust => "core",
                Language::Python => "python",
                Language::Jdbc => "jdbc",
                Language::Odbc => "odbc",
                _ => "language",
            };

            // Check feature-level tags
            let feature_generic_languages = TestDiscovery::get_generic_languages(&feature.tags);
            for lang in &feature_generic_languages {
                if lang != only_lang {
                    let lang_name = match lang {
                        Language::Rust => "core",
                        Language::Python => "python",
                        Language::Jdbc => "jdbc",
                        Language::Odbc => "odbc",
                        _ => "language",
                    };
                    tag_errors.push(format!(
                        "VALIDATION ERROR: Feature is in {0}/ folder but has @{1} tag. Only @{0} tag should be used in language-specific folders.",
                        only_lang_name, lang_name
                    ));
                }
            }

            // Check scenario-level tags
            for scenario in &feature.scenarios {
                let scenario_languages = TestDiscovery::get_target_languages(&scenario.tags);
                for lang in scenario_languages {
                    if &lang != only_lang {
                        let lang_name = match lang {
                            Language::Rust => "core",
                            Language::Python => "python",
                            Language::Jdbc => "jdbc",
                            Language::Odbc => "odbc",
                            _ => "language",
                        };
                        tag_errors.push(format!(
                            "VALIDATION ERROR: Scenario '{}' is in {}/ folder but has @{}_e2e or @{}_int tags. Only @{}_e2e or @{}_int tags should be used.",
                            scenario.name, only_lang_name, lang_name, lang_name, only_lang_name, only_lang_name
                        ));
                    }
                }
            }
        }

        for tag in &feature.tags {
            // Check if tag has level suffix (_e2e or _int)
            if tag.ends_with("_e2e") || tag.ends_with("_int") {
                tag_errors.push(format!(
                    "VALIDATION ERROR: Invalid feature-level tag '@{}'. Feature-level tags cannot specify test level (_e2e/_int). Use scenario-level tags to specify test levels.",
                    tag
                ));
            }
            // _not_needed and generic language tags are allowed
        }

        // Get generic languages declared at feature level
        let feature_declared_languages = TestDiscovery::get_generic_languages(&feature.tags);
        let feature_excluded = TestDiscovery::get_excluded_languages(&feature.tags);
        let mut language_set = std::collections::HashSet::new();

        // If feature is in a language-specific folder, only validate that language
        // BUT only if there are scenarios with implementation tags for it
        if let Some(only_lang) = &folder_language {
            // Check if any scenario has implementation tags for this language
            let has_implementation_tags = feature.scenarios.iter().any(|scenario| {
                let scenario_languages = TestDiscovery::get_target_languages(&scenario.tags);
                scenario_languages.contains(only_lang)
            });
            if has_implementation_tags {
                language_set.insert(only_lang.clone());
            }
        } else {
            // Collect all unique languages from scenario tags
            // BUT only if the feature declares that language at feature level
            // ALSO validate that scenarios don't have tags for languages marked as not_needed at feature level
            for scenario in &feature.scenarios {
                let scenario_excluded = TestDiscovery::get_excluded_languages(&scenario.tags);
                let scenario_languages = TestDiscovery::get_target_languages(&scenario.tags);

                // Check if scenario has tags for languages that are marked as not_needed at feature level
                for language in &scenario_languages {
                    if feature_excluded.contains(language) {
                        let lang_tag = match language {
                            Language::Rust => "core",
                            Language::Python => "python",
                            Language::Jdbc => "jdbc",
                            Language::Odbc => "odbc",
                            _ => "language",
                        };
                        tag_errors.push(format!(
                            "VALIDATION ERROR: Scenario '{}' has @{} tags but feature has @{}_not_needed. Remove scenario-level tags for excluded languages.",
                            scenario.name, lang_tag, lang_tag
                        ));
                    }
                }

                for language in scenario_languages {
                    // Language is validated if:
                    // 1. Feature has generic tag for this language (e.g., @core, @python)
                    // 2. Not excluded at feature or scenario level
                    if feature_declared_languages.contains(&language)
                        && !feature_excluded.contains(&language)
                        && !scenario_excluded.contains(&language)
                    {
                        language_set.insert(language);
                    }
                }
            }
        }

        // Check if feature declares languages but scenarios don't have tags for them
        // Skip this check for language-specific folder features as they're validated differently
        let mut missing_scenario_tags_errors = Vec::new();
        if folder_language.is_none()
            && !feature_declared_languages.is_empty()
            && !feature.scenarios.is_empty()
        {
            // Check each declared language to see if scenarios have tags for it
            for language in &feature_declared_languages {
                if !feature_excluded.contains(language) && !language_set.contains(language) {
                    // Feature declares this language but no scenario has level tags for it
                    let lang_tag = match language {
                        Language::Rust => "core",
                        Language::Python => "python",
                        Language::Jdbc => "jdbc",
                        Language::Odbc => "odbc",
                        _ => "language",
                    };
                    missing_scenario_tags_errors.push(format!(
                        "VALIDATION ERROR: Feature has @{} tag but no scenarios have @{}_e2e or @{}_int tags. Add scenario-level tags to specify which test level to use.",
                        lang_tag, lang_tag, lang_tag
                    ));
                }
            }
        }

        // Validate each unique language
        let mut validations = Vec::new();
        for language in language_set {
            let mut validation =
                self.validate_language_implementation_with_path(feature, feature_path, language)?;

            // Add feature-level tag errors to first language validation as missing_steps (fails validation)
            if validations.is_empty() && !tag_errors.is_empty() {
                validation.missing_steps.extend(tag_errors.clone());
            }

            validations.push(validation);
        }

        // Add missing scenario tags errors to first validation, or create new one
        if !missing_scenario_tags_errors.is_empty() {
            if let Some(first_validation) = validations.first_mut() {
                first_validation
                    .missing_steps
                    .extend(missing_scenario_tags_errors);
            } else {
                // No validations at all, create one to show errors
                let mut all_errors = tag_errors;
                all_errors.extend(missing_scenario_tags_errors);
                validations.push(LanguageValidation {
                    language: Language::Rust, // Arbitrary choice for display
                    test_file_found: false,
                    test_file_path: None,
                    missing_steps: all_errors,
                    implemented_steps: Vec::new(),
                    warnings: Vec::new(),
                    missing_steps_by_method: Vec::new(),
                    empty_steps: Vec::new(),
                });
            }
        } else if validations.is_empty() && !tag_errors.is_empty() {
            // No validations and we have tag errors
            validations.push(LanguageValidation {
                language: Language::Rust, // Arbitrary choice for display
                test_file_found: false,
                test_file_path: None,
                missing_steps: tag_errors,
                implemented_steps: Vec::new(),
                warnings: Vec::new(),
                missing_steps_by_method: Vec::new(),
                empty_steps: Vec::new(),
            });
        }

        let scenario_structure_errors: Vec<String> = feature
            .scenarios
            .iter()
            .flat_map(|scenario| scenario.validate_mandatory_steps())
            .collect();

        Ok(ValidationResult {
            feature_file: feature.file_path.clone(),
            validations,
            scenario_structure_errors,
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
            let mut all_empty_steps = Vec::new();
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
                    // Determine the test level for this scenario (language-specific)
                    let test_level =
                        TestDiscovery::get_test_level_for_language(&scenario.tags, &language);

                    // Find the appropriate test file based on test level
                    let scenario_test_file = self.discovery.find_test_file_with_path_and_level(
                        feature_path,
                        &language,
                        test_level.clone(),
                    );

                    // Check if scenario has explicit level tag
                    let has_explicit_level_tag = scenario
                        .tags
                        .iter()
                        .any(|tag| tag.ends_with("_e2e") || tag.ends_with("_int"));

                    let actual_test_file_path = if let Some(ref scenario_file) = scenario_test_file
                    {
                        scenario_file
                    } else {
                        // If no file found at the required level, check if it's in wrong directory
                        if has_explicit_level_tag {
                            // Check if the test exists in the opposite directory
                            let opposite_level = match test_level {
                                TestLevel::E2E => TestLevel::Integration,
                                TestLevel::Integration => TestLevel::E2E,
                            };
                            let wrong_level_file =
                                self.discovery.find_test_file_with_path_and_level(
                                    feature_path,
                                    &language,
                                    opposite_level.clone(),
                                );

                            if let Some(ref wrong_file) = wrong_level_file {
                                // Check if the test method actually exists in the wrong directory file
                                let wrong_dir_methods = step_finder
                                    .find_test_methods_with_lines(wrong_file, &scenario.name)?;

                                if !wrong_dir_methods.is_empty() {
                                    // Test method exists in wrong directory - this is a validation error
                                    all_missing_steps.push(format!(
                                        "VALIDATION ERROR: Scenario '{}' is tagged with '{}' level but test found in '{}' directory: {}. Move test to {} directory.",
                                        scenario.name,
                                        test_level,
                                        opposite_level,
                                        wrong_file.display(),
                                        test_level
                                    ));
                                    // Don't process this scenario further
                                    continue;
                                } else {
                                    // File exists at wrong level but method doesn't - report as missing
                                    warnings.push(format!(
                                        "No test method found for scenario: {} (expected in {} directory)",
                                        scenario.name,
                                        test_level
                                    ));
                                    continue;
                                }
                            } else {
                                warnings.push(format!(
                                    "No test method found for scenario: {} (expected in {} directory)",
                                    scenario.name,
                                    test_level
                                ));
                                continue;
                            }
                        } else {
                            // No explicit level tag, fall back to general test file
                            &test_file_path
                        }
                    };

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
                            let step_found = method_steps
                                .iter()
                                .any(|impl_step| self.steps_match(impl_step, step_text));
                            if !step_found {
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

                        // Check for empty steps (step comments with no implementation code)
                        let method_empty_steps = step_finder
                            .find_empty_steps_in_method(actual_test_file_path, &method_name)?;
                        for empty_step in &method_empty_steps {
                            if !all_empty_steps.contains(empty_step) {
                                all_empty_steps.push(empty_step.clone());
                            }
                        }

                        if !method_missing_steps.is_empty() || !method_empty_steps.is_empty() {
                            missing_steps_by_method.push(MethodValidation {
                                method_name: method_name.clone(),
                                scenario_name: scenario.name.clone(),
                                missing_steps: method_missing_steps,
                                empty_steps: method_empty_steps,
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
                empty_steps: all_empty_steps,
            })
        } else {
            Ok(LanguageValidation {
                language,
                test_file_found: false,
                test_file_path: None,
                missing_steps: Vec::new(),
                implemented_steps: Vec::new(),
                warnings: vec![format!("No test file found for feature: {}", feature.name)],
                missing_steps_by_method: Vec::new(),
                empty_steps: Vec::new(),
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
                let behavior_difference_scenarios: Vec<String> = feature
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

                let feature_id = self.get_feature_id(entry.path());
                features
                    .entry(feature_id)
                    .or_insert_with(|| crate::behavior_differences_processor::FeatureInfo {
                        behavior_difference_scenarios: Vec::new(),
                    })
                    .behavior_difference_scenarios
                    .extend(behavior_difference_scenarios);
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
