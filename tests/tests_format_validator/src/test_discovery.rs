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
                "core" => languages.push(Language::Rust),
                "odbc" => languages.push(Language::Odbc),
                "jdbc" => languages.push(Language::Jdbc),
                "python" | "pep249" => languages.push(Language::Python),
                "csharp" | "dotnet" => languages.push(Language::CSharp),
                "javascript" | "nodejs" | "js" => languages.push(Language::JavaScript),
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

    /// Extract subdirectory from feature path relative to e2e root
    fn extract_feature_subdir(&self, feature_path: &Path) -> Option<String> {
        // Try to find the e2e directory in the path and extract the subdirectory
        let path_components: Vec<&str> = feature_path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        // Look for "e2e" in the path and get the next component
        for (i, component) in path_components.iter().enumerate() {
            if *component == "e2e" && i + 1 < path_components.len() {
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
        let snake_name = to_snake_case(feature_name);
        let pascal_name = to_pascal_case(feature_name);

        match language {
            Language::Rust => {
                let base_path = if let Some(subdir) = subdir {
                    self.workspace_root.join("sf_core/tests/e2e").join(subdir)
                } else {
                    self.workspace_root.join("sf_core/tests/e2e")
                };

                vec![
                    // sf_core/tests/e2e/[subdir/]feature_name.rs
                    base_path.join(format!("{}.rs", snake_name)),
                    // sf_core/tests/e2e/[subdir/]feature_name_tests.rs
                    base_path.join(format!("{}_tests.rs", snake_name)),
                    // sf_core/tests/e2e/[subdir/]feature_name_test.rs
                    base_path.join(format!("{}_test.rs", snake_name)),
                    // Fallback to old location (no subdir)
                    self.workspace_root
                        .join("sf_core/tests/e2e")
                        .join(format!("{}.rs", snake_name)),
                ]
            }
            Language::Odbc => {
                let base_path = if let Some(subdir) = subdir {
                    self.workspace_root
                        .join("odbc_tests/tests/e2e")
                        .join(subdir)
                } else {
                    self.workspace_root.join("odbc_tests/tests/e2e")
                };

                vec![
                    // odbc_tests/tests/e2e/[subdir/]feature_name.cpp
                    base_path.join(format!("{}.cpp", snake_name)),
                    // odbc_tests/tests/e2e/[subdir/]feature_name_tests.cpp
                    base_path.join(format!("{}_tests.cpp", snake_name)),
                    // Fallback to old locations
                    self.workspace_root
                        .join("odbc/tests")
                        .join(format!("{}_tests.cpp", snake_name)),
                    self.workspace_root
                        .join("odbc_tests/tests")
                        .join(format!("{}.cpp", snake_name)),
                    self.workspace_root
                        .join("odbc_tests/tests")
                        .join(format!("{}_tests.cpp", snake_name)),
                ]
            }
            Language::Jdbc => {
                let base_path = if let Some(subdir) = subdir {
                    self.workspace_root
                        .join("jdbc/src/test/java/com/snowflake/jdbc/e2e")
                        .join(subdir)
                } else {
                    self.workspace_root
                        .join("jdbc/src/test/java/com/snowflake/jdbc/e2e")
                };

                let simple_base_path = if let Some(subdir) = subdir {
                    self.workspace_root
                        .join("jdbc/src/test/java/e2e")
                        .join(subdir)
                } else {
                    self.workspace_root.join("jdbc/src/test/java/e2e")
                };

                vec![
                    // Test workspace paths (for unit tests)
                    simple_base_path.join(format!("{}Test.java", pascal_name)),
                    simple_base_path.join(format!("{}Tests.java", pascal_name)),
                    // jdbc/src/test/java/com/snowflake/jdbc/e2e/[subdir/]FeatureNameTest.java
                    base_path.join(format!("{}Test.java", pascal_name)),
                    // jdbc/src/test/java/com/snowflake/jdbc/e2e/[subdir/]FeatureNameTests.java
                    base_path.join(format!("{}Tests.java", pascal_name)),
                    // Fallback to old location (no subdir)
                    self.workspace_root
                        .join("jdbc/src/test/java/com/snowflake/jdbc/e2e")
                        .join(format!("{}Test.java", pascal_name)),
                    self.workspace_root
                        .join("jdbc/src/test/java/com/snowflake/jdbc/e2e")
                        .join(format!("{}Tests.java", pascal_name)),
                    // Fallback to older location
                    self.workspace_root
                        .join("jdbc/src/test/java/com/snowflake/jdbc")
                        .join(format!("{}Test.java", pascal_name)),
                    self.workspace_root
                        .join("jdbc/src/test/java/com/snowflake/jdbc")
                        .join(format!("{}Tests.java", pascal_name)),
                ]
            }
            Language::Python => {
                let base_path = if let Some(subdir) = subdir {
                    self.workspace_root
                        .join("pep249_dbapi/tests/e2e")
                        .join(subdir)
                } else {
                    self.workspace_root.join("pep249_dbapi/tests/e2e")
                };

                vec![
                    // pep249_dbapi/tests/e2e/[subdir/]test_feature_name.py (pytest convention)
                    base_path.join(format!("test_{}.py", snake_name)),
                    // pep249_dbapi/tests/e2e/[subdir/]feature_name.py (legacy)
                    base_path.join(format!("{}.py", snake_name)),
                    // pep249_dbapi/tests/integ/test_feature_name.py (fallback)
                    self.workspace_root
                        .join("pep249_dbapi/tests/integ")
                        .join(format!("test_{}.py", snake_name)),
                    // pep249_dbapi/tests/test_feature_name.py (fallback)
                    self.workspace_root
                        .join("pep249_dbapi/tests")
                        .join(format!("test_{}.py", snake_name)),
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
