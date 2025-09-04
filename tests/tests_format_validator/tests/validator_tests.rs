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
        assert!(validation.test_file_found);
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
        assert_eq!(validation.missing_steps.len(), 3); // All steps should be missing
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
        let features_dir = workspace_root.join("tests/e2e");

        // Create directory structure
        fs::create_dir_all(&features_dir)?;
        fs::create_dir_all(workspace_root.join("sf_core/tests/e2e/auth"))?;
        fs::create_dir_all(workspace_root.join("sf_core/tests/e2e/query"))?;
        fs::create_dir_all(workspace_root.join("jdbc/src/test/java/e2e/auth"))?;
        fs::create_dir_all(workspace_root.join("jdbc/src/test/java/e2e/query"))?;
        fs::create_dir_all(workspace_root.join("odbc_tests/tests/e2e/auth"))?;
        fs::create_dir_all(workspace_root.join("odbc_tests/tests/e2e/query"))?;

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
            .join("jdbc/src/test/java/e2e")
            .join(subdir)
            .join(format!("{}Test.java", name));
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

    fn get_validator(&self) -> Result<GherkinValidator> {
        GherkinValidator::new(self.workspace_root.clone(), self.features_dir.clone())
    }
}

/// Test implementations for different scenarios
struct TestImplementations;

impl TestImplementations {
    fn create_complete_login_feature() -> &'static str {
        r#"@core @jdbc @odbc
Feature: User Login

  @core @jdbc @odbc
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

  @core @jdbc
  Scenario: Test missing file scenario
    Given I have test data
    When I perform an action
    Then I should get expected result
"#
    }

    fn create_missing_function_feature() -> &'static str {
        r#"@core @jdbc
Feature: Missing Function Test

  @core @jdbc  
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

  @core @jdbc
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

  @core @jdbc
  Scenario: Complete scenario
    Given I have order data
    When I fetch orders
    Then I should get order list

  @core @jdbc
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
        r#"@core @jdbc @odbc
Feature: Nested Blocks Test

  @core @jdbc @odbc
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

  @core @jdbc
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
}
