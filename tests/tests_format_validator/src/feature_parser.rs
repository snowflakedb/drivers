use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// All recognized Gherkin scenario keywords (longest prefixes first so that
/// `Scenario Outline:` and `Scenario Template:` are matched before `Scenario:`).
const SCENARIO_PREFIXES: &[&str] = &["Scenario Template:", "Scenario Outline:", "Scenario:"];

static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"@(\w+)").unwrap());
static STEP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(Given|When|Then|And|But)\s+(.+)$").unwrap());

/// Check if a trimmed line starts a scenario (any keyword).
fn is_scenario_start(line: &str) -> bool {
    SCENARIO_PREFIXES.iter().any(|p| line.starts_with(p))
}

/// Strip the scenario keyword prefix from a line, returning the scenario name.
fn strip_scenario_prefix(line: &str) -> &str {
    for prefix in SCENARIO_PREFIXES {
        if let Some(rest) = line.strip_prefix(prefix) {
            return rest.trim();
        }
    }
    line
}

#[derive(Debug, Clone)]
pub struct Feature {
    pub name: String,
    pub file_path: PathBuf,
    pub tags: Vec<String>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: String,
    pub tags: Vec<String>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub step_type: StepType,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum StepType {
    Given,
    When,
    Then,
    And,
    But,
}

impl Scenario {
    /// Checks that the scenario has at least one When and one Then step.
    /// Returns a list of error messages for any missing mandatory step types.
    pub fn validate_mandatory_steps(&self) -> Vec<String> {
        let mut errors = Vec::new();

        let has_when = self
            .steps
            .iter()
            .any(|s| matches!(s.step_type, StepType::When));
        let has_then = self
            .steps
            .iter()
            .any(|s| matches!(s.step_type, StepType::Then));

        if !has_when {
            errors.push(format!(
                "Scenario '{}' is missing a mandatory 'When' step",
                self.name
            ));
        }
        if !has_then {
            errors.push(format!(
                "Scenario '{}' is missing a mandatory 'Then' step",
                self.name
            ));
        }

        errors
    }
}

impl Feature {
    pub fn parse_from_file(path: &Path) -> Result<Feature> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read feature file: {}", path.display()))?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut feature = Feature {
            name,
            file_path: path.to_path_buf(),
            tags: Vec::new(),
            scenarios: Vec::new(),
        };

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        // Parse feature-level tags and feature line
        while i < lines.len() {
            let line = lines[i].trim();

            if line.starts_with('@') {
                // Parse tags
                let tags = Self::parse_tags(line);
                if feature.scenarios.is_empty() {
                    // These are feature-level tags
                    feature.tags.extend(tags);
                }
            } else if line.starts_with("Feature:") {
                // Feature declaration found
                break;
            }
            i += 1;
        }

        // Parse scenarios
        while i < lines.len() {
            let line = lines[i].trim();

            if is_scenario_start(line) {
                let scenario = Self::parse_scenario(&lines, &mut i)?;
                feature.scenarios.push(scenario);
            } else {
                i += 1;
            }
        }

        Ok(feature)
    }

    fn parse_tags(line: &str) -> Vec<String> {
        TAG_REGEX
            .captures_iter(line)
            .map(|cap| cap[1].to_string())
            .collect()
    }

    fn parse_scenario(lines: &[&str], i: &mut usize) -> Result<Scenario> {
        let mut scenario_tags = Vec::new();

        // Look for tags before the scenario
        let mut tag_start = *i;
        while tag_start > 0 && lines[tag_start - 1].trim().starts_with('@') {
            tag_start -= 1;
        }

        // Collect tags
        for line in lines.iter().take(*i).skip(tag_start).map(|s| s.trim()) {
            if line.starts_with('@') {
                scenario_tags.extend(Self::parse_tags(line));
            }
        }

        // Parse scenario name
        let scenario_line = lines[*i].trim();
        let scenario_name = strip_scenario_prefix(scenario_line).to_string();

        *i += 1;

        // Parse steps
        let mut steps = Vec::new();
        while *i < lines.len() {
            let line = lines[*i].trim();

            if line.is_empty() {
                *i += 1;
                continue;
            }

            if is_scenario_start(line) || line.starts_with("Feature:") {
                // Next scenario or feature, stop parsing this scenario
                break;
            }

            // Skip table rows (Examples tables and step data tables)
            if line.starts_with("Examples:") || line.starts_with("|") {
                *i += 1;
                continue;
            }

            if let Some(step) = Self::parse_step(line) {
                steps.push(step);
            }

            *i += 1;
        }

        Ok(Scenario {
            name: scenario_name,
            tags: scenario_tags,
            steps,
        })
    }

    fn parse_step(line: &str) -> Option<Step> {
        if let Some(captures) = STEP_REGEX.captures(line) {
            let step_type = match &captures[1] {
                "Given" => StepType::Given,
                "When" => StepType::When,
                "Then" => StepType::Then,
                "And" => StepType::And,
                "But" => StepType::But,
                _ => return None,
            };

            Some(Step {
                step_type,
                text: captures[2].trim().to_string(),
            })
        } else {
            None
        }
    }

    pub fn get_all_step_texts(&self) -> Vec<String> {
        self.scenarios
            .iter()
            .flat_map(|scenario| &scenario.steps)
            .map(|step| format!("{:?} {}", step.step_type, step.text))
            .collect()
    }
}
