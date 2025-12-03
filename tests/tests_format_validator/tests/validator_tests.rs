use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use tests_format_validator::GherkinValidator;

#[test]
fn should_pass_validation_when_all_steps_are_implemented() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create a complete feature file
    workspace.create_feature_file(
        "auth",
        "login",
        TestImplementations::create_complete_login_feature(),
    )?;

    // Create matching Rust test
    workspace.create_rust_test(
        "auth",
        "login",
        TestImplementations::create_complete_rust_login_test(),
    )?;

    // Create matching Java test
    workspace.create_java_test(
        "auth",
        "Login",
        TestImplementations::create_complete_jdbc_login_test(),
    )?;

    // Create matching C++ test
    workspace.create_cpp_test(
        "auth",
        "login",
        TestImplementations::create_complete_odbc_login_test(),
    )?;

    let validator = workspace.get_validator()?;
    let results = validator.validate_all_features()?;

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(
        result.feature_file.file_stem().unwrap().to_str().unwrap(),
        "login"
    );
    assert_eq!(result.validations.len(), 3); // Rust, Jdbc, and Odbc

    // Both languages should pass
    for validation in &result.validations {
        assert!(
            validation.test_file_found,
            "Language {:?} should find test file",
            validation.language
        );
        assert!(
            validation.missing_steps.is_empty(),
            "Language {:?} should have no missing steps, but missing: {:?}",
            validation.language,
            validation.missing_steps
        );
        assert_eq!(validation.implemented_steps.len(), 4);
    }

    Ok(())
}

#[test]
fn should_report_missing_files_when_no_test_files_exist() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create feature file but no corresponding test files
    workspace.create_feature_file(
        "auth",
        "missing_file",
        TestImplementations::create_missing_file_feature(),
    )?;

    let validator = workspace.get_validator()?;
    let results = validator.validate_all_features()?;

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(
        result.feature_file.file_stem().unwrap().to_str().unwrap(),
        "missing_file"
    );
    assert_eq!(result.validations.len(), 2); // Rust and Jdbc

    // Both languages should report missing files
    for validation in &result.validations {
        assert!(
            !validation.test_file_found,
            "Language {:?} should report missing test file",
            validation.language
        );
        assert!(validation.test_file_path.is_none());
        assert_eq!(validation.missing_steps.len(), 0); // No individual steps when file doesn't exist
        assert!(!validation.warnings.is_empty());
        assert!(validation.warnings[0].contains("No test file found"));
    }

    Ok(())
}

#[test]
fn should_report_missing_functions_when_method_names_dont_match_scenarios() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create feature file
    workspace.create_feature_file(
        "query",
        "missing_function",
        TestImplementations::create_missing_function_feature(),
    )?;

    // Create Rust test file but without the expected function name
    workspace.create_rust_test(
        "query",
        "missing_function",
        TestImplementations::create_rust_test_with_wrong_function_name(),
    )?;

    // Create Java test file but without the expected function name
    workspace.create_java_test(
        "query",
        "MissingFunction",
        TestImplementations::create_jdbc_test_with_wrong_function_name(),
    )?;

    let validator = workspace.get_validator()?;
    let results = validator.validate_all_features()?;

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(
        result.feature_file.file_stem().unwrap().to_str().unwrap(),
        "missing_function"
    );

    // Both languages should find test files but report missing test methods
    for validation in &result.validations {
        assert!(
            validation.test_file_found,
            "Language {:?} should find test file",
            validation.language
        );
        assert!(validation.test_file_path.is_some());

        // Should have warnings about missing test methods
        assert!(!validation.warnings.is_empty());
        assert!(
            validation
                .warnings
                .iter()
                .any(|w| w.contains("No test method found for scenario"))
        );
    }

    Ok(())
}

#[test]
fn should_report_missing_steps_when_some_gherkin_comments_are_missing() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create feature file
    workspace.create_feature_file(
        "auth",
        "missing_step",
        TestImplementations::create_missing_step_feature(),
    )?;

    // Create Rust test with missing steps
    workspace.create_rust_test(
        "auth",
        "missing_step",
        TestImplementations::create_rust_test_with_missing_step(),
    )?;

    // Create Java test with missing steps
    workspace.create_java_test(
        "auth",
        "MissingStep",
        TestImplementations::create_jdbc_test_with_missing_step(),
    )?;

    let validator = workspace.get_validator()?;
    let results = validator.validate_all_features()?;

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(
        result.feature_file.file_stem().unwrap().to_str().unwrap(),
        "missing_step"
    );

    // Both languages should find test files and methods but report missing steps
    for validation in &result.validations {
        assert!(
            validation.test_file_found,
            "Language {:?} should find test file",
            validation.language
        );
        assert!(validation.test_file_path.is_some());

        // Should have exactly one missing step
        assert_eq!(
            validation.missing_steps.len(),
            1,
            "Language {:?} should have exactly 1 missing step, but has: {:?}",
            validation.language,
            validation.missing_steps
        );
        assert!(validation.missing_steps[0].contains("And user session should be created"));

        // Should have implemented steps (the other 3)
        assert_eq!(validation.implemented_steps.len(), 3);

        // Should have method-specific missing steps info
        assert_eq!(validation.missing_steps_by_method.len(), 1);
        let method_validation = &validation.missing_steps_by_method[0];
        assert_eq!(method_validation.missing_steps.len(), 1);
        assert!(method_validation.missing_steps[0].contains("And user session should be created"));
    }

    Ok(())
}

#[test]
fn should_handle_mixed_complete_and_incomplete_scenarios() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create a feature with multiple scenarios - some complete, some incomplete
    workspace.create_feature_file(
        "query",
        "mixed",
        TestImplementations::create_mixed_scenarios_feature(),
    )?;

    // Create Rust test with one complete, one incomplete scenario
    workspace.create_rust_test(
        "query",
        "mixed",
        TestImplementations::create_rust_mixed_scenarios_test(),
    )?;

    // Create Java test with similar pattern
    workspace.create_java_test(
        "query",
        "Mixed",
        TestImplementations::create_jdbc_mixed_scenarios_test(),
    )?;

    let validator = workspace.get_validator()?;
    let results = validator.validate_all_features()?;

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(
        result.feature_file.file_stem().unwrap().to_str().unwrap(),
        "mixed"
    );

    for validation in &result.validations {
        assert!(validation.test_file_found);

        // Should have one missing step (from incomplete scenario)
        assert_eq!(validation.missing_steps.len(), 1);
        assert!(validation.missing_steps[0].contains("And orders should be sorted by date"));

        // Should have implemented steps from both scenarios
        // The feature has: Given, When, Then (3 steps) repeated in both scenarios
        // Plus one unique "And orders should be sorted by date" step
        // Total unique steps: 4, with 3 implemented = 3 implemented steps
        assert_eq!(validation.implemented_steps.len(), 3);

        // Should have method-specific info for the incomplete scenario
        assert_eq!(validation.missing_steps_by_method.len(), 1);
        let method_validation = &validation.missing_steps_by_method[0];
        assert_eq!(method_validation.scenario_name, "Incomplete scenario");
        assert_eq!(method_validation.missing_steps.len(), 1);
    }

    Ok(())
}

#[test]
fn should_detect_orphaned_test_files_and_methods() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create a feature file
    workspace.create_feature_file(
        "auth",
        "login",
        TestImplementations::create_complete_login_feature(),
    )?;

    // Create matching test with extra orphaned method
    workspace.create_rust_test(
        "auth",
        "login",
        TestImplementations::create_rust_test_with_orphaned_method(),
    )?;

    // Create an orphaned test file that doesn't match any feature
    workspace.create_rust_test(
        "query",
        "orphaned_file",
        TestImplementations::create_orphaned_rust_test(),
    )?;

    let validator = workspace.get_validator()?;
    let orphan_results = validator.find_orphaned_tests()?;

    assert_eq!(orphan_results.len(), 1); // Only Rust has orphaned tests
    let rust_orphans = &orphan_results[0];
    assert_eq!(
        rust_orphans.language,
        tests_format_validator::Language::Rust
    );

    // Should find both the file with orphaned methods and the completely orphaned file
    assert_eq!(rust_orphans.orphaned_files.len(), 2);

    // Check for the file with orphaned methods
    let file_with_orphaned_methods = rust_orphans
        .orphaned_files
        .iter()
        .find(|f| f.file_path.file_stem().and_then(|s| s.to_str()) == Some("login"))
        .expect("Should find login file with orphaned methods");
    assert_eq!(file_with_orphaned_methods.orphaned_methods.len(), 1);
    assert_eq!(
        file_with_orphaned_methods.orphaned_methods[0],
        "orphaned_test_method"
    );

    // Check for the completely orphaned file
    let orphaned_file = rust_orphans
        .orphaned_files
        .iter()
        .find(|f| f.file_path.file_stem().and_then(|s| s.to_str()) == Some("orphaned_file"))
        .expect("Should find completely orphaned file");
    assert!(orphaned_file.orphaned_methods.is_empty()); // Orphaned files don't list methods

    Ok(())
}

#[test]
fn should_handle_nested_blocks_correctly() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create a feature file
    workspace.create_feature_file(
        "query",
        "nested_blocks",
        TestImplementations::create_nested_blocks_feature(),
    )?;

    // Create Java test with nested blocks
    workspace.create_java_test(
        "query",
        "NestedBlocks",
        TestImplementations::create_java_test_with_nested_blocks(),
    )?;

    // Create Rust test with nested blocks
    workspace.create_rust_test(
        "query",
        "nested_blocks",
        TestImplementations::create_rust_test_with_nested_blocks(),
    )?;

    // Create C++ test with nested blocks
    workspace.create_cpp_test(
        "query",
        "nested_blocks",
        TestImplementations::create_cpp_test_with_nested_blocks(),
    )?;

    let validator = workspace.get_validator()?;
    let results = validator.validate_all_features()?;

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(
        result.feature_file.file_stem().unwrap().to_str().unwrap(),
        "nested_blocks"
    );

    // All languages should correctly identify steps within nested blocks
    for validation in &result.validations {
        assert!(
            validation.test_file_found,
            "Language {:?} should find test file",
            validation.language
        );
        assert!(
            validation.missing_steps.is_empty(),
            "Language {:?} should have no missing steps, but missing: {:?}",
            validation.language,
            validation.missing_steps
        );
        assert_eq!(
            validation.implemented_steps.len(),
            4,
            "Language {:?} should find all 4 steps",
            validation.language
        );
    }

    Ok(())
}

#[test]
fn should_ignore_braces_in_strings() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create a feature file
    workspace.create_feature_file(
        "query",
        "string_braces",
        TestImplementations::create_string_braces_feature(),
    )?;

    // Create Java test with braces in strings
    workspace.create_java_test(
        "query",
        "StringBraces",
        TestImplementations::create_java_test_with_string_braces(),
    )?;

    // Create Rust test with braces in strings
    workspace.create_rust_test(
        "query",
        "string_braces",
        TestImplementations::create_rust_test_with_string_braces(),
    )?;

    let validator = workspace.get_validator()?;
    let results = validator.validate_all_features()?;

    assert_eq!(results.len(), 1);
    let result = &results[0];

    // All languages should correctly identify steps despite braces in strings
    for validation in &result.validations {
        assert!(
            validation.test_file_found,
            "Language {:?} should find test file",
            validation.language
        );
        assert!(
            validation.missing_steps.is_empty(),
            "Language {:?} should have no missing steps, but missing: {:?}",
            validation.language,
            validation.missing_steps
        );
        assert_eq!(
            validation.implemented_steps.len(),
            3,
            "Language {:?} should find all 3 steps",
            validation.language
        );
    }

    Ok(())
}

// ===== Breaking Change Detection Tests =====

#[test]
fn should_detect_simple_breaking_change_annotations_in_cpp() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create feature with Breaking Change annotation
    workspace.create_feature_file(
        "auth",
        "simple_breaking_change",
        TestImplementations::create_simple_breaking_change_feature(),
    )?;

    // Create C++ test with Breaking Change macros
    workspace.create_cpp_test(
        "auth",
        "simple_breaking_change",
        TestImplementations::create_cpp_test_with_simple_breaking_change(),
    )?;

    let validator = workspace.get_validator()?;

    let enhanced_results = validator.validate_all_with_breaking_changes()?;

    // Check Breaking Change report
    let breaking_changes_report = &enhanced_results.behavior_differences_report;

    assert!(
        !breaking_changes_report
            .behavior_difference_descriptions
            .is_empty(),
        "Should find Breaking Change descriptions"
    );
    assert!(
        !breaking_changes_report
            .behavior_differences_by_language
            .is_empty(),
        "Should find Breaking Changes by language"
    );

    // Check ODBC Breaking Changes
    let odbc_breaking_changes = breaking_changes_report
        .behavior_differences_by_language
        .get("odbc");
    assert!(
        odbc_breaking_changes.is_some(),
        "Should find ODBC Breaking Changes"
    );

    let odbc_breaking_changes = odbc_breaking_changes.unwrap();
    assert_eq!(
        odbc_breaking_changes.len(),
        1,
        "Should find exactly one Breaking Change"
    );

    let breaking_change = &odbc_breaking_changes[0];
    assert_eq!(
        breaking_change.behavior_difference_id, "BD#1",
        "Should find BD#1"
    );
    assert_eq!(
        breaking_change.implementations.len(),
        1,
        "Should find one implementation"
    );

    let impl_info = &breaking_change.implementations[0];
    assert_eq!(
        impl_info.test_method,
        "should authenticate using private key"
    );
    assert!(impl_info.test_file.contains("simple_breaking_change.cpp"));
    assert!(
        impl_info.test_line > 0,
        "Should have valid test line number"
    );

    // Should have both new and old behavior
    assert!(
        impl_info.new_behaviour_file.is_some(),
        "Should find new behavior"
    );
    assert!(
        impl_info.new_behaviour_line.is_some(),
        "Should find new behavior line"
    );
    assert!(
        impl_info.old_behaviour_file.is_some(),
        "Should find old behavior"
    );
    assert!(
        impl_info.old_behaviour_line.is_some(),
        "Should find old behavior line"
    );

    Ok(())
}

#[test]
fn should_detect_cross_file_breaking_change_in_helper_methods() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create feature with Breaking Change annotation
    workspace.create_feature_file(
        "auth",
        "cross_file_breaking_change",
        TestImplementations::create_cross_file_breaking_change_feature(),
    )?;

    // Create main C++ test that calls helper methods
    workspace.create_cpp_test(
        "auth",
        "cross_file_breaking_change",
        TestImplementations::create_cpp_test_with_cross_file_breaking_change(),
    )?;

    // Create common helper file with Breaking Change implementations
    workspace.create_cpp_common_file(
        "auth_helpers",
        TestImplementations::create_cpp_common_helper_with_breaking_change(),
    )?;

    let validator = workspace.get_validator()?;
    let enhanced_results = validator.validate_all_with_breaking_changes()?;

    // Check Breaking Change report
    let breaking_changes_report = &enhanced_results.behavior_differences_report;
    let odbc_breaking_changes = breaking_changes_report
        .behavior_differences_by_language
        .get("odbc");
    assert!(
        odbc_breaking_changes.is_some(),
        "Should find ODBC Breaking Changes"
    );

    let odbc_breaking_changes = odbc_breaking_changes.unwrap();
    assert_eq!(odbc_breaking_changes.len(), 2, "Should find BD#2 and BD#3");

    // Check that both Breaking Changes are found
    let breaking_change_ids: Vec<&str> = odbc_breaking_changes
        .iter()
        .map(|b| b.behavior_difference_id.as_str())
        .collect();
    assert!(breaking_change_ids.contains(&"BD#2"), "Should find BD#2");
    assert!(breaking_change_ids.contains(&"BD#3"), "Should find BD#3");

    // Check that implementations point to the helper file
    for breaking_change in odbc_breaking_changes {
        let impl_info = &breaking_change.implementations[0];
        assert_eq!(
            impl_info.test_method,
            "should authenticate using private key with helper"
        );
        assert!(
            impl_info
                .test_file
                .contains("cross_file_breaking_change.cpp")
        );

        // Breaking Change should be found in the helper file
        if let Some(new_file) = &impl_info.new_behaviour_file {
            assert!(
                new_file.contains("auth_helpers.cpp"),
                "New behavior should be in helper file"
            );
        }
        if let Some(old_file) = &impl_info.old_behaviour_file {
            assert!(
                old_file.contains("auth_helpers.cpp"),
                "Old behavior should be in helper file"
            );
        }
    }

    Ok(())
}

#[test]
fn should_detect_nested_helper_method_breaking_change() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create feature with Breaking Change annotation
    workspace.create_feature_file(
        "auth",
        "nested_breaking_change",
        TestImplementations::create_nested_breaking_change_feature(),
    )?;

    // Create C++ test with nested helper calls
    workspace.create_cpp_test(
        "auth",
        "nested_breaking_change",
        TestImplementations::create_cpp_test_with_nested_breaking_change(),
    )?;

    let validator = workspace.get_validator()?;
    let enhanced_results = validator.validate_all_with_breaking_changes()?;

    // Check Breaking Change report
    let breaking_changes_report = &enhanced_results.behavior_differences_report;
    let odbc_breaking_changes = breaking_changes_report
        .behavior_differences_by_language
        .get("odbc");
    assert!(
        odbc_breaking_changes.is_some(),
        "Should find ODBC Breaking Changes"
    );

    let odbc_breaking_changes = odbc_breaking_changes.unwrap();
    assert_eq!(odbc_breaking_changes.len(), 1, "Should find BD#4");

    let breaking_change = &odbc_breaking_changes[0];
    assert_eq!(breaking_change.behavior_difference_id, "BD#4");

    let impl_info = &breaking_change.implementations[0];
    assert_eq!(impl_info.test_method, "should handle nested authentication");

    // Should find the Breaking Change in the deeply nested helper method
    assert!(
        impl_info.new_behaviour_line.is_some(),
        "Should find Breaking Change in nested helper"
    );

    Ok(())
}

#[test]
fn should_only_find_breaking_changes_for_scenarios_with_breaking_change_annotation() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create feature with mixed scenarios - some with Breaking Change, some without
    workspace.create_feature_file(
        "auth",
        "mixed_breaking_change",
        TestImplementations::create_mixed_breaking_change_feature(),
    )?;

    // Create C++ test with Breaking Changes in both scenarios
    workspace.create_cpp_test(
        "auth",
        "mixed_breaking_change",
        TestImplementations::create_cpp_test_with_mixed_breaking_change(),
    )?;

    let validator = workspace.get_validator()?;
    let enhanced_results = validator.validate_all_with_breaking_changes()?;

    // Check Breaking Change report
    let breaking_changes_report = &enhanced_results.behavior_differences_report;
    let odbc_breaking_changes = breaking_changes_report
        .behavior_differences_by_language
        .get("odbc");
    assert!(
        odbc_breaking_changes.is_some(),
        "Should find ODBC Breaking Changes"
    );

    let odbc_breaking_changes = odbc_breaking_changes.unwrap();
    // Should now find Breaking Changes for both scenarios since both have @odbc annotation
    assert_eq!(
        odbc_breaking_changes.len(),
        2,
        "Should find BD#5 and BD#6 (both scenarios have @odbc)"
    );

    // Sort by Breaking Change ID to ensure consistent order
    let mut sorted_breaking_changes = odbc_breaking_changes.clone();
    sorted_breaking_changes.sort_by(|a, b| a.behavior_difference_id.cmp(&b.behavior_difference_id));

    assert_eq!(sorted_breaking_changes[0].behavior_difference_id, "BD#5");
    assert_eq!(
        sorted_breaking_changes[0].implementations[0].test_method,
        "should authenticate with breaking_change annotation"
    );

    assert_eq!(sorted_breaking_changes[1].behavior_difference_id, "BD#6");
    assert_eq!(
        sorted_breaking_changes[1].implementations[0].test_method,
        "should authenticate without breaking_change annotation"
    );

    Ok(())
}

#[test]
fn should_find_correct_line_numbers_for_breaking_change_locations() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create feature with Breaking Change annotation
    workspace.create_feature_file(
        "auth",
        "line_numbers",
        TestImplementations::create_line_numbers_breaking_change_feature(),
    )?;

    // Create C++ test with specific line arrangements
    workspace.create_cpp_test(
        "auth",
        "line_numbers",
        TestImplementations::create_cpp_test_with_specific_line_numbers(),
    )?;

    let validator = workspace.get_validator()?;
    let enhanced_results = validator.validate_all_with_breaking_changes()?;

    // Check Breaking Change report
    let breaking_changes_report = &enhanced_results.behavior_differences_report;
    let odbc_breaking_changes = breaking_changes_report
        .behavior_differences_by_language
        .get("odbc");
    assert!(
        odbc_breaking_changes.is_some(),
        "Should find ODBC Breaking Changes"
    );

    let odbc_breaking_changes = odbc_breaking_changes.unwrap();
    assert_eq!(odbc_breaking_changes.len(), 1, "Should find BD#7");

    let breaking_change = &odbc_breaking_changes[0];
    let impl_info = &breaking_change.implementations[0];

    // Check that line numbers are accurate
    assert_eq!(impl_info.test_line, 3, "Test should start at line 3"); // TEST_CASE line
    assert_eq!(
        impl_info.new_behaviour_line.unwrap(),
        7,
        "NEW_DRIVER_ONLY should be at line 7"
    );
    assert_eq!(
        impl_info.old_behaviour_line.unwrap(),
        12,
        "OLD_DRIVER_ONLY should be at line 12"
    );

    Ok(())
}

#[test]
fn should_handle_multiple_breaking_changes_in_single_test_method() -> Result<()> {
    let workspace = TestWorkspace::new()?;

    // Create feature with Breaking Change annotation
    workspace.create_feature_file(
        "auth",
        "multiple_breaking_changes",
        TestImplementations::create_multiple_breaking_changes_feature(),
    )?;

    // Create C++ test with multiple Breaking Change macros
    workspace.create_cpp_test(
        "auth",
        "multiple_breaking_changes",
        TestImplementations::create_cpp_test_with_multiple_breaking_changes(),
    )?;

    let validator = workspace.get_validator()?;
    let enhanced_results = validator.validate_all_with_breaking_changes()?;

    // Check Breaking Change report
    let breaking_changes_report = &enhanced_results.behavior_differences_report;
    let odbc_breaking_changes = breaking_changes_report
        .behavior_differences_by_language
        .get("odbc");
    assert!(
        odbc_breaking_changes.is_some(),
        "Should find ODBC Breaking Changes"
    );

    let odbc_breaking_changes = odbc_breaking_changes.unwrap();
    assert_eq!(
        odbc_breaking_changes.len(),
        3,
        "Should find BD#8, BD#9, and BD#10"
    );

    // All Breaking Changes should point to the same test method
    for breaking_change in odbc_breaking_changes {
        assert_eq!(
            breaking_change.implementations[0].test_method,
            "should handle multiple authentication methods"
        );
        assert!(
            breaking_change.implementations[0].test_line > 0,
            "Should have valid test line"
        );
    }

    // Check specific Breaking Change IDs
    let breaking_change_ids: Vec<&str> = odbc_breaking_changes
        .iter()
        .map(|b| b.behavior_difference_id.as_str())
        .collect();
    assert!(breaking_change_ids.contains(&"BD#8"), "Should find BD#8");
    assert!(breaking_change_ids.contains(&"BD#9"), "Should find BD#9");
    assert!(breaking_change_ids.contains(&"BD#10"), "Should find BD#10");

    Ok(())
}

// ===== Helper Structs and Test Data =====

/// Helper to create a temporary workspace with features and test files
struct TestWorkspace {
    _temp_dir: TempDir,
    workspace_root: PathBuf,
    features_dir: PathBuf,
}

impl TestWorkspace {
    fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let workspace_root = temp_dir.path().to_path_buf();
        let features_dir = workspace_root.join("tests/definitions");

        // Create directory structure
        fs::create_dir_all(&features_dir)?;
        fs::create_dir_all(workspace_root.join("sf_core/tests/e2e/auth"))?;
        fs::create_dir_all(workspace_root.join("sf_core/tests/e2e/query"))?;
        fs::create_dir_all(workspace_root.join("jdbc/src/test/java/e2e/auth"))?;
        fs::create_dir_all(workspace_root.join("jdbc/src/test/java/e2e/query"))?;
        fs::create_dir_all(workspace_root.join("odbc_tests/tests/e2e/auth"))?;
        fs::create_dir_all(workspace_root.join("odbc_tests/tests/e2e/query"))?;

        // Create BehaviorDifferences.yaml file for tests
        let breaking_change_file = workspace_root.join("odbc_tests/BehaviorDifferences.yaml");
        fs::write(
            breaking_change_file,
            Self::create_test_breaking_change_descriptions(),
        )?;

        Ok(Self {
            _temp_dir: temp_dir,
            workspace_root,
            features_dir,
        })
    }

    fn create_feature_file(&self, subdir: &str, name: &str, content: &str) -> Result<()> {
        let feature_dir = self.features_dir.join(subdir);
        fs::create_dir_all(&feature_dir)?;
        let feature_path = feature_dir.join(format!("{}.feature", name));
        fs::write(feature_path, content)?;
        Ok(())
    }

    fn create_rust_test(&self, subdir: &str, name: &str, content: &str) -> Result<()> {
        let test_path = self
            .workspace_root
            .join("sf_core/tests/e2e")
            .join(subdir)
            .join(format!("{}.rs", name));
        fs::write(test_path, content)?;
        Ok(())
    }

    fn create_java_test(&self, subdir: &str, name: &str, content: &str) -> Result<()> {
        let test_path = self
            .workspace_root
            .join("jdbc/src/test/java/com/snowflake/jdbc/integration")
            .join(subdir)
            .join(format!("{}Test.java", name));
        fs::create_dir_all(test_path.parent().unwrap())?;
        fs::write(test_path, content)?;
        Ok(())
    }

    fn create_cpp_test(&self, subdir: &str, name: &str, content: &str) -> Result<()> {
        let test_path = self
            .workspace_root
            .join("odbc_tests/tests/e2e")
            .join(subdir)
            .join(format!("{}.cpp", name));
        fs::write(test_path, content)?;
        Ok(())
    }

    fn create_cpp_common_file(&self, name: &str, content: &str) -> Result<()> {
        let common_dir = self.workspace_root.join("odbc_tests/common/src");
        fs::create_dir_all(&common_dir)?;
        let file_path = common_dir.join(format!("{}.cpp", name));
        fs::write(file_path, content)?;

        // Also create the header file
        let header_dir = self.workspace_root.join("odbc_tests/common/include");
        fs::create_dir_all(&header_dir)?;
        let header_path = header_dir.join(format!("{}.hpp", name));
        let header_content = format!(
            "#pragma once\n\nvoid validate_breaking_change_successful_authentication();\nvoid validate_nested_authentication();\n"
        );
        fs::write(header_path, header_content)?;
        Ok(())
    }

    fn get_validator(&self) -> Result<GherkinValidator> {
        GherkinValidator::new(self.workspace_root.clone(), self.features_dir.clone())
    }

    fn create_test_breaking_change_descriptions() -> &'static str {
        r#"behavior_differences:
  1:
    name: "Simple authentication Breaking Change for testing basic Breaking Change detection"
  
  2:
    name: "Cross-file Breaking Change for testing helper method detection"
  
  3:
    name: "Additional cross-file Breaking Change for comprehensive testing"
  
  4:
    name: "Nested helper method Breaking Change for testing deep call chains"
  
  5:
    name: "Mixed scenario Breaking Change for testing annotation filtering"
  
  6:
    name: "Non-Breaking Change scenario test (should not be found)"
  
  7:
    name: "Line number accuracy Breaking Change for testing precise location tracking"
  
  8:
    name: "First multiple Breaking Change for testing multiple Breaking Changes in single method"
  
  9:
    name: "Second multiple Breaking Change for testing multiple Breaking Changes in single method"
  
  10:
    name: "Third multiple Breaking Change for testing multiple Breaking Changes in single method"
"#
    }
}

/// Test implementations for different scenarios
struct TestImplementations;

impl TestImplementations {
    fn create_complete_login_feature() -> &'static str {
        r#"@core @jdbc @odbc
Feature: User Login

  @core_e2e @jdbc_int @odbc_e2e
  Scenario: Successful login with valid credentials
    Given I have valid credentials
    When I attempt to login
    Then login should succeed
    And I should have access to the system
"#
    }

    fn create_complete_rust_login_test() -> &'static str {
        r#"
#[test]
fn successful_login_with_valid_credentials() {
    // Given I have valid credentials
    let credentials = setup_valid_credentials();

    // When I attempt to login
    let result = attempt_login(&credentials);

    // Then login should succeed
    assert!(result.is_ok());

    // And I should have access to the system
    assert!(has_system_access(&result.unwrap()));
}
"#
    }

    fn create_complete_jdbc_login_test() -> &'static str {
        r#"
import org.junit.Test;
import static org.junit.Assert.*;

public class LoginTest {
    @Test
    public void successfulLoginWithValidCredentials() throws Exception {
        // Given I have valid credentials
        Credentials credentials = setupValidCredentials();

        // When I attempt to login
        LoginResult result = attemptLogin(credentials);

        // Then login should succeed
        assertTrue("Login should succeed", result.isSuccess());

        // And I should have access to the system
        assertTrue("Should have system access", result.hasSystemAccess());
    }
}
"#
    }

    fn create_complete_odbc_login_test() -> &'static str {
        r#"
#include <catch2/catch.hpp>

TEST_CASE("Successful login with valid credentials") {
    // Given I have valid credentials
    auto credentials = setup_valid_credentials();

    // When I attempt to login
    auto result = attempt_login(credentials);

    // Then login should succeed
    REQUIRE(result.is_success());

    // And I should have access to the system
    REQUIRE(has_system_access(result));
}
"#
    }

    fn create_missing_file_feature() -> &'static str {
        r#"@core @jdbc
Feature: Missing File Test

  @core_e2e @jdbc_int
  Scenario: Test missing file scenario
    Given I have test data
    When I perform an action
    Then I should get expected result
"#
    }

    fn create_missing_function_feature() -> &'static str {
        r#"@core @jdbc
Feature: Missing Function Test

  @core_e2e @jdbc_int  
  Scenario: Test missing function scenario
    Given I have test setup
    When I execute the function
    Then I should see the result
"#
    }

    fn create_rust_test_with_wrong_function_name() -> &'static str {
        r#"
#[test]
fn wrong_function_name() {
    // Given I have test setup
    let setup = create_test_setup();

    // When I execute the function
    let result = execute_function(&setup);

    // Then I should see the result
    assert!(result.is_ok());
}
"#
    }

    fn create_jdbc_test_with_wrong_function_name() -> &'static str {
        r#"
import org.junit.Test;
import static org.junit.Assert.*;

public class MissingFunctionTest {
    @Test
    public void wrongFunctionName() throws Exception {
        // Given I have test setup
        TestSetup setup = createTestSetup();

        // When I execute the function
        Result result = executeFunction(setup);

        // Then I should see the result
        assertTrue("Should see result", result.isSuccess());
    }
}
"#
    }

    fn create_missing_step_feature() -> &'static str {
        r#"@core @jdbc
Feature: Missing Step Test

  @core_e2e @jdbc_int
  Scenario: Test missing step scenario
    Given I have valid credentials
    When I attempt to login
    Then login should succeed
    And user session should be created
"#
    }

    fn create_rust_test_with_missing_step() -> &'static str {
        r#"
#[test]
fn test_missing_step_scenario() {
    // Given I have valid credentials
    let credentials = setup_valid_credentials();

    // When I attempt to login
    let result = attempt_login(&credentials);

    // Then login should succeed
    assert!(result.is_ok());
    
    // Missing: And user session should be created
}
"#
    }

    fn create_jdbc_test_with_missing_step() -> &'static str {
        r#"
import org.junit.Test;
import static org.junit.Assert.*;

public class MissingStepTest {
    @Test
    public void testMissingStepScenario() throws Exception {
        // Given I have valid credentials
        Credentials credentials = setupValidCredentials();

        // When I attempt to login
        LoginResult result = attemptLogin(credentials);

        // Then login should succeed
        assertTrue("Login should succeed", result.isSuccess());
        
        // Missing: And user session should be created
    }
}
"#
    }

    fn create_mixed_scenarios_feature() -> &'static str {
        r#"@core @jdbc
Feature: Mixed Scenarios Test

  @core_e2e @jdbc_int
  Scenario: Complete scenario
    Given I have order data
    When I fetch orders
    Then I should get order list

  @core_e2e @jdbc_int
  Scenario: Incomplete scenario
    Given I have order data
    When I fetch orders
    Then I should get order list
    And orders should be sorted by date
"#
    }

    fn create_rust_mixed_scenarios_test() -> &'static str {
        r#"
#[test]
fn complete_scenario() {
    // Given I have order data
    let order_data = setup_order_data();

    // When I fetch orders
    let orders = fetch_orders(&order_data);

    // Then I should get order list
    assert!(!orders.is_empty());
}

#[test]
fn incomplete_scenario() {
    // Given I have order data
    let order_data = setup_order_data();

    // When I fetch orders
    let orders = fetch_orders(&order_data);

    // Then I should get order list
    assert!(!orders.is_empty());
    
    // Missing: And orders should be sorted by date
}
"#
    }

    fn create_jdbc_mixed_scenarios_test() -> &'static str {
        r#"
import org.junit.Test;
import static org.junit.Assert.*;

public class MixedTest {
    @Test
    public void completeScenario() throws Exception {
        // Given I have order data
        OrderData orderData = setupOrderData();

        // When I fetch orders
        List<Order> orders = fetchOrders(orderData);

        // Then I should get order list
        assertFalse("Should get order list", orders.isEmpty());
    }

    @Test
    public void incompleteScenario() throws Exception {
        // Given I have order data
        OrderData orderData = setupOrderData();

        // When I fetch orders
        List<Order> orders = fetchOrders(orderData);

        // Then I should get order list
        assertFalse("Should get order list", orders.isEmpty());
        
        // Missing: And orders should be sorted by date
    }
}
"#
    }

    fn create_rust_test_with_orphaned_method() -> &'static str {
        r#"
#[test]
fn successful_login_with_valid_credentials() {
    // Given I have valid credentials
    let credentials = setup_valid_credentials();

    // When I attempt to login
    let result = attempt_login(&credentials);

    // Then login should succeed
    assert!(result.is_ok());

    // And I should have access to the system
    assert!(has_system_access(&result.unwrap()));
}

#[test]
fn orphaned_test_method() {
    // This method doesn't correspond to any scenario
    let data = setup_test_data();
    assert!(data.is_valid());
}
"#
    }

    fn create_orphaned_rust_test() -> &'static str {
        r#"
#[test]
fn orphaned_test_function() {
    // This entire file doesn't match any feature
    let result = perform_orphaned_test();
    assert!(result.is_ok());
}
"#
    }

    fn create_nested_blocks_feature() -> &'static str {
        r#"@core @jdbc
Feature: Nested Blocks Test

  @core_e2e @jdbc_int @odbc_e2e
  Scenario: Test with nested control structures
    Given I have test data
    When I process data with nested logic
    Then results should be correct
    And cleanup should be completed
"#
    }

    fn create_java_test_with_nested_blocks() -> &'static str {
        r#"
import org.junit.Test;
import static org.junit.Assert.*;

public class NestedBlocksTest {
    @Test
    public void testWithNestedControlStructures() throws Exception {
        // Given I have test data
        TestData data = setupTestData();

        // When I process data with nested logic
        if (data.isValid()) {
            for (int i = 0; i < data.getCount(); i++) {
                if (data.getItem(i) != null) {
                    processItem(data.getItem(i));
                }
            }
        }
        Result result = getProcessingResult();

        // Then results should be correct
        assertTrue("Results should be correct", result.isSuccess());

        // And cleanup should be completed
        try {
            cleanup();
        } finally {
            verifyCleanup();
        }
    }
}
"#
    }

    fn create_rust_test_with_nested_blocks() -> &'static str {
        r#"
#[test]
fn test_with_nested_control_structures() {
    // Given I have test data
    let data = setup_test_data();

    // When I process data with nested logic
    let result = if data.is_valid() {
        let mut processed = Vec::new();
        for item in data.items() {
            if let Some(valid_item) = item.validate() {
                processed.push(process_item(valid_item));
            }
        }
        ProcessingResult::new(processed)
    } else {
        ProcessingResult::empty()
    };

    // Then results should be correct
    assert!(result.is_success());

    // And cleanup should be completed
    match cleanup() {
        Ok(_) => verify_cleanup(),
        Err(e) => panic!("Cleanup failed: {e}"),
    }
}
"#
    }

    fn create_cpp_test_with_nested_blocks() -> &'static str {
        r#"
#include <catch2/catch.hpp>

TEST_CASE("Test with nested control structures") {
    // Given I have test data
    auto data = setup_test_data();

    // When I process data with nested logic
    if (data.is_valid()) {
        for (int i = 0; i < data.get_count(); i++) {
            if (data.get_item(i) != nullptr) {
                process_item(data.get_item(i));
            }
        }
    }
    auto result = get_processing_result();

    // Then results should be correct
    REQUIRE(result.is_success());

    // And cleanup should be completed
    try {
        cleanup();
    } catch (...) {
        FAIL("Cleanup should not throw");
    }
    verify_cleanup();
}
"#
    }

    fn create_string_braces_feature() -> &'static str {
        r#"@core @jdbc
Feature: String Braces Test

  @core_e2e @jdbc_int
  Scenario: Test with braces in strings
    Given I have JSON data with braces
    When I process strings containing braces
    Then parsing should ignore string braces
"#
    }

    fn create_java_test_with_string_braces() -> &'static str {
        r#"
import org.junit.Test;
import static org.junit.Assert.*;

public class StringBracesTest {
    @Test
    public void testWithBracesInStrings() throws Exception {
        // Given I have JSON data with braces
        String json = "{\"key\": \"value\", \"nested\": {\"inner\": \"data\"}}";
        String template = "Expected format: { data: [...] }";
        
        // When I process strings containing braces
        String query = "SELECT * FROM table WHERE json_data = '" + json + "'";
        String message = "Failed to process: { error: 'invalid format' }";
        Result result = processStrings(json, template, query, message);

        // Then parsing should ignore string braces
        assertTrue("Should parse correctly", result.isSuccess());
    }
}
"#
    }

    fn create_rust_test_with_string_braces() -> &'static str {
        r##"
#[test]
fn test_with_braces_in_strings() {
    // Given I have JSON data with braces
    let json = r#"{"key": "value", "nested": {"inner": "data"}}"#;
    let template = "Expected format: { data: [...] }";
    
    // When I process strings containing braces
    let query = format!("SELECT * FROM table WHERE json_data = '{}'", json);
    let message = "Failed to process: { error: 'invalid format' }";
    let result = process_strings(&json, &template, &query, &message);

    // Then parsing should ignore string braces
    assert!(result.is_success());
}
"##
    }

    // ===== Breaking Change Test Data =====

    fn create_simple_breaking_change_feature() -> &'static str {
        r#"@core @jdbc
Feature: Simple Breaking Change Test

  @odbc
  Scenario: should authenticate using private key
    Given I have a private key file
    When I attempt to authenticate
    Then authentication should succeed
"#
    }

    fn create_cpp_test_with_simple_breaking_change() -> &'static str {
        r#"#include <catch2/catch.hpp>

TEST_CASE("should authenticate using private key") {
    // Given I have a private key file
    auto private_key = setup_private_key();

    // When I attempt to authenticate
    NEW_DRIVER_ONLY("BD#1") {
        auto result = authenticate_with_new_method(private_key);
        REQUIRE(result.is_success());
    }

    OLD_DRIVER_ONLY("BD#1") {
        auto result = authenticate_with_old_method(private_key);
        REQUIRE(result.is_success());
    }

    // Then authentication should succeed
    REQUIRE(authentication_successful());
}
"#
    }

    fn create_cross_file_breaking_change_feature() -> &'static str {
        r#"@core @jdbc
Feature: Cross File Breaking Change Test

  @odbc
  Scenario: should authenticate using private key with helper
    Given I have authentication setup
    When I call helper methods
    Then authentication should be validated
"#
    }

    fn create_cpp_test_with_cross_file_breaking_change() -> &'static str {
        r#"#include <catch2/catch.hpp>
#include "../../../common/include/auth_helpers.hpp"

TEST_CASE("should authenticate using private key with helper") {
    // Given I have authentication setup
    auto auth_setup = create_auth_setup();

    // When I call helper methods
    validate_breaking_change_successful_authentication();

    // Then authentication should be validated
    REQUIRE(auth_setup.is_validated());
}
"#
    }

    fn create_cpp_common_helper_with_breaking_change() -> &'static str {
        r#"#include "auth_helpers.hpp"

void validate_breaking_change_successful_authentication() {
    // BD#2: Successful authentication validation
    OLD_DRIVER_ONLY("BD#2") {
        INFO("Successfully authenticated with private key - old driver behavior");
        // Add any old driver specific validations here
    }

    NEW_DRIVER_ONLY("BD#2") {
        INFO("Successfully authenticated with private key - new driver behavior");
        // Add any new driver specific validations here
    }

    // BD#3: Additional successful authentication validation
    NEW_DRIVER_ONLY("BD#3") {
        INFO("Additional validation for successful authentication");
        // Add any additional validations here
    }
}

void validate_nested_authentication() {
    deep_nested_auth_helper();
}

void deep_nested_auth_helper() {
    NEW_DRIVER_ONLY("BD#4") {
        INFO("Deep nested authentication check");
    }
}
"#
    }

    fn create_nested_breaking_change_feature() -> &'static str {
        r#"@core @jdbc
Feature: Nested Breaking Change Test

  @odbc
  Scenario: should handle nested authentication
    Given I have nested authentication setup
    When I call nested helper methods
    Then deep authentication should work
"#
    }

    fn create_cpp_test_with_nested_breaking_change() -> &'static str {
        r#"#include <catch2/catch.hpp>

TEST_CASE("should handle nested authentication") {
    // Given I have nested authentication setup
    auto setup = create_nested_setup();

    // When I call nested helper methods
    first_level_helper();

    // Then deep authentication should work
    REQUIRE(setup.is_deeply_authenticated());
}

void first_level_helper() {
    second_level_helper();
}

void second_level_helper() {
    third_level_helper();
}

void third_level_helper() {
    NEW_DRIVER_ONLY("BD#4") {
        INFO("Nested Breaking Change validation in third level");
    }
}
"#
    }

    fn create_mixed_breaking_change_feature() -> &'static str {
        r#"@core @jdbc
Feature: Mixed Breaking Change Test

  @odbc
  Scenario: should authenticate with breaking_change annotation
    Given I have authentication data
    When I authenticate
    Then it should succeed with Breaking Change

  @odbc
  Scenario: should authenticate without breaking_change annotation
    Given I have authentication data
    When I authenticate
    Then it should succeed normally
"#
    }

    fn create_cpp_test_with_mixed_breaking_change() -> &'static str {
        r#"#include <catch2/catch.hpp>

TEST_CASE("should authenticate with breaking_change annotation") {
    // Given I have authentication data
    auto auth_data = setup_auth_data();

    // When I authenticate
    NEW_DRIVER_ONLY("BD#5") {
        auto result = authenticate_new_way(auth_data);
        REQUIRE(result.is_success());
    }

    // Then it should succeed with Breaking Change
    REQUIRE(authentication_with_breaking_change_succeeded());
}

TEST_CASE("should authenticate without breaking_change annotation") {
    // Given I have authentication data
    auto auth_data = setup_auth_data();

    // When I authenticate
    NEW_DRIVER_ONLY("BD#6") {
        // This Breaking Change should now be found since scenario has @odbc
        auto result = authenticate_normally(auth_data);
        REQUIRE(result.is_success());
    }

    // Then it should succeed normally
    REQUIRE(normal_authentication_succeeded());
}
"#
    }

    fn create_line_numbers_breaking_change_feature() -> &'static str {
        r#"@core @jdbc
Feature: Line Numbers Breaking Change Test

  @odbc
  Scenario: should test specific line numbers
    Given I have line number test setup
    When I check specific lines
    Then line numbers should be accurate
"#
    }

    fn create_cpp_test_with_specific_line_numbers() -> &'static str {
        r#"#include <catch2/catch.hpp>

TEST_CASE("should test specific line numbers") {
    // Given I have line number test setup
    auto setup = create_line_test_setup();
    
    NEW_DRIVER_ONLY("BD#7") {
        INFO("This NEW_DRIVER_ONLY should be at line 8");
        REQUIRE(setup.is_valid());
    }
    
    OLD_DRIVER_ONLY("BD#7") {
        INFO("This OLD_DRIVER_ONLY should be at line 13");
        REQUIRE(setup.is_valid_old_way());
    }
    
    // Then line numbers should be accurate
    REQUIRE(line_numbers_are_correct());
}
"#
    }

    fn create_multiple_breaking_changes_feature() -> &'static str {
        r#"@core @jdbc
Feature: Multiple Breaking Changes Test

  @odbc
  Scenario: should handle multiple authentication methods
    Given I have multiple authentication options
    When I test each method
    Then all methods should work with their respective Breaking Changes
"#
    }

    fn create_cpp_test_with_multiple_breaking_changes() -> &'static str {
        r#"#include <catch2/catch.hpp>

TEST_CASE("should handle multiple authentication methods") {
    // Given I have multiple authentication options
    auto auth_options = setup_multiple_auth_options();

    // When I test each method
    NEW_DRIVER_ONLY("BD#8") {
        auto result1 = test_first_method(auth_options);
        REQUIRE(result1.is_success());
    }

    NEW_DRIVER_ONLY("BD#9") {
        auto result2 = test_second_method(auth_options);
        REQUIRE(result2.is_success());
    }

    OLD_DRIVER_ONLY("BD#10") {
        auto result3 = test_third_method_old_way(auth_options);
        REQUIRE(result3.is_success());
    }

    // Then all methods should work with their respective Breaking Changes
    REQUIRE(all_authentication_methods_work());
}
"#
    }
}
