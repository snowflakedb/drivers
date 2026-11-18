use super::base_handler::{BaseDriverHandler, BehaviorDifferenceLocation, TestMethod};
use crate::behavior_differences_utils::compile_bd_regex;
use crate::behavior_differences_utils::load_behavior_difference_files;
use crate::behavior_differences_utils::parse_behavior_differences_descriptions as parse_behavior_differences_descriptions_util;
use crate::step_finder::{
    is_dotnet_test_attribute, DOTNET_ATTRIBUTE_LOOKAHEAD_LINES, DOTNET_METHOD_DECL_REGEX,
};
use crate::utils::{
    clean_method_name, line_index_at_offset, read_source_files, strings_match_normalized,
    to_pascal_case,
};
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, OnceLock};

const STATEMENT_KEYWORDS: &[&str] = &[
    "return", "await", "var", "new", "if", "else", "while", "for", "foreach", "using", "throw",
    "switch", "case", "lock", "yield", "catch",
];
const IGNORED_CALLS: &[&str] = &[
    "nameof",
    "typeof",
    "base",
    "this",
    "ToString",
    "Equals",
    "GetHashCode",
];
const IGNORED_QUALIFIERS: &[&str] = &["Assert"];
const TYPE_KEYWORDS: &[&str] = &["class", "struct", "record", "interface"];

static METHOD_DECL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<modifiers_and_type>[A-Za-z_][\w<>,\[\]?.\s]*?)\s+(?P<name>[A-Za-z_]\w*)\s*(?:<[^>]*>)?\s*\(",
    )
    .unwrap()
});
static METHOD_CALL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?P<constructor>\bnew\s+)?(?:\b(?P<qualifier>[A-Za-z_][\w.]*)\.)?\b(?P<name>[A-Za-z_]\w*)\s*\(",
    )
    .unwrap()
});
static SKIP_ON_OLD_DRIVER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| compile_bd_regex(r#"Skip\.OnOldDriver\s*\(\s*"(?P<id>{bd})""#));
static SKIP_ON_NEW_DRIVER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| compile_bd_regex(r#"Skip\.OnNewDriver\s*\(\s*"(?P<id>{bd})""#));
static NEW_DRIVER_ONLY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    compile_bd_regex(r#"BehaviorDifference\.NewDriverOnly\s*\(\s*"(?P<id>{bd})""#)
});
static OLD_DRIVER_ONLY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    compile_bd_regex(r#"BehaviorDifference\.OldDriverOnly\s*\(\s*"(?P<id>{bd})""#)
});

enum AssertedDriver {
    New,
    Old,
}

struct Marker {
    regex: &'static LazyLock<Regex>,
    asserts: AssertedDriver,
    skips_other_driver: bool,
}

static MARKERS: [Marker; 4] = [
    Marker {
        regex: &SKIP_ON_OLD_DRIVER_REGEX,
        asserts: AssertedDriver::New,
        skips_other_driver: true,
    },
    Marker {
        regex: &SKIP_ON_NEW_DRIVER_REGEX,
        asserts: AssertedDriver::Old,
        skips_other_driver: true,
    },
    Marker {
        regex: &NEW_DRIVER_ONLY_REGEX,
        asserts: AssertedDriver::New,
        skips_other_driver: false,
    },
    Marker {
        regex: &OLD_DRIVER_ONLY_REGEX,
        asserts: AssertedDriver::Old,
        skips_other_driver: false,
    },
];

pub struct DotnetHandler {
    workspace_root: PathBuf,
    common_files: OnceLock<Vec<(PathBuf, String)>>,
}

impl DotnetHandler {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            common_files: OnceLock::new(),
        }
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

    fn extract_helper_method_calls(&self, content: &str, test_method: &str) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for call in helper_calls(content, test_method) {
            if !names.contains(&call.name) {
                names.push(call.name);
            }
        }
        names
    }

    fn find_behavior_differences_in_method(
        &self,
        content: &str,
        method_name: &str,
        file_path: &Path,
    ) -> Result<HashMap<String, BehaviorDifferenceLocation>> {
        let mut behavior_differences = HashMap::new();

        self.find_direct_behavior_differences_in_method(
            content,
            method_name,
            file_path,
            &mut behavior_differences,
        );

        for call in helper_calls(content, method_name) {
            let common_type = call
                .qualifier
                .as_deref()
                .and_then(|qualifier| self.common_files_declaring_type(qualifier));
            if let Some(type_files) = common_type {
                for (path, helper_content) in type_files {
                    self.find_direct_behavior_differences_in_method(
                        helper_content,
                        &call.name,
                        path,
                        &mut behavior_differences,
                    );
                }
            } else if method_body(content, &call.name).is_some() {
                self.find_direct_behavior_differences_in_method(
                    content,
                    &call.name,
                    file_path,
                    &mut behavior_differences,
                );
            } else if let Some((path, helper_content)) = self.only_common_file_defining(&call.name)
            {
                self.find_direct_behavior_differences_in_method(
                    helper_content,
                    &call.name,
                    path,
                    &mut behavior_differences,
                );
            }
        }

        Ok(behavior_differences)
    }

    fn method_matches_scenario(&self, method_name: &str, scenario_name: &str) -> bool {
        strings_match_normalized(
            clean_method_name(method_name),
            &to_pascal_case(scenario_name),
        )
    }
}

impl DotnetHandler {
    fn find_direct_behavior_differences_in_method(
        &self,
        content: &str,
        method_name: &str,
        file_path: &Path,
        behavior_differences: &mut HashMap<String, BehaviorDifferenceLocation>,
    ) {
        let Some((method_start_line, body)) = method_body(content, method_name) else {
            return;
        };

        let rel_path = file_path
            .strip_prefix(&self.workspace_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        let code = body
            .iter()
            .map(|(_, line)| strip_line_comment(line))
            .collect::<Vec<_>>()
            .join("\n");
        let mut matches: Vec<(usize, &Marker, &str)> = MARKERS
            .iter()
            .flat_map(|marker| {
                marker.regex.captures_iter(&code).filter_map(move |c| {
                    let call = c.get(0)?;
                    let id = c.name("id")?;
                    Some((call.start(), marker, id.as_str()))
                })
            })
            .collect();
        matches.sort_by_key(|(offset, _, _)| *offset);

        for (offset, marker, id) in matches {
            let loc = behavior_differences.entry(id.to_string()).or_default();
            let line_no = if marker.skips_other_driver {
                method_start_line
            } else {
                body[line_index_at_offset(&code, offset)].0
            };
            match marker.asserts {
                AssertedDriver::New => {
                    loc.new_behaviour_file = Some(rel_path.clone());
                    loc.new_behaviour_line = Some(line_no);
                    loc.old_driver_skipped |= marker.skips_other_driver;
                }
                AssertedDriver::Old => {
                    loc.old_behaviour_file = Some(rel_path.clone());
                    loc.old_behaviour_line = Some(line_no);
                    loc.new_driver_skipped |= marker.skips_other_driver;
                }
            }
        }
    }

    fn common_files(&self) -> &[(PathBuf, String)] {
        self.common_files.get_or_init(|| {
            read_source_files(
                &self
                    .get_test_directory()
                    .join("Snowflake.Data.Tests.Common"),
                "cs",
            )
        })
    }

    fn common_files_declaring_type(&self, qualifier: &str) -> Option<Vec<&(PathBuf, String)>> {
        let type_name = qualifier.rsplit('.').next().unwrap_or(qualifier);
        let files: Vec<_> = self
            .common_files()
            .iter()
            .filter(|(_, content)| declares_type(content, type_name))
            .collect();
        (!files.is_empty()).then_some(files)
    }

    /// Unqualified and extension-method calls carry no type name, so a Common helper is
    /// attributed only when exactly one Common file defines a method with that name.
    fn only_common_file_defining(&self, method_name: &str) -> Option<&(PathBuf, String)> {
        let mut defining = self
            .common_files()
            .iter()
            .filter(|(_, content)| method_body(content, method_name).is_some());
        let first = defining.next()?;
        defining.next().is_none().then_some(first)
    }
}

struct HelperCall {
    qualifier: Option<String>,
    name: String,
}

fn helper_calls(content: &str, method_name: &str) -> Vec<HelperCall> {
    let mut calls: Vec<HelperCall> = Vec::new();
    let Some((_, body)) = method_body(content, method_name) else {
        return calls;
    };

    for (_, line) in body {
        let code = blank_string_literals(strip_line_comment(line));
        for captures in METHOD_CALL_REGEX.captures_iter(&code) {
            let Some(name) = captures.name("name").map(|m| m.as_str()) else {
                continue;
            };
            let qualifier = captures.name("qualifier").map(|m| m.as_str());
            let is_constructor = captures.name("constructor").is_some();
            let ignored_qualifier = qualifier.is_some_and(|q| IGNORED_QUALIFIERS.contains(&q));
            let already_seen = calls
                .iter()
                .any(|call| call.name == name && call.qualifier.as_deref() == qualifier);
            if is_constructor
                || ignored_qualifier
                || already_seen
                || name == method_name
                || STATEMENT_KEYWORDS.contains(&name)
                || IGNORED_CALLS.contains(&name)
            {
                continue;
            }
            calls.push(HelperCall {
                qualifier: qualifier.map(str::to_string),
                name: name.to_string(),
            });
        }
    }

    calls
}

fn declares_type(content: &str, type_name: &str) -> bool {
    content.lines().any(|line| {
        let words: Vec<&str> = strip_line_comment(line).split_whitespace().collect();
        words.windows(2).any(|pair| {
            let declared = pair[1]
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .unwrap_or("");
            TYPE_KEYWORDS.contains(&pair[0]) && declared == type_name
        })
    })
}

fn is_method_declaration(line: &str, method_name: &str) -> bool {
    let code = strip_line_comment(line).trim();
    if code.ends_with(';') {
        return false;
    }
    let Some(captures) = METHOD_DECL_REGEX.captures(code) else {
        return false;
    };
    if captures.name("name").map(|m| m.as_str()) != Some(method_name) {
        return false;
    }
    let first_word = captures
        .name("modifiers_and_type")
        .and_then(|m| m.as_str().split_whitespace().next())
        .unwrap_or("");
    !STATEMENT_KEYWORDS.contains(&first_word)
}

/// Returns the body of the first method declared with `method_name` in `content`.
///
/// Overloads and methods with the same name in different classes of one file are not told
/// apart: only the first declaration is read, so markers in the others are missed without an
/// error. The .NET test files do not declare such methods today.
fn method_body<'a>(content: &'a str, method_name: &str) -> Option<(usize, Vec<(usize, &'a str)>)> {
    let lines: Vec<&str> = content.lines().collect();
    let start = lines
        .iter()
        .position(|line| is_method_declaration(line, method_name))?;

    let mut body = Vec::new();
    let mut depth: i32 = 0;
    let mut entered_block = false;
    for (offset, line) in lines[start..].iter().enumerate() {
        let code = blank_string_literals(strip_line_comment(line));
        body.push((start + offset + 1, *line));
        depth += code.matches('{').count() as i32 - code.matches('}').count() as i32;
        if code.contains('{') {
            entered_block = true;
        }
        if entered_block && depth <= 0 {
            break;
        }
        if !entered_block && code.trim_end().ends_with(';') {
            break;
        }
    }
    Some((start + 1, body))
}

fn literal_mask(line: &str) -> Vec<bool> {
    let bytes = line.as_bytes();
    let mut mask = vec![false; bytes.len()];
    let mut i = 0;
    while i < bytes.len() {
        let quote = bytes[i];
        if quote != b'"' && quote != b'\'' {
            i += 1;
            continue;
        }
        let verbatim = quote == b'"' && (line[..i].ends_with('@') || line[..i].ends_with("@$"));
        let start = i;
        i += 1;
        while i < bytes.len() {
            match bytes[i] {
                b'\\' if !verbatim => i += 2,
                b'"' if verbatim && bytes.get(i + 1) == Some(&b'"') => i += 2,
                b if b == quote => {
                    i += 1;
                    break;
                }
                _ => i += 1,
            }
        }
        let end = i.min(bytes.len());
        mask[start..end].fill(true);
        i = end;
    }
    mask
}

fn strip_line_comment(line: &str) -> &str {
    let mask = literal_mask(line);
    let end = line
        .match_indices("//")
        .map(|(i, _)| i)
        .find(|&i| !mask[i])
        .unwrap_or(line.len());
    &line[..end]
}

fn blank_string_literals(code: &str) -> String {
    let blanked: Vec<u8> = code
        .bytes()
        .zip(literal_mask(code))
        .map(|(byte, in_literal)| if in_literal { b' ' } else { byte })
        .collect();
    String::from_utf8_lossy(&blanked).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handler() -> DotnetHandler {
        DotnetHandler::new(PathBuf::from("/workspace"))
    }

    #[test]
    fn bd_markers_accept_number_and_slug() {
        assert!(SKIP_ON_OLD_DRIVER_REGEX.is_match(r#"Skip.OnOldDriver("BD#12", "reason");"#));
        assert!(SKIP_ON_OLD_DRIVER_REGEX
            .is_match(r#"Skip.OnOldDriver("BD#untrusted-root-rejected", "reason");"#));
        assert!(SKIP_ON_NEW_DRIVER_REGEX
            .is_match(r#"Skip.OnNewDriver( "BD#untrusted-root-rejected", "reason");"#));
        assert!(NEW_DRIVER_ONLY_REGEX.is_match(r#"if (BehaviorDifference.NewDriverOnly("BD#3"))"#));
        assert!(OLD_DRIVER_ONLY_REGEX
            .is_match(r#"if (BehaviorDifference.OldDriverOnly("BD#server-version-patch"))"#));
        assert!(!SKIP_ON_OLD_DRIVER_REGEX.is_match(r#"Skip.OnOldDriver("BD#Bad_Slug", "x");"#));
        assert!(!SKIP_ON_NEW_DRIVER_REGEX.is_match(r#"Skip.OnNewDriver("BD#Bad_Slug", "x");"#));
        assert!(
            !NEW_DRIVER_ONLY_REGEX.is_match(r#"BehaviorDifference.NewDriverOnly("BD#Bad_Slug")"#)
        );
        assert!(
            !OLD_DRIVER_ONLY_REGEX.is_match(r#"BehaviorDifference.OldDriverOnly("BD#Bad_Slug")"#)
        );
        assert!(!NEW_DRIVER_ONLY_REGEX.is_match(r#"BehaviorDifference.NewDriverOnly("BD#")"#));
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
    fn records_skip_and_only_markers_with_expected_lines() {
        let content = r#"
public class ConnectionTest
{
    public override Task ShouldRejectUntrustedRootAsync()
    {
        Skip.OnOldDriver("BD#untrusted-root-rejected", "old driver accepts it");
        return base.ShouldRejectUntrustedRootAsync();
    }

    [SnowflakeFact]
    public void ShouldReportServerVersion()
    {
        var version = connection.ServerVersion;
        if (BehaviorDifference.NewDriverOnly("BD#server-version-patch"))
        {
            Assert.Equal("9.1.0", version);
        }
        if (BehaviorDifference.OldDriverOnly("BD#server-version-patch"))
        {
            Assert.Equal("9.1", version);
        }
    }
}
"#;
        let path = PathBuf::from("/workspace/dotnet/tests/X/ConnectionTest.cs");
        let handler = handler();

        let skip = handler
            .find_behavior_differences_in_method(content, "ShouldRejectUntrustedRootAsync", &path)
            .unwrap();
        let loc = &skip["BD#untrusted-root-rejected"];
        assert_eq!(loc.new_behaviour_line, Some(4));
        assert_eq!(
            loc.new_behaviour_file.as_deref(),
            Some("dotnet/tests/X/ConnectionTest.cs")
        );
        assert!(loc.old_driver_skipped);
        assert!(!loc.new_driver_skipped);

        let only = handler
            .find_behavior_differences_in_method(content, "ShouldReportServerVersion", &path)
            .unwrap();
        let loc = &only["BD#server-version-patch"];
        assert_eq!(loc.new_behaviour_line, Some(14));
        assert_eq!(loc.old_behaviour_line, Some(18));
        assert!(!loc.old_driver_skipped && !loc.new_driver_skipped);
    }

    #[test]
    fn follows_same_file_helper() {
        let content = r#"
public class ConnectionTest
{
    [SnowflakeFact]
    public async Task ShouldRejectUntrustedRootAsync()
    {
        await AssertRejectedAsync(connection);
    }

    private static async Task AssertRejectedAsync(SnowflakeDbConnection connection)
    {
        Skip.OnNewDriver("BD#7", "new driver rejects");
    }
}
"#;
        let path = PathBuf::from("/workspace/dotnet/tests/X/ConnectionTest.cs");
        let handler = handler();
        assert_eq!(
            handler.extract_helper_method_calls(content, "ShouldRejectUntrustedRootAsync"),
            vec!["AssertRejectedAsync".to_string()]
        );
        let bds = handler
            .find_behavior_differences_in_method(content, "ShouldRejectUntrustedRootAsync", &path)
            .unwrap();
        let loc = &bds["BD#7"];
        assert_eq!(loc.old_behaviour_line, Some(10));
        assert!(loc.new_driver_skipped);
    }

    #[test]
    fn override_delegating_to_base_has_no_helpers() {
        let content = r#"
public class ConnectionTest : ConnectionTestBase
{
    public override Task ShouldRejectUntrustedRootAsync()
    {
        return base.ShouldRejectUntrustedRootAsync();
    }
}
"#;
        assert!(handler()
            .extract_helper_method_calls(content, "ShouldRejectUntrustedRootAsync")
            .is_empty());
    }

    #[test]
    fn ignores_constructors_assertions_and_calls_inside_strings() {
        let content = r#"
public class ConnectionTest
{
    [SnowflakeFact]
    public void ShouldConnect()
    {
        var builder = new SnowflakeDbConnectionStringBuilder();
        Assert.Equal("Open(", Describe(builder));
        Assert.Throws<SnowflakeDbException>(() => Connect(builder)); // Unused(x)
    }
}
"#;
        assert_eq!(
            handler().extract_helper_method_calls(content, "ShouldConnect"),
            vec!["Describe".to_string(), "Connect".to_string()]
        );
    }

    #[test]
    fn records_marker_split_across_lines() {
        let content = r#"
public class ConnectionTest
{
    [SnowflakeFact]
    public void ShouldReportServerVersion()
    {
        if (BehaviorDifference.NewDriverOnly(
                "BD#server-version-patch"))
        {
        }
    }

    public override Task ShouldRejectUntrustedRootAsync()
    {
        Skip.OnOldDriver(
            "BD#untrusted-root-rejected",
            "old driver accepts it");
        return base.ShouldRejectUntrustedRootAsync();
    }
}
"#;
        let path = PathBuf::from("/workspace/dotnet/tests/X/ConnectionTest.cs");
        let handler = handler();

        let only = handler
            .find_behavior_differences_in_method(content, "ShouldReportServerVersion", &path)
            .unwrap();
        assert_eq!(only["BD#server-version-patch"].new_behaviour_line, Some(7));

        let skip = handler
            .find_behavior_differences_in_method(content, "ShouldRejectUntrustedRootAsync", &path)
            .unwrap();
        let loc = &skip["BD#untrusted-root-rejected"];
        assert_eq!(loc.new_behaviour_line, Some(13));
        assert!(loc.old_driver_skipped);
    }

    #[test]
    fn ignores_commented_out_markers() {
        let content = r#"
public class ConnectionTest
{
    [SnowflakeFact]
    public void ShouldConnect()
    {
        // Skip.OnOldDriver("BD#full-line", "disabled");
        Connect(); // Skip.OnNewDriver("BD#trailing", "disabled");
    }
}
"#;
        let path = PathBuf::from("/workspace/dotnet/tests/X/ConnectionTest.cs");
        let bds = handler()
            .find_behavior_differences_in_method(content, "ShouldConnect", &path)
            .unwrap();
        assert!(bds.is_empty(), "unexpected markers: {:?}", bds.keys());
    }

    #[test]
    fn strip_line_comment_keeps_slashes_inside_literals() {
        assert_eq!(
            strip_line_comment(r#"var url = "https://host"; // note"#),
            r#"var url = "https://host"; "#
        );
        assert_eq!(
            strip_line_comment(r#"var p = @"C:\dir\""//x"; // note"#),
            r#"var p = @"C:\dir\""//x"; "#
        );
        assert_eq!(
            strip_line_comment(r#"var s = "a\"//b"; // note"#),
            r#"var s = "a\"//b"; "#
        );
        assert_eq!(strip_line_comment("var c = '/'; // note"), "var c = '/'; ");
        assert_eq!(strip_line_comment(r"var c = '\''; // x"), r"var c = '\''; ");
        assert_eq!(strip_line_comment("Connect(); // it's fine"), "Connect(); ");
    }

    #[test]
    fn braces_inside_string_literals_do_not_end_the_body() {
        let content = r#"
public class ConnectionTest
{
    [SnowflakeFact]
    public void ShouldReportServerVersion()
    {
        var json = "{\"a\": }}";
        var c = '}';
        if (BehaviorDifference.NewDriverOnly("BD#server-version-patch"))
        {
        }
    }
}
"#;
        let path = PathBuf::from("/workspace/dotnet/tests/X/ConnectionTest.cs");
        let bds = handler()
            .find_behavior_differences_in_method(content, "ShouldReportServerVersion", &path)
            .unwrap();
        assert_eq!(bds["BD#server-version-patch"].new_behaviour_line, Some(9));
    }

    #[test]
    fn attributes_marker_from_common_helper_to_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let common = dir
            .path()
            .join("dotnet")
            .join("tests")
            .join("Snowflake.Data.Tests.Common");
        fs::create_dir_all(&common).unwrap();
        fs::write(
            common.join("TlsAssertions.cs"),
            r#"public static class TlsAssertions
{
    public static void AssertRejected(SnowflakeDbConnection connection)
    {
        Skip.OnNewDriver("BD#untrusted-root-rejected", "new driver rejects");
        if (BehaviorDifference.OldDriverOnly("BD#untrusted-root-rejected"))
        {
        }
    }
}
"#,
        )
        .unwrap();
        let content = r#"
public class ConnectionTest
{
    [SnowflakeFact]
    public void ShouldRejectUntrustedRoot()
    {
        TlsAssertions.AssertRejected(connection);
    }
}
"#;
        let test_path = dir.path().join("dotnet/tests/X/ConnectionTest.cs");
        let bds = DotnetHandler::new(dir.path().to_path_buf())
            .find_behavior_differences_in_method(content, "ShouldRejectUntrustedRoot", &test_path)
            .unwrap();
        let loc = &bds["BD#untrusted-root-rejected"];
        assert_eq!(
            loc.old_behaviour_file.as_deref(),
            Some("dotnet/tests/Snowflake.Data.Tests.Common/TlsAssertions.cs")
        );
        assert_eq!(loc.old_behaviour_line, Some(6));
        assert!(loc.new_driver_skipped);
        assert!(loc.new_behaviour_file.is_none());
    }

    fn handler_with_common_files(files: &[(&str, &str)]) -> (tempfile::TempDir, DotnetHandler) {
        let dir = tempfile::tempdir().unwrap();
        let common = dir
            .path()
            .join("dotnet")
            .join("tests")
            .join("Snowflake.Data.Tests.Common");
        fs::create_dir_all(&common).unwrap();
        for (name, content) in files {
            fs::write(common.join(name), content).unwrap();
        }
        let handler = DotnetHandler::new(dir.path().to_path_buf());
        (dir, handler)
    }

    const TLS_ASSERTIONS: &str = r#"public static class TlsAssertions
{
    public static void Foo()
    {
        Skip.OnNewDriver("BD#tls-foo", "tls");
    }
}
"#;
    const DB_ASSERTIONS: &str = r#"public static class DbAssertions
{
    public static void Foo()
    {
        Skip.OnNewDriver("BD#db-foo", "db");
    }
}
"#;

    #[test]
    fn attributes_qualified_common_helper_only_from_the_called_type() {
        let (dir, handler) = handler_with_common_files(&[
            ("TlsAssertions.cs", TLS_ASSERTIONS),
            ("DbAssertions.cs", DB_ASSERTIONS),
        ]);
        let content = r#"
public class ConnectionTest
{
    [SnowflakeFact]
    public void ShouldConnect()
    {
        TlsAssertions.Foo();
    }
}
"#;
        let test_path = dir.path().join("dotnet/tests/X/ConnectionTest.cs");
        let bds = handler
            .find_behavior_differences_in_method(content, "ShouldConnect", &test_path)
            .unwrap();
        assert_eq!(bds.keys().collect::<Vec<_>>(), vec!["BD#tls-foo"]);
        assert_eq!(
            bds["BD#tls-foo"].old_behaviour_file.as_deref(),
            Some("dotnet/tests/Snowflake.Data.Tests.Common/TlsAssertions.cs")
        );
    }

    #[test]
    fn unqualified_common_helper_defined_in_two_files_is_not_attributed() {
        let (dir, handler) = handler_with_common_files(&[
            ("TlsAssertions.cs", TLS_ASSERTIONS),
            ("DbAssertions.cs", DB_ASSERTIONS),
        ]);
        let content = r#"
public class ConnectionTest
{
    [SnowflakeFact]
    public void ShouldConnect()
    {
        connection.Foo();
    }
}
"#;
        let test_path = dir.path().join("dotnet/tests/X/ConnectionTest.cs");
        let bds = handler
            .find_behavior_differences_in_method(content, "ShouldConnect", &test_path)
            .unwrap();
        assert!(bds.is_empty());
    }

    #[test]
    fn unqualified_common_helper_defined_in_one_file_is_attributed() {
        let (dir, handler) = handler_with_common_files(&[("TlsAssertions.cs", TLS_ASSERTIONS)]);
        let content = r#"
public class ConnectionTest
{
    [SnowflakeFact]
    public void ShouldConnect()
    {
        connection.Foo();
    }
}
"#;
        let test_path = dir.path().join("dotnet/tests/X/ConnectionTest.cs");
        let bds = handler
            .find_behavior_differences_in_method(content, "ShouldConnect", &test_path)
            .unwrap();
        assert_eq!(bds.keys().collect::<Vec<_>>(), vec!["BD#tls-foo"]);
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
