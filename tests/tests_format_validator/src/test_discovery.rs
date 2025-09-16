use crate::utils::{to_pascal_case, to_snake_case};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Odbc,
    Jdbc,
    Python,
    CSharp,
    JavaScript,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestLevel {
    E2E,
    Integration,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "Rust"),
            Language::Odbc => write!(f, "Odbc"),
            Language::Jdbc => write!(f, "Jdbc"),
            Language::Python => write!(f, "Python"),
            Language::CSharp => write!(f, "CSharp"),
            Language::JavaScript => write!(f, "JavaScript"),
        }
    }
}

pub struct TestDiscovery {
    workspace_root: PathBuf,
}

impl TestDiscovery {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    /// Map feature tags to target languages
    pub fn get_target_languages(tags: &[String]) -> Vec<Language> {
        let mut languages = Vec::new();

        for tag in tags {
            match tag.as_str() {
                "core" | "core_e2e" | "core_int" => languages.push(Language::Rust),
                "odbc" | "odbc_e2e" | "odbc_int" => languages.push(Language::Odbc),
                "jdbc" | "jdbc_e2e" | "jdbc_int" => languages.push(Language::Jdbc),
                "python" | "python_e2e" | "python_int" | "pep249" => {
                    languages.push(Language::Python)
                }
                "csharp" | "csharp_e2e" | "csharp_int" | "dotnet" => {
                    languages.push(Language::CSharp)
                }
                "javascript" | "javascript_e2e" | "javascript_int" | "nodejs" | "js" => {
                    languages.push(Language::JavaScript)
                }
                // Note: _not_needed tags are NOT included here - they explicitly exclude tests
                // Default behavior: if feature has driver tag but scenario doesn't, it's TODO
                _ => {} // Unknown tag, ignore
            }
        }

        // Remove duplicates
        languages.sort_by_key(|l| format!("{:?}", l));
        languages.dedup();

        languages
    }

    /// Determine test level (e2e or integration) based on tags
    pub fn get_test_level(tags: &[String]) -> TestLevel {
        for tag in tags {
            if tag.ends_with("_e2e")
                || tag == "core_e2e"
                || tag == "odbc_e2e"
                || tag == "jdbc_e2e"
                || tag == "python_e2e"
                || tag == "csharp_e2e"
                || tag == "javascript_e2e"
            {
                return TestLevel::E2E;
            }
            if tag.ends_with("_int")
                || tag == "core_int"
                || tag == "odbc_int"
                || tag == "jdbc_int"
                || tag == "python_int"
                || tag == "csharp_int"
                || tag == "javascript_int"
            {
                return TestLevel::Integration;
            }
        }
        // Default to e2e if no specific level tag found
        TestLevel::E2E
    }

    /// Find test file for a given feature path and language (includes subdirectory structure)
    pub fn find_test_file_with_path(
        &self,
        feature_path: &Path,
        language: &Language,
    ) -> Option<PathBuf> {
        let feature_name = feature_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Extract subdirectory from feature path (e.g., "auth", "query")
        let subdir = self.extract_feature_subdir(feature_path);

        let candidates =
            self.generate_test_file_candidates(feature_name, subdir.as_deref(), language);

        candidates.into_iter().find(|candidate| candidate.exists())
    }

    /// Find test file for a given feature path, language, and test level
    pub fn find_test_file_with_path_and_level(
        &self,
        feature_path: &Path,
        language: &Language,
        test_level: TestLevel,
    ) -> Option<PathBuf> {
        let feature_name = feature_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Extract subdirectory from feature path (e.g., "auth", "query")
        let subdir = self.extract_feature_subdir(feature_path);

        let candidates = self.generate_test_file_candidates_with_level(
            feature_name,
            subdir.as_deref(),
            language,
            test_level,
        );

        candidates.into_iter().find(|candidate| candidate.exists())
    }

    /// Extract subdirectory from feature path relative to definitions root
    fn extract_feature_subdir(&self, feature_path: &Path) -> Option<String> {
        // Try to find the definitions directory in the path and extract the subdirectory
        let path_components: Vec<&str> = feature_path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        // Look for "definitions" in the path and get the next component
        for (i, component) in path_components.iter().enumerate() {
            if *component == "definitions" && i + 1 < path_components.len() {
                let subdir = path_components[i + 1];
                if subdir
                    != feature_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                {
                    return Some(subdir.to_string());
                }
            }
        }
        None
    }

    fn generate_test_file_candidates(
        &self,
        feature_name: &str,
        subdir: Option<&str>,
        language: &Language,
    ) -> Vec<PathBuf> {
        let mut candidates = Vec::new();

        // Try both e2e and integration directories
        candidates.extend(self.generate_test_file_candidates_with_level(
            feature_name,
            subdir,
            language,
            TestLevel::E2E,
        ));
        candidates.extend(self.generate_test_file_candidates_with_level(
            feature_name,
            subdir,
            language,
            TestLevel::Integration,
        ));

        candidates
    }

    fn generate_test_file_candidates_with_level(
        &self,
        feature_name: &str,
        subdir: Option<&str>,
        language: &Language,
        test_level: TestLevel,
    ) -> Vec<PathBuf> {
        let snake_name = to_snake_case(feature_name);
        let pascal_name = to_pascal_case(feature_name);

        match language {
            Language::Rust => {
                let test_dir = match test_level {
                    TestLevel::E2E => "e2e",
                    TestLevel::Integration => "integration",
                };

                let base_path = if let Some(subdir) = subdir {
                    self.workspace_root
                        .join(format!("sf_core/tests/{}", test_dir))
                        .join(subdir)
                } else {
                    self.workspace_root
                        .join(format!("sf_core/tests/{}", test_dir))
                };

                vec![
                    // sf_core/tests/[e2e|integration]/[subdir/]feature_name.rs
                    base_path.join(format!("{}.rs", snake_name)),
                    // sf_core/tests/[e2e|integration]/[subdir/]feature_name_tests.rs
                    base_path.join(format!("{}_tests.rs", snake_name)),
                    // sf_core/tests/[e2e|integration]/[subdir/]feature_name_test.rs
                    base_path.join(format!("{}_test.rs", snake_name)),
                ]
            }
            Language::Odbc => {
                let test_dir = match test_level {
                    TestLevel::E2E => "e2e",
                    TestLevel::Integration => "integration",
                };

                let base_path = if let Some(subdir) = subdir {
                    self.workspace_root
                        .join(format!("odbc_tests/tests/{}", test_dir))
                        .join(subdir)
                } else {
                    self.workspace_root
                        .join(format!("odbc_tests/tests/{}", test_dir))
                };

                vec![
                    // odbc_tests/tests/[e2e|integration]/[subdir/]feature_name.cpp
                    base_path.join(format!("{}.cpp", snake_name)),
                    // odbc_tests/tests/[e2e|integration]/[subdir/]feature_name_tests.cpp
                    base_path.join(format!("{}_tests.cpp", snake_name)),
                ]
            }
            Language::Jdbc => {
                let test_dir = match test_level {
                    TestLevel::E2E => "e2e",
                    TestLevel::Integration => "integration",
                };

                let base_path = if let Some(subdir) = subdir {
                    self.workspace_root
                        .join(format!(
                            "jdbc/src/test/java/com/snowflake/jdbc/{}",
                            test_dir
                        ))
                        .join(subdir)
                } else {
                    self.workspace_root.join(format!(
                        "jdbc/src/test/java/com/snowflake/jdbc/{}",
                        test_dir
                    ))
                };

                let simple_base_path = if let Some(subdir) = subdir {
                    self.workspace_root
                        .join(format!("jdbc/src/test/java/{}", test_dir))
                        .join(subdir)
                } else {
                    self.workspace_root
                        .join(format!("jdbc/src/test/java/{}", test_dir))
                };

                vec![
                    // Test workspace paths (for unit tests)
                    simple_base_path.join(format!("{}Test.java", pascal_name)),
                    simple_base_path.join(format!("{}Tests.java", pascal_name)),
                    // jdbc/src/test/java/com/snowflake/jdbc/[e2e|integration]/[subdir/]FeatureNameTest.java
                    base_path.join(format!("{}Test.java", pascal_name)),
                    // jdbc/src/test/java/com/snowflake/jdbc/[e2e|integration]/[subdir/]FeatureNameTests.java
                    base_path.join(format!("{}Tests.java", pascal_name)),
                ]
            }
            Language::Python => {
                let test_dir = match test_level {
                    TestLevel::E2E => "e2e",
                    TestLevel::Integration => "integ", // Python uses "integ" not "integration"
                };

                let base_path = if let Some(subdir) = subdir {
                    self.workspace_root
                        .join(format!("pep249_dbapi/tests/{}", test_dir))
                        .join(subdir)
                } else {
                    self.workspace_root
                        .join(format!("pep249_dbapi/tests/{}", test_dir))
                };

                vec![
                    // pep249_dbapi/tests/[e2e|integ]/[subdir/]test_feature_name.py (pytest convention)
                    base_path.join(format!("test_{}.py", snake_name)),
                    // pep249_dbapi/tests/[e2e|integ]/[subdir/]feature_name.py (legacy)
                    base_path.join(format!("{}.py", snake_name)),
                ]
            }
            Language::CSharp => vec![
                // Add C# test paths when needed
                self.workspace_root
                    .join("dotnet/tests")
                    .join(format!("{}Test.cs", pascal_name)),
            ],
            Language::JavaScript => vec![
                // Add JavaScript test paths when needed
                self.workspace_root
                    .join("nodejs/tests")
                    .join(format!("{}.test.js", snake_name)),
            ],
        }
    }
}
