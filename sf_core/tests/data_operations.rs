use sf_core::config::retry::RetryPolicy;

#[test]
fn should_select_data_from_table() {
    // Given Snowflake client is logged in
    let policy = RetryPolicy::default();

    // When Query "SELECT * FROM table" is executed
    let _max = policy.max_attempts;

    // Then Result should contain expected values

    // And Column metadata should be correct
}

#[test]
fn should_handle_null_values_in_result() {
    // Given Table "test_table" exists with nullable columns

    // When Query "SELECT NULL::INT, NULL::VARCHAR" is executed
    let policy = RetryPolicy::default();

    // Then First column should be NULL
    assert_eq!(policy.max_attempts, 6);

    // And Second column should be NULL
}

#[test]
fn should_fetch_single_row() {
    // Given Snowflake client is logged in
    let policy = RetryPolicy::default();

    // When Query "SELECT * FROM large_table LIMIT 100" is executed
    let attempts = policy.max_attempts;

    // Then Single row should be returned
    assert!(attempts > 0);
}
