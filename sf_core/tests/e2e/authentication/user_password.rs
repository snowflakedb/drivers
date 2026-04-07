use crate::common::snowflake_test_client::SnowflakeTestClient;

// Some test accounts (notably GCP) enforce MFA at the account level, which
// causes plain username+password login to fail with "Multi-factor authentication
// is required for this account." This is a server-side policy, not a driver bug.
// We detect this and skip instead of failing.

fn is_mfa_enforced(err: &str) -> bool {
    err.contains("Multi-factor authentication is required")
}

#[test]
fn should_authenticate_using_username_and_password() {
    //Given Authentication is set to default (snowflake) with valid username and password
    let client = SnowflakeTestClient::with_default_params();
    let Some(password) = client.parameters.password.clone() else {
        eprintln!("SKIPPED: no password configured (environment uses JWT-only auth)");
        return;
    };
    client.set_connection_option("password", &password);

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    match result {
        Ok(()) => client.verify_simple_query(Ok(())),
        Err(ref e) if is_mfa_enforced(e) => {
            eprintln!("SKIPPED: account has MFA enforcement enabled");
        }
        Err(e) => panic!("Expected login to succeed, got: {e}"),
    }
}

#[test]
fn should_authenticate_using_explicit_snowflake_authenticator() {
    //Given Authentication is explicitly set to snowflake with valid username and password
    let client = SnowflakeTestClient::with_default_params();
    let Some(password) = client.parameters.password.clone() else {
        eprintln!("SKIPPED: no password configured (environment uses JWT-only auth)");
        return;
    };
    client.set_connection_option("password", &password);
    client.set_connection_option("authenticator", "snowflake");

    //When Trying to Connect
    let result = client.connect();

    //Then Login is successful and simple query can be executed
    match result {
        Ok(()) => client.verify_simple_query(Ok(())),
        Err(ref e) if is_mfa_enforced(e) => {
            eprintln!("SKIPPED: account has MFA enforcement enabled");
        }
        Err(e) => panic!("Expected login to succeed, got: {e}"),
    }
}

#[test]
fn should_fail_authentication_when_wrong_password_is_provided() {
    //Given Authentication is set to default with valid username and wrong password
    let client = SnowflakeTestClient::with_default_params();
    client.set_connection_option("password", "definitely_not_a_valid_password_12345");

    //When Trying to Connect
    let result = client.connect();

    //Then There is error returned
    client.assert_login_error(result);
}
