use crate::test_discovery::Language;
use crate::utils::{clean_method_name, strings_match_normalized, to_pascal_case, to_snake_case};
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;

/// Configuration for language-specific step finding
#[derive(Debug)]
struct LanguageConfig {
    /// Test annotation/decorator (e.g., "@Test", "#[test]")
    test_annotation: &'static str,
    /// Regex pattern for method/function declaration
    method_pattern: fn(&str) -> String,
    /// Patterns that indicate end of method (next test, top-level constructs)
    method_end_patterns: &'static [&'static str],
    /// Whether the language uses braces to delimit method bodies (false for Python)
    uses_braces: bool,
}

impl LanguageConfig {
    fn jdbc() -> Self {
        Self {
            test_annotation: "@Test",
            method_pattern: |method_name| format!("{}(", method_name),
            method_end_patterns: &[
                // Empty - rely purely on brace counting for Java
            ],
            uses_braces: true,
        }
    }

    fn rust() -> Self {
        Self {
            test_annotation: "#[test]",
            // Allow optional async before fn
            method_pattern: |method_name| {
                format!(r"(?:async\s+)?fn\s+{}\s*\(", regex::escape(method_name))
            },
            method_end_patterns: &[
                // Empty - rely purely on brace counting for Rust
            ],
            uses_braces: true,
        }
    }

    fn python() -> Self {
        Self {
            test_annotation: "def test_", // Python test methods start with "def test_"
            method_pattern: |method_name| format!(r"def\s+{}\s*\(", regex::escape(method_name)),
            method_end_patterns: &[
                "def ",        // Function definition
                "async def ",  // Async function definition
                "class ",      // Class definition
                "@",           // Decorators (@pytest.fixture, @property, etc.)
                "if __name__", // Main guard
                "import ",     // Import statement
                "from ",       // From import statement
                "global ",     // Global variable declaration
                "nonlocal ",   // Nonlocal variable declaration
            ],
            uses_braces: false,
        }
    }

    fn csharp() -> Self {
        Self {
            test_annotation: "[Test]",
            method_pattern: |method_name| format!("{}(", method_name),
            method_end_patterns: &[
                // Empty - rely purely on brace counting for C#
            ],
            uses_braces: true,
        }
    }

    fn odbc() -> Self {
        Self {
            test_annotation: "TEST_CASE(", // Catch2 style
            method_pattern: |method_name| {
                format!(r#"TEST_CASE\s*\(\s*"{}""#, regex::escape(method_name))
            },
            method_end_patterns: &[
                // Empty - rely purely on brace counting for C++
            ],
            uses_braces: true,
        }
    }

    fn javascript() -> Self {
        Self {
            test_annotation: "test(", // Jest/Mocha style
            method_pattern: |method_name| {
                format!(r"test\s*\(\s*['\x22]{}['\x22]", regex::escape(method_name))
            },
            method_end_patterns: &[
                // Empty - rely purely on brace counting for JavaScript
            ],
            uses_braces: true,
        }
    }
}

/// Parsed step position within method lines, tracking where the comment starts
/// and where the code region after it begins.
struct StepPosition {
    text: String,
    comment_start: usize,
    code_search_start: usize,
}

/// Generic method boundary finder that works across languages
struct MethodBoundaryFinder {
    config: LanguageConfig,
}

impl MethodBoundaryFinder {
    fn new(config: LanguageConfig) -> Self {
        Self { config }
    }

    /// Find all test methods in the content with their line numbers
    fn find_all_test_methods_with_lines(&self, content: &str) -> Result<Vec<(String, usize)>> {
        let lines: Vec<&str> = content.lines().collect();
        let mut methods = Vec::new();

        // Pre-compile regexes outside the loop
        let test_method_regex = Regex::new(r"def\s+(test_\w+)\s*\(")?;
        let catch2_regex = Regex::new(r#"TEST_CASE\s*\(\s*"([^"]+)""#)?;
        // Rust: support optional async before fn
        let fn_regex = Regex::new(r"(?:async\s+)?fn\s+(\w+)")?;
        // Match Java method declarations (not annotation lines like @MethodSource(...))
        let method_regex = Regex::new(
            r"^(?:public|protected|private)?\s*(?:static\s+)?(?:async\s+)?(?:final\s+)?(?:void|Task(?:<[^>]+>)?)\s+(\w+)\s*\(",
        )?;
        let rust_test_attr_regex = Regex::new(r"^#\[\s*(?:[a-zA-Z0-9_]+::)?test(?:\(.*\))?\s*\]$")?;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Handle different test annotation styles
            match self.config.test_annotation {
                "def test_" => {
                    // Python: method declaration is the test itself
                    if let Some(captures) = test_method_regex.captures(trimmed) {
                        let method_name = captures[1].to_string();
                        methods.push((method_name, i + 1)); // +1 for 1-indexed line numbers
                    }
                }
                "TEST_CASE(" => {
                    // C++ Catch2: TEST_CASE("method_name")
                    if trimmed.starts_with("TEST_CASE(") {
                        if let Some(captures) = catch2_regex.captures(trimmed) {
                            let method_name = captures[1].to_string();
                            methods.push((method_name, i + 1)); // +1 for 1-indexed line numbers
                        }
                    }
                }
                "#[test]" => {
                    // Rust: any test attribute (#[test], #[tokio::test], #[tokio::test(...)], #[rstest], #[test_case]) followed by (async)? fn
                    let is_rust_attr = rust_test_attr_regex.is_match(trimmed)
                        || trimmed.starts_with("#[rstest")
                        || trimmed.starts_with("#[test_case");
                    if is_rust_attr && i + 1 < lines.len() {
                        // Allow more lookahead for rstest/test_case due to case lines
                        let lookahead = if trimmed.starts_with("#[rstest")
                            || trimmed.starts_with("#[test_case")
                        {
                            20
                        } else {
                            8
                        };
                        let mut search_idx = i + 1;
                        let end_idx = (i + 1 + lookahead).min(lines.len());
                        while search_idx < end_idx {
                            let search_line = lines[search_idx].trim();
                            // Skip case annotations and blanks/comments
                            if search_line.starts_with("#[case")
                                || search_line.starts_with("#[test_case")
                                || search_line.is_empty()
                                || search_line.starts_with("//")
                            {
                                search_idx += 1;
                                continue;
                            }
                            if let Some(captures) = fn_regex.captures(search_line) {
                                let method_name = captures[1].to_string();
                                methods.push((method_name, search_idx + 1)); // +1 for 1-indexed
                                break;
                            }
                            search_idx += 1;
                        }
                    }
                }
                "@Test" => {
                    // Java: @Test/@ParameterizedTest followed by method declaration
                    if trimmed == "@Test" || trimmed == "@ParameterizedTest" {
                        // Look ahead for the method declaration
                        for (j, method_line) in lines.iter().enumerate().skip(i + 1).take(4) {
                            if let Some(captures) = method_regex.captures(method_line.trim()) {
                                let method_name = captures[1].to_string();
                                methods.push((method_name, j + 1)); // +1 for 1-indexed line numbers
                                break;
                            }
                        }
                    }
                }
                "[Test]" => {
                    // C#: [Test] followed by method declaration
                    if trimmed == "[Test]" {
                        // Look ahead for the method declaration
                        for (j, method_line) in lines.iter().enumerate().skip(i + 1).take(4) {
                            if let Some(captures) = method_regex.captures(method_line.trim()) {
                                let method_name = captures[1].to_string();
                                methods.push((method_name, j + 1)); // +1 for 1-indexed line numbers
                                break;
                            }
                        }
                    }
                }
                _ => {} // Unknown annotation pattern
            }
        }

        Ok(methods)
    }

    /// Find the boundaries of a specific method in the content
    fn find_method_boundaries(
        &self,
        content: &str,
        method_name: &str,
    ) -> Result<Option<(usize, usize)>> {
        let lines: Vec<&str> = content.lines().collect();
        let mut method_start_line: Option<usize> = None;
        let mut method_end_line: Option<usize> = None;

        // Regex to detect Rust test attributes like #[test], #[tokio::test], #[tokio::test(...)]
        let rust_test_attr_regex = Regex::new(r"^#\[\s*(?:[a-zA-Z0-9_]+::)?test(?:\(.*\))?\s*\]$")?;

        // Find the method start
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Special handling for Python - method declaration is the annotation
            if self.config.test_annotation == "def test_" {
                let pattern = (self.config.method_pattern)(method_name);
                let method_regex = Regex::new(&pattern)?;
                if method_regex.is_match(trimmed) {
                    method_start_line = Some(i);
                    break;
                }
            }
            // Look for test annotation
            else if trimmed == self.config.test_annotation
                || (self.config.test_annotation == "@Test" && trimmed == "@ParameterizedTest")
                || (self.config.test_annotation == "#[test]"
                    && (trimmed.starts_with("#[rstest") || trimmed.starts_with("#[test_case")))
                || (self.config.test_annotation.contains("pytest")
                    && trimmed.starts_with("@pytest"))
                || (self.config.test_annotation == "TEST_CASE("
                    && trimmed.starts_with("TEST_CASE("))
                || (self.config.test_annotation == "#[test]"
                    && rust_test_attr_regex.is_match(trimmed))
            {
                // Rust special-case: generic test attribute matched above
                // For C++, the TEST_CASE line itself contains the method name
                if self.config.test_annotation == "TEST_CASE(" {
                    let pattern = (self.config.method_pattern)(method_name);
                    let method_regex = Regex::new(&pattern)?;
                    if method_regex.is_match(trimmed) {
                        method_start_line = Some(i);
                        break;
                    }
                } else {
                    // Look ahead for the method declaration
                    let search_limit = if self.config.test_annotation == "#[test]"
                        && (trimmed.starts_with("#[rstest") || trimmed.starts_with("#[test_case"))
                    {
                        // For #[rstest] or #[test_case], we might need to look further ahead due to #[case] or #[test_case] lines
                        20
                    } else {
                        4
                    };

                    for (j, method_line) in lines.iter().enumerate().skip(i + 1).take(search_limit)
                    {
                        let method_line_trimmed = method_line.trim();

                        // Skip #[case] lines for rstest and #[test_case] lines for test_case
                        if self.config.test_annotation == "#[test]"
                            && (method_line_trimmed.starts_with("#[case")
                                || method_line_trimmed.starts_with("#[test_case"))
                        {
                            continue;
                        }

                        // Skip empty lines and comments
                        if method_line_trimmed.is_empty() || method_line_trimmed.starts_with("//") {
                            continue;
                        }

                        let pattern = (self.config.method_pattern)(method_name);
                        if self.config.test_annotation == "#[test]" {
                            // Rust uses regex pattern
                            let method_regex = Regex::new(&pattern)?;
                            if method_regex.is_match(method_line) {
                                method_start_line = Some(j);
                                break;
                            }
                        } else {
                            // Java use simple contains
                            if method_line.contains(&pattern) {
                                method_start_line = Some(j);
                                break;
                            }
                        }
                    }
                }
                if method_start_line.is_some() {
                    break;
                }
            }
        }

        if method_start_line.is_none() {
            return Ok(None);
        }

        let start_idx = method_start_line.unwrap();

        // Find the method end - handle nested blocks properly
        let mut brace_depth = 0;
        let mut found_opening_brace = false;
        let search_limit = start_idx + 500; // Allow larger bodies (async runtimes, long setups)

        // For brace-delimited languages, scan for matching braces to find method end.
        // Skipped for Python which uses indentation-based boundaries instead, since
        // dict/set literals (e.g. {"key": "value"}) would falsely terminate the method.
        if self.config.uses_braces {
            let start_line = lines[start_idx].trim();
            let mut in_string = false;
            let mut string_delimiter = '\0';
            let mut escaped = false;

            for ch in start_line.chars() {
                if escaped {
                    escaped = false;
                    continue;
                }

                match ch {
                    '\\' if in_string => {
                        escaped = true;
                    }
                    '"' | '\'' => {
                        if !in_string {
                            in_string = true;
                            string_delimiter = ch;
                        } else if ch == string_delimiter {
                            in_string = false;
                            string_delimiter = '\0';
                        }
                    }
                    '{' if !in_string => {
                        brace_depth += 1;
                        found_opening_brace = true;
                    }
                    '}' if !in_string => {
                        if found_opening_brace {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                method_end_line = Some(start_idx);
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // If we didn't find the end on the start line, continue searching
        if method_end_line.is_none() {
            for (i, line) in lines.iter().enumerate().skip(start_idx + 1) {
                let line = line.trim();

                // Limit search range - if we haven't found opening brace within reasonable distance,
                // something is wrong with the method detection
                if i > search_limit && !found_opening_brace {
                    break;
                }

                if self.config.uses_braces {
                    // Track brace depth for proper nesting (ignoring braces in strings)
                    let mut in_string = false;
                    let mut string_delimiter = '\0';
                    let mut escaped = false;

                    for ch in line.chars() {
                        if escaped {
                            escaped = false;
                            continue;
                        }

                        match ch {
                            '\\' if in_string => {
                                escaped = true;
                            }
                            '"' | '\'' => {
                                if !in_string {
                                    in_string = true;
                                    string_delimiter = ch;
                                } else if ch == string_delimiter {
                                    in_string = false;
                                    string_delimiter = '\0';
                                }
                            }
                            '{' if !in_string => {
                                brace_depth += 1;
                                found_opening_brace = true;
                            }
                            '}' if !in_string => {
                                if found_opening_brace {
                                    brace_depth -= 1;
                                    if brace_depth == 0 {
                                        method_end_line = Some(i);
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // If we found the method end by brace counting, stop
                    if method_end_line.is_some() {
                        break;
                    }
                }

                // Check if any end pattern matches (for indentation-based languages like Python)
                if !self.config.method_end_patterns.is_empty() {
                    // Note: `line` is shadowed to trimmed above, use lines[i] for indent
                    let original_line = lines[i];
                    let line_indent = original_line.len() - original_line.trim_start().len();
                    let start_line_indent =
                        lines[start_idx].len() - lines[start_idx].trim_start().len();

                    // Only consider end patterns at same or lower indentation (not nested code)
                    if line_indent <= start_line_indent {
                        for pattern in self.config.method_end_patterns {
                            if line == *pattern || line.starts_with(pattern) {
                                method_end_line = Some(i);
                                break;
                            }
                        }
                        if method_end_line.is_some() {
                            break;
                        }
                    }
                }
            }
        }

        let end_idx = method_end_line.unwrap_or(lines.len());
        Ok(Some((start_idx, end_idx)))
    }

    /// Parse all Gherkin step comments (with continuation line handling) from a slice
    /// of method lines. Returns normalized step positions that both step extraction
    /// and empty-step detection can consume.
    fn parse_step_positions(
        method_lines: &[&str],
        comment_prefix: &str,
    ) -> Result<Vec<StepPosition>> {
        let gherkin_comment_regex = Regex::new(&format!(
            r"{}\s*(Given|When|Then|And|But)\s+(.+)",
            regex::escape(comment_prefix)
        ))?;
        let continuation_regex = Regex::new(&format!(r"{}\s+(.+)", regex::escape(comment_prefix)))?;

        let mut step_positions: Vec<StepPosition> = Vec::new();
        let mut i = 0;
        while i < method_lines.len() {
            let line = method_lines[i].trim();
            if let Some(captures) = gherkin_comment_regex.captures(line) {
                let comment_start = i;
                let step_type = &captures[1];
                let mut step_text = captures[2].to_string();

                i += 1;
                while i < method_lines.len() {
                    let next_line = method_lines[i].trim();
                    if next_line.starts_with(comment_prefix) {
                        if let Some(cont_captures) = continuation_regex.captures(next_line) {
                            if !gherkin_comment_regex.is_match(next_line) {
                                step_text.push(' ');
                                step_text.push_str(cont_captures[1].trim());
                                i += 1;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                step_positions.push(StepPosition {
                    text: format!("{step_type} {step_text}"),
                    comment_start,
                    code_search_start: i,
                });
            } else {
                i += 1;
            }
        }

        Ok(step_positions)
    }

    /// Extract step text strings from method boundaries.
    fn extract_steps_from_boundaries_generic(
        &self,
        content: &str,
        start_idx: usize,
        end_idx: usize,
        comment_prefix: &str,
    ) -> Result<Vec<String>> {
        let lines: Vec<&str> = content.lines().collect();
        let method_lines: Vec<&str> = lines
            .iter()
            .take(end_idx)
            .skip(start_idx)
            .cloned()
            .collect();

        let step_positions = Self::parse_step_positions(&method_lines, comment_prefix)?;
        Ok(step_positions.into_iter().map(|sp| sp.text).collect())
    }

    /// Find Gherkin step comments that are not followed by any implementation code
    /// before the next step comment or end of method.
    fn find_empty_steps_from_boundaries_generic(
        &self,
        content: &str,
        start_idx: usize,
        end_idx: usize,
        comment_prefix: &str,
    ) -> Result<Vec<String>> {
        let lines: Vec<&str> = content.lines().collect();
        let method_lines: Vec<&str> = lines
            .iter()
            .take(end_idx)
            .skip(start_idx)
            .cloned()
            .collect();

        let step_positions = Self::parse_step_positions(&method_lines, comment_prefix)?;
        let mut empty_steps = Vec::new();

        for (idx, step) in step_positions.iter().enumerate() {
            let code_end = if idx + 1 < step_positions.len() {
                step_positions[idx + 1].comment_start
            } else {
                method_lines.len()
            };

            let has_code = method_lines[step.code_search_start..code_end]
                .iter()
                .any(|line| {
                    let trimmed = line.trim();
                    !trimmed.is_empty() && !trimmed.starts_with(comment_prefix)
                });

            if !has_code {
                empty_steps.push(step.text.clone());
            }
        }

        Ok(empty_steps)
    }
}

pub struct StepFinder {
    language: Language,
}

impl StepFinder {
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    /// Find all implemented steps in a test file using comment-based approach
    pub fn find_implemented_steps(&self, file_path: &Path) -> Result<Vec<String>> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read test file: {}", file_path.display()))?;

        let comment_prefix = match self.language {
            Language::Python => "#",
            _ => "//", // Rust, ODBC, JDBC, CSharp, JavaScript all use //
        };

        self.find_steps_generic(&content, comment_prefix)
    }

    /// Generic step finding method that works for all languages with continuation support
    fn find_steps_generic(&self, content: &str, comment_prefix: &str) -> Result<Vec<String>> {
        let mut steps = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        // Create regex for Gherkin comments
        let comment_regex = format!(
            r"{}\s*(Given|When|Then|And|But)\s+(.+)",
            regex::escape(comment_prefix)
        );
        let gherkin_comment_regex = Regex::new(&comment_regex)?;
        let continuation_regex = format!(r"{}\s+(.+)", regex::escape(comment_prefix));
        let continuation_comment_regex = Regex::new(&continuation_regex)?;

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            if let Some(captures) = gherkin_comment_regex.captures(line) {
                let step_type = &captures[1];
                let mut step_text = captures[2].to_string();

                // Check for continuation lines - only if next line is purely a comment
                i += 1;
                while i < lines.len() {
                    let next_line = lines[i].trim();
                    // Only treat as continuation if the line starts with comment prefix (no code before comment)
                    if next_line.starts_with(comment_prefix) {
                        if let Some(cont_captures) = continuation_comment_regex.captures(next_line)
                        {
                            // Check if this is a continuation (doesn't start with Given/When/Then/And/But)
                            let cont_text = cont_captures[1].trim();
                            if !gherkin_comment_regex.is_match(next_line) {
                                step_text.push(' ');
                                step_text.push_str(cont_text);
                                i += 1;
                            } else {
                                // This is a new step, don't consume it
                                break;
                            }
                        } else {
                            // No more continuation lines
                            break;
                        }
                    } else {
                        // Next line is not a pure comment line, stop looking for continuations
                        break;
                    }
                }

                steps.push(format!("{} {}", step_type, step_text));
            } else {
                i += 1;
            }
        }

        Ok(steps)
    }

    fn comment_prefix(&self) -> &'static str {
        match self.language {
            Language::Python => "#",
            _ => "//",
        }
    }

    fn language_config(&self) -> LanguageConfig {
        match self.language {
            Language::Rust => LanguageConfig::rust(),
            Language::Odbc => LanguageConfig::odbc(),
            Language::Jdbc => LanguageConfig::jdbc(),
            Language::Python => LanguageConfig::python(),
            Language::CSharp => LanguageConfig::csharp(),
            Language::JavaScript => LanguageConfig::javascript(),
        }
    }

    /// Find implemented steps within a specific test method
    pub fn find_steps_in_method(&self, file_path: &Path, method_name: &str) -> Result<Vec<String>> {
        let (steps, _) = self.find_steps_and_empty_steps_in_method(file_path, method_name)?;
        Ok(steps)
    }

    /// Find Gherkin step comments in a method that have no implementation code following them
    pub fn find_empty_steps_in_method(
        &self,
        file_path: &Path,
        method_name: &str,
    ) -> Result<Vec<String>> {
        let (_, empty_steps) = self.find_steps_and_empty_steps_in_method(file_path, method_name)?;
        Ok(empty_steps)
    }

    /// Find both implemented steps and empty steps in a single pass, reading the file
    /// and computing method boundaries only once.
    pub fn find_steps_and_empty_steps_in_method(
        &self,
        file_path: &Path,
        method_name: &str,
    ) -> Result<(Vec<String>, Vec<String>)> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read test file: {}", file_path.display()))?;

        let comment_prefix = self.comment_prefix();
        let boundary_finder = MethodBoundaryFinder::new(self.language_config());

        if let Some((start_idx, end_idx)) =
            boundary_finder.find_method_boundaries(&content, method_name)?
        {
            let mut steps = boundary_finder.extract_steps_from_boundaries_generic(
                &content,
                start_idx,
                end_idx,
                comment_prefix,
            )?;
            if steps.is_empty() {
                let lines: Vec<&str> = content.lines().collect();
                steps = boundary_finder.extract_steps_from_boundaries_generic(
                    &content,
                    start_idx,
                    lines.len(),
                    comment_prefix,
                )?;
            }
            let empty_steps = boundary_finder.find_empty_steps_from_boundaries_generic(
                &content,
                start_idx,
                end_idx,
                comment_prefix,
            )?;
            Ok((steps, empty_steps))
        } else {
            Ok((vec![], vec![]))
        }
    }

    /// Find test functions/methods with line numbers that match a scenario name
    pub fn find_test_methods_with_lines(
        &self,
        file_path: &Path,
        scenario_name: &str,
    ) -> Result<Vec<(String, usize)>> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read test file: {}", file_path.display()))?;

        match self.language {
            Language::Rust => self.find_rust_test_methods_with_lines(&content, scenario_name),
            Language::Odbc => self.find_odbc_test_methods_with_lines(&content, scenario_name),
            Language::Jdbc => self.find_jdbc_test_methods_with_lines(&content, scenario_name),
            Language::Python => self.find_python_test_methods_with_lines(&content, scenario_name),
            Language::CSharp => self.find_csharp_test_methods_with_lines(&content, scenario_name),
            Language::JavaScript => {
                self.find_javascript_test_methods_with_lines(&content, scenario_name)
            }
        }
    }

    fn find_rust_test_methods_with_lines(
        &self,
        content: &str,
        scenario_name: &str,
    ) -> Result<Vec<(String, usize)>> {
        let boundary_finder = MethodBoundaryFinder::new(LanguageConfig::rust());
        let all_methods = boundary_finder.find_all_test_methods_with_lines(content)?;

        // Generate possible function names from scenario name
        let snake_scenario = to_snake_case(scenario_name);

        let matching_methods = all_methods
            .into_iter()
            .filter(|(method_name, _line)| {
                strings_match_normalized(
                    clean_method_name(method_name),
                    clean_method_name(&snake_scenario),
                )
            })
            .collect();

        Ok(matching_methods)
    }

    // Implementations for *_with_lines methods using MethodBoundaryFinder
    fn find_odbc_test_methods_with_lines(
        &self,
        content: &str,
        scenario_name: &str,
    ) -> Result<Vec<(String, usize)>> {
        let boundary_finder = MethodBoundaryFinder::new(LanguageConfig::odbc());
        let all_methods = boundary_finder.find_all_test_methods_with_lines(content)?;

        let snake_scenario = to_snake_case(scenario_name);
        let pascal_scenario = to_pascal_case(scenario_name);

        let matching_methods = all_methods
            .into_iter()
            .filter(|(method_name, _line)| {
                strings_match_normalized(method_name, scenario_name)
                    || strings_match_normalized(method_name, &snake_scenario)
                    || strings_match_normalized(method_name, &pascal_scenario)
            })
            .collect();

        Ok(matching_methods)
    }

    fn find_jdbc_test_methods_with_lines(
        &self,
        content: &str,
        scenario_name: &str,
    ) -> Result<Vec<(String, usize)>> {
        let boundary_finder = MethodBoundaryFinder::new(LanguageConfig::jdbc());
        let all_methods = boundary_finder.find_all_test_methods_with_lines(content)?;

        let pascal_scenario = to_pascal_case(scenario_name);

        let matching_methods = all_methods
            .into_iter()
            .filter(|(method_name, _line)| strings_match_normalized(method_name, &pascal_scenario))
            .collect();

        Ok(matching_methods)
    }

    fn find_python_test_methods_with_lines(
        &self,
        content: &str,
        scenario_name: &str,
    ) -> Result<Vec<(String, usize)>> {
        let boundary_finder = MethodBoundaryFinder::new(LanguageConfig::python());
        let all_methods = boundary_finder.find_all_test_methods_with_lines(content)?;

        let snake_scenario = to_snake_case(scenario_name);
        let expected_method_name = format!("test_{}", snake_scenario);

        let matching_methods = all_methods
            .into_iter()
            .filter(|(method_name, _line)| {
                strings_match_normalized(method_name, &expected_method_name)
            })
            .collect();

        Ok(matching_methods)
    }

    fn find_csharp_test_methods_with_lines(
        &self,
        content: &str,
        scenario_name: &str,
    ) -> Result<Vec<(String, usize)>> {
        let boundary_finder = MethodBoundaryFinder::new(LanguageConfig::csharp());
        let all_methods = boundary_finder.find_all_test_methods_with_lines(content)?;

        let pascal_scenario = to_pascal_case(scenario_name);

        let matching_methods = all_methods
            .into_iter()
            .filter(|(method_name, _line)| strings_match_normalized(method_name, &pascal_scenario))
            .collect();

        Ok(matching_methods)
    }

    fn find_javascript_test_methods_with_lines(
        &self,
        content: &str,
        scenario_name: &str,
    ) -> Result<Vec<(String, usize)>> {
        // For JavaScript, we need to look for test functions differently since they don't use annotations
        let mut methods = Vec::new();
        let lines: Vec<&str> = content.lines().collect();

        let test_regex = Regex::new(r#"(?:it|test)\s*\(\s*['"]([^'"]+)['"]"#)?;

        for (i, line) in lines.iter().enumerate() {
            if let Some(captures) = test_regex.captures(line.trim()) {
                let test_name = &captures[1];
                if strings_match_normalized(test_name, scenario_name) {
                    methods.push((test_name.to_string(), i + 1)); // +1 for 1-indexed line numbers
                }
            }
        }

        Ok(methods)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that Python method boundary detection correctly handles nested constructs.
    /// This is a regression test for the fix where nested `import` or `def` statements
    /// inside a test method were incorrectly terminating the method boundary.
    #[test]
    fn test_python_method_boundary_with_nested_constructs() {
        let boundary_finder = MethodBoundaryFinder::new(LanguageConfig::python());

        // Python test with nested import and nested def - these should NOT end the method
        let content = r#"
import pytest

def test_something_with_nested_constructs():
    # Given the system is ready
    value = 42
    
    # Nested import inside the function (e.g., lazy import)
    import json
    
    # Nested function definition
    def helper():
        return "helper result"
    
    # When we process the data
    result = helper()
    
    # Then we should get the expected result
    assert result == "helper result"
    
    # Nested async def
    async def async_helper():
        return "async result"

def test_next_method():
    # This is a different test
    pass
"#;

        // Find boundaries for test_something_with_nested_constructs
        let boundaries = boundary_finder
            .find_method_boundaries(content, "test_something_with_nested_constructs")
            .expect("Should find method boundaries");

        assert!(boundaries.is_some(), "Should find the method");
        let (start, end) = boundaries.unwrap();

        // The method should include all the nested constructs
        let lines: Vec<&str> = content.lines().collect();

        // Method should start at the def line
        assert!(
            lines[start].contains("def test_something_with_nested_constructs"),
            "Start line should be the method definition"
        );

        // Method should end at or before the next top-level def (test_next_method)
        // The end should be after the nested async def but before the next test
        let method_content: String = lines[start..end].join("\n");

        // Verify the nested import is included
        assert!(
            method_content.contains("import json"),
            "Method should include nested import"
        );

        // Verify the nested def is included
        assert!(
            method_content.contains("def helper():"),
            "Method should include nested def"
        );

        // Verify the nested async def is included
        assert!(
            method_content.contains("async def async_helper():"),
            "Method should include nested async def"
        );

        // Verify the next test method is NOT included
        assert!(
            !method_content.contains("def test_next_method"),
            "Method should NOT include the next test method"
        );
    }

    /// Test that top-level constructs still correctly end methods
    #[test]
    fn test_python_method_boundary_ends_at_top_level_def() {
        let boundary_finder = MethodBoundaryFinder::new(LanguageConfig::python());

        let content = r#"
def test_first():
    # Given something
    x = 1
    # Then check
    assert x == 1

def test_second():
    # Given something else
    y = 2
"#;

        let boundaries = boundary_finder
            .find_method_boundaries(content, "test_first")
            .expect("Should find method boundaries");

        assert!(boundaries.is_some());
        let (start, end) = boundaries.unwrap();

        let lines: Vec<&str> = content.lines().collect();
        let method_content: String = lines[start..end].join("\n");

        // Should include the first test's content
        assert!(method_content.contains("x = 1"));
        assert!(method_content.contains("assert x == 1"));

        // Should NOT include the second test
        assert!(!method_content.contains("def test_second"));
        assert!(!method_content.contains("y = 2"));
    }

    /// Regression test: dict literals like {"key": "value"} in Python should NOT
    /// cause the method boundary to end prematurely due to brace counting.
    #[test]
    fn test_python_method_boundary_not_truncated_by_dict_literals() {
        let boundary_finder = MethodBoundaryFinder::new(LanguageConfig::python());

        let content = r#"
class TestFetchAll:
    def test_fetch_with_nulls(self, cursor):
        # Given Snowflake client is logged in
        cursor.execute("SELECT 1")

        # And Row 1 should have correct typed values for all columns
        row1 = result.iloc[0]
        assert row1["OBJ_COL"] == {"key": "value"}

        # And Row 2 should have nulls for all nullable columns
        row2 = result.iloc[1]
        assert row2["TEXT_COL"] is None

    def test_next_method(self):
        pass
"#;

        let boundaries = boundary_finder
            .find_method_boundaries(content, "test_fetch_with_nulls")
            .expect("Should find method boundaries");

        assert!(boundaries.is_some(), "Should find the method");
        let (start, end) = boundaries.unwrap();

        let lines: Vec<&str> = content.lines().collect();
        let method_content: String = lines[start..end].join("\n");

        assert!(
            method_content.contains(r#"{"key": "value"}"#),
            "Method should include dict literal line"
        );
        assert!(
            method_content.contains("Row 2 should have nulls"),
            "Method should include steps after dict literal"
        );
        assert!(
            method_content.contains("row2[\"TEXT_COL\"] is None"),
            "Method should include assertions after dict literal"
        );
        assert!(
            !method_content.contains("def test_next_method"),
            "Method should NOT include the next test method"
        );
    }

    #[test]
    fn test_jdbc_parameterized_test_with_method_source_extracts_real_method_name() {
        let finder = MethodBoundaryFinder::new(LanguageConfig::jdbc());
        let content = r#"
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

public class IntTests {
    @ParameterizedTest
    @MethodSource("intTypeSynonyms")
    public void shouldSelectIntegerLiteralsForIntAndSynonyms(String typeName) throws Exception {
        // Given Snowflake client is logged in
    }
}
"#;

        let methods = finder
            .find_all_test_methods_with_lines(content)
            .expect("Should parse methods");

        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].0, "shouldSelectIntegerLiteralsForIntAndSynonyms");
    }
}
