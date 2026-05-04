use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::feature_parser::Feature;
use crate::step_finder::StepFinder;
use crate::test_discovery::Language;
use crate::utils::{clean_method_name, strings_match_normalized, to_snake_case};

// ---------------------------------------------------------------------------
// Config types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AlignmentConfig {
    pub target: Vec<AlignmentTarget>,
}

#[derive(Debug, Deserialize)]
pub struct AlignmentTarget {
    pub name: String,
    pub language: String,
    pub features: String,
    pub tests: String,
}

impl AlignmentTarget {
    fn language_enum(&self) -> Result<Language> {
        match self.language.as_str() {
            "python" => Ok(Language::Python),
            "rust" => Ok(Language::Rust),
            "jdbc" => Ok(Language::Jdbc),
            "odbc" => Ok(Language::Odbc),
            other => anyhow::bail!("unsupported target language: {other}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Validation result types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct AlignmentValidationResult {
    pub target_name: String,
    pub pair_results: Vec<PairResult>,
    pub orphan_test_files: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct PairResult {
    pub feature_file: PathBuf,
    pub test_file: PathBuf,
    pub missing_methods: Vec<String>,
    pub orphan_methods: Vec<String>,
    pub missing_steps_by_method: Vec<MethodStepResult>,
    pub empty_steps_by_method: Vec<MethodStepResult>,
}

#[derive(Debug)]
pub struct MethodStepResult {
    pub method_name: String,
    pub scenario_name: String,
    pub issues: Vec<String>,
}

impl AlignmentValidationResult {
    pub fn has_issues(&self) -> bool {
        !self.orphan_test_files.is_empty()
            || self.pair_results.iter().any(|pr| {
                !pr.missing_methods.is_empty()
                    || !pr.orphan_methods.is_empty()
                    || !pr.missing_steps_by_method.is_empty()
                    || !pr.empty_steps_by_method.is_empty()
            })
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn load_config(workspace_root: &Path) -> Result<Option<AlignmentConfig>> {
    let config_path = workspace_root.join("tests/tests_format_validator/alignment_targets.toml");
    if !config_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let config: AlignmentConfig =
        toml::from_str(&content).with_context(|| "failed to parse alignment_targets.toml")?;
    Ok(Some(config))
}

pub fn validate_alignment_targets(
    workspace_root: &Path,
    config: &AlignmentConfig,
) -> Result<Vec<AlignmentValidationResult>> {
    let mut results = Vec::new();
    for target in &config.target {
        results.push(validate_target(workspace_root, target)?);
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// Per-target validation
// ---------------------------------------------------------------------------

fn validate_target(
    workspace_root: &Path,
    target: &AlignmentTarget,
) -> Result<AlignmentValidationResult> {
    let language = target.language_enum()?;
    let features_dir = workspace_root.join(&target.features);
    let tests_dir = workspace_root.join(&target.tests);

    anyhow::ensure!(
        features_dir.is_dir(),
        "target '{name}': features directory does not exist: {path}",
        name = target.name,
        path = features_dir.display(),
    );
    anyhow::ensure!(
        tests_dir.is_dir(),
        "target '{name}': tests directory does not exist: {path}",
        name = target.name,
        path = tests_dir.display(),
    );

    let step_finder = StepFinder::new(language.clone());

    let test_files = collect_test_files(&tests_dir, &language)?;
    let feature_files = collect_feature_files(&features_dir)?;

    let mut pair_results = Vec::new();
    let mut orphan_test_files = Vec::new();
    let mut matched_test_stems: std::collections::HashSet<String> =
        std::collections::HashSet::new();

    for test_path in &test_files {
        let test_stem = test_file_stem(test_path, &language);
        match feature_files.iter().find(|fp| {
            fp.file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s == test_stem)
        }) {
            Some(feature_path) => {
                matched_test_stems.insert(test_stem);
                let pr = validate_pair(&step_finder, feature_path, test_path, &language)?;
                pair_results.push(pr);
            }
            None => {
                orphan_test_files.push(test_path.clone());
            }
        }
    }

    Ok(AlignmentValidationResult {
        target_name: target.name.clone(),
        pair_results,
        orphan_test_files,
    })
}

fn validate_pair(
    step_finder: &StepFinder,
    feature_path: &Path,
    test_path: &Path,
    language: &Language,
) -> Result<PairResult> {
    let feature = Feature::parse_from_file(feature_path)
        .with_context(|| format!("failed to parse {}", feature_path.display()))?;

    let test_content = std::fs::read_to_string(test_path)
        .with_context(|| format!("failed to read {}", test_path.display()))?;

    let all_test_methods = collect_test_methods(&test_content, language)?;

    let mut missing_methods = Vec::new();
    let mut matched_methods: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut missing_steps_by_method = Vec::new();
    let mut empty_steps_by_method = Vec::new();

    for scenario in &feature.scenarios {
        let matching: Vec<&String> = all_test_methods
            .iter()
            .filter(|m| method_matches_scenario(m, &scenario.name))
            .collect();

        if matching.is_empty() {
            missing_methods.push(scenario.name.clone());
            continue;
        }

        for method_name in &matching {
            matched_methods.insert((*method_name).clone());

            let (steps, empty) =
                step_finder.find_steps_and_empty_steps_in_method(test_path, method_name)?;

            let scenario_steps: Vec<String> = scenario
                .steps
                .iter()
                .map(|step| format!("{:?} {}", step.step_type, step.text))
                .collect();

            let missing: Vec<String> = scenario_steps
                .iter()
                .filter(|ss| !steps.iter().any(|is| steps_match(is, ss)))
                .cloned()
                .collect();

            if !missing.is_empty() {
                missing_steps_by_method.push(MethodStepResult {
                    method_name: (*method_name).clone(),
                    scenario_name: scenario.name.clone(),
                    issues: missing,
                });
            }
            if !empty.is_empty() {
                empty_steps_by_method.push(MethodStepResult {
                    method_name: (*method_name).clone(),
                    scenario_name: scenario.name.clone(),
                    issues: empty,
                });
            }
        }
    }

    let orphan_methods: Vec<String> = all_test_methods
        .into_iter()
        .filter(|m| !matched_methods.contains(m))
        .collect();

    Ok(PairResult {
        feature_file: feature_path.to_path_buf(),
        test_file: test_path.to_path_buf(),
        missing_methods,
        orphan_methods,
        missing_steps_by_method,
        empty_steps_by_method,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn collect_test_files(dir: &Path, language: &Language) -> Result<Vec<PathBuf>> {
    let ext = match language {
        Language::Python => "py",
        Language::Rust => "rs",
        Language::Jdbc => "java",
        Language::Odbc => "cpp",
        _ => return Ok(vec![]),
    };
    let prefix = match language {
        Language::Python => "test_",
        _ => "",
    };

    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some(ext)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| prefix.is_empty() || n.starts_with(prefix))
        })
        .collect();

    files.sort();
    Ok(files)
}

fn collect_feature_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("feature"))
        .collect();
    files.sort();
    Ok(files)
}

fn test_file_stem(path: &Path, language: &Language) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    match language {
        Language::Python => stem.strip_prefix("test_").unwrap_or(stem).to_string(),
        _ => stem.to_string(),
    }
}

fn collect_test_methods(content: &str, language: &Language) -> Result<Vec<String>> {
    use regex::Regex;
    let re = match language {
        Language::Python => Regex::new(r"def\s+(test_\w+)\s*\(")?,
        Language::Rust => Regex::new(
            r"#\[\s*(?:[a-zA-Z0-9_]+::)?test(?:\([^)]*\))?\s*\]\s*(?:\n\s*)*(?:async\s+)?fn\s+(\w+)\s*\(",
        )?,
        _ => return Ok(vec![]),
    };
    let mut methods: Vec<String> = re
        .captures_iter(content)
        .map(|c| c[1].to_string())
        .collect();
    methods.sort();
    methods.dedup();
    Ok(methods)
}

fn method_matches_scenario(method_name: &str, scenario_name: &str) -> bool {
    let clean = clean_method_name(method_name);
    let snake = to_snake_case(scenario_name);
    strings_match_normalized(clean, scenario_name)
        || strings_match_normalized(clean, &snake)
}

fn steps_match(implemented: &str, expected: &str) -> bool {
    let normalize = |s: &str| {
        s.to_lowercase()
            .replace('"', "")
            .replace('\'', "")
            .replace(',', "")
            .replace('.', "")
            .replace(':', "")
            .replace(';', "")
            .replace('!', "")
            .replace('?', "")
            .replace('(', "")
            .replace(')', "")
            .trim()
            .to_string()
    };
    normalize(implemented) == normalize(expected)
}
