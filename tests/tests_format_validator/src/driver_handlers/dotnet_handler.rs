use super::base_handler::{BaseDriverHandler, BehaviorDifferenceLocation, TestMethod};
use crate::behavior_differences_utils::load_behavior_difference_files;
use crate::behavior_differences_utils::parse_behavior_differences_descriptions as parse_behavior_differences_descriptions_util;
use crate::step_finder::{
    is_dotnet_test_attribute, DOTNET_ATTRIBUTE_LOOKAHEAD_LINES, DOTNET_METHOD_DECL_REGEX,
};
use crate::utils::{clean_method_name, strings_match_normalized, to_pascal_case};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct DotnetHandler {
    workspace_root: PathBuf,
}

impl DotnetHandler {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }
}

impl BaseDriverHandler for DotnetHandler {
    fn supports_behavior_differences(&self) -> bool {
        true
    }

    fn get_behavior_differences_file_path(&self) -> PathBuf {
        self.workspace_root
            .join("dotnet")
            .join("BehaviorDifferences.yaml")
    }

    fn get_test_directory(&self) -> PathBuf {
        self.workspace_root.join("dotnet").join("tests")
    }

    fn get_test_file_extensions(&self) -> Vec<String> {
        vec!["*.cs".to_string()]
    }

    fn parse_behavior_differences_descriptions(&self) -> Result<HashMap<String, String>> {
        let mut descriptions = HashMap::new();
        let monolith_path = self.get_behavior_differences_file_path();
        if monolith_path.exists() {
            let content = fs::read_to_string(&monolith_path).with_context(|| {
                format!(
                    "Failed to read Behavior Difference file: {}",
                    monolith_path.display()
                )
            })?;
            descriptions.extend(parse_behavior_differences_descriptions_util(&content)?);
        }

        let files_dir = self
            .workspace_root
            .join("dotnet")
            .join("behavior_differences");
        for (id, description) in load_behavior_difference_files(&files_dir)? {
            descriptions.entry(id).or_insert(description);
        }
        Ok(descriptions)
    }

    fn extract_test_methods(&self, content: &str) -> Vec<TestMethod> {
        let mut methods = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            if is_dotnet_test_attribute(trimmed) {
                let declaration = lines
                    .iter()
                    .enumerate()
                    .skip(i + 1)
                    .take(DOTNET_ATTRIBUTE_LOOKAHEAD_LINES)
                    .find_map(|(j, candidate)| {
                        DOTNET_METHOD_DECL_REGEX
                            .captures(candidate.trim())
                            .and_then(|c| c.get(1))
                            .map(|name| (j, name.as_str().to_string()))
                    });
                if let Some((j, name)) = declaration {
                    methods.push(TestMethod { name, line: j + 1 });
                    i = j;
                }
            } else if trimmed.starts_with("public override ") {
                if let Some(name) = DOTNET_METHOD_DECL_REGEX
                    .captures(trimmed)
                    .and_then(|c| c.get(1))
                {
                    methods.push(TestMethod {
                        name: name.as_str().to_string(),
                        line: i + 1,
                    });
                }
            }
            i += 1;
        }

        methods
    }

    fn extract_helper_method_calls(&self, _content: &str, _test_method: &str) -> Vec<String> {
        Vec::new()
    }

    fn find_behavior_differences_in_method(
        &self,
        _content: &str,
        _method_name: &str,
        _file_path: &Path,
    ) -> Result<HashMap<String, BehaviorDifferenceLocation>> {
        Ok(HashMap::new())
    }

    fn method_matches_scenario(&self, method_name: &str, scenario_name: &str) -> bool {
        strings_match_normalized(
            clean_method_name(method_name),
            &to_pascal_case(scenario_name),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handler() -> DotnetHandler {
        DotnetHandler::new(PathBuf::from("/workspace"))
    }

    #[test]
    fn extracts_attributed_and_reference_override_methods() {
        let content = r#"
public class TlsVersionTest
{
    [SnowflakeFact]
    public virtual async Task ShouldNegotiateTlsAsync()
    {
    }

    [SnowflakeTheory]
    [InlineData(1)]
    [InlineData(2)]
    public void ShouldParseVersion(int value)
    {
    }

    public override Task ShouldFailTheHandshakeAsync()
    {
        return base.ShouldFailTheHandshakeAsync();
    }

    protected override string ExpectedMessage => "x";

    private static void Helper()
    {
    }
}
"#;
        let methods = handler().extract_test_methods(content);
        let names: Vec<(&str, usize)> = methods.iter().map(|m| (m.name.as_str(), m.line)).collect();
        assert_eq!(
            names,
            vec![
                ("ShouldNegotiateTlsAsync", 5),
                ("ShouldParseVersion", 12),
                ("ShouldFailTheHandshakeAsync", 16),
            ]
        );
    }

    #[test]
    fn extracts_methods_after_multi_line_and_many_attributes() {
        let inline_data: String = (0..12)
            .map(|n| format!("    [InlineData({n})]\n"))
            .collect();
        let content = format!(
            r#"
public class TlsVersionTest
{{
    [SnowflakeFact(
        SkipCondition.SkipOnMacOS,
        "TLS 1.3 is not supported on macOS")]
    public override Task ShouldNegotiateTlsAsync()
    {{
    }}

    [SnowflakeTheory]
{inline_data}    public void ShouldParseVersion(int value)
    {{
    }}
}}
"#
        );
        let methods = handler().extract_test_methods(&content);
        let names: Vec<(&str, usize)> = methods.iter().map(|m| (m.name.as_str(), m.line)).collect();
        assert_eq!(
            names,
            vec![("ShouldNegotiateTlsAsync", 7), ("ShouldParseVersion", 24)]
        );
    }

    #[test]
    fn matches_scenario_ignoring_async_suffix() {
        let handler = handler();
        assert!(handler.method_matches_scenario(
            "ShouldRejectUntrustedRootAsync",
            "should reject untrusted root"
        ));
        assert!(handler.method_matches_scenario("ShouldParseVersion", "should parse version"));
        assert!(!handler.method_matches_scenario("ShouldParseVersionAsync", "should parse"));
    }

    #[test]
    fn loads_monolith_key_and_per_file_entries() {
        let dir = tempfile::tempdir().unwrap();
        let dotnet = dir.path().join("dotnet");
        let area = dotnet.join("behavior_differences").join("connection");
        fs::create_dir_all(&area).unwrap();
        fs::write(
            dotnet.join("BehaviorDifferences.yaml"),
            "behavior_differences:\n  untrusted-root-rejected:\n    name: \"Still in the monolith\"\n",
        )
        .unwrap();
        fs::write(
            area.join("server-version-patch.yaml"),
            "name: \"ServerVersion drops the patch digit\"\ntype: bugfix\nrationale: \"sf_core reports it\"\n",
        )
        .unwrap();

        let descriptions = DotnetHandler::new(dir.path().to_path_buf())
            .parse_behavior_differences_descriptions()
            .unwrap();
        assert!(descriptions
            .get("BD#server-version-patch")
            .is_some_and(|text| text.contains("ServerVersion drops the patch digit")));
        assert!(descriptions.contains_key("BD#untrusted-root-rejected"));
    }
}
