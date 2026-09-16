use crate::common::arrow_result_helper::ArrowResultHelper;
use crate::common::file_utils::{create_test_file, path_to_sql_uri};
use crate::common::put_get_common::{ArrowDeserialize, GetResult, PutResult};
use crate::common::snowflake_test_client::SnowflakeTestClient;
use uuid::Uuid;

fn random_stage_name() -> String {
    format!("TEST_PUT_CWD_{}", Uuid::new_v4().simple())
}

fn execute_with_cwd<T: ArrowDeserialize>(client: &SnowflakeTestClient, sql: &str, cwd: &str) -> T {
    let result = client.execute_query_with_statement_params(sql, &[("cwd", cwd)]);
    let mut helper = ArrowResultHelper::from_result(result);
    helper.fetch_one().expect("query result")
}

fn put_data_csv_on_stage(client: &SnowflakeTestClient, stage_name: &str) {
    let upload_dir = tempfile::TempDir::new().unwrap();
    let file = create_test_file(upload_dir.path(), "data.csv", "a,b,c\n");
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} AUTO_COMPRESS=FALSE",
        path_to_sql_uri(&file)
    );
    client.execute_query(&put_sql);
}

#[test]
fn should_ignore_cwd_when_the_put_source_path_is_absolute() {
    // Given A source file exists at an absolute path
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name();
    client.create_temporary_stage(&stage_name);
    let file_dir = tempfile::TempDir::new().unwrap();
    let abs_file = create_test_file(file_dir.path(), "abs.csv", "a,b,c\n");
    // And cwd points at a different empty directory
    let other_dir = tempfile::TempDir::new().unwrap();

    // When PUT is executed with that absolute file URI and cwd
    let put_sql = format!(
        "PUT 'file://{}' @{stage_name} AUTO_COMPRESS=FALSE",
        path_to_sql_uri(&abs_file)
    );
    let put: PutResult = execute_with_cwd(
        &client,
        &put_sql,
        other_dir.path().to_str().expect("utf-8 cwd"),
    );

    // Then The file is uploaded from the absolute path
    assert_eq!(put.status, "UPLOADED");
    assert_eq!(put.target, "abs.csv");
}

#[test]
fn should_resolve_a_relative_put_source_path_against_a_relative_cwd() {
    // Given A source file exists under a directory relative to the process working directory
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name();
    client.create_temporary_stage(&stage_name);
    let process_cwd = std::env::current_dir().expect("process cwd");
    let work_dir = tempfile::TempDir::new_in(&process_cwd).expect("temp dir under process cwd");
    create_test_file(work_dir.path(), "data.csv", "a,b,c\n");
    let relative_cwd = work_dir
        .path()
        .strip_prefix(&process_cwd)
        .expect("temp dir is under process cwd")
        .to_str()
        .expect("utf-8 relative cwd");

    // When PUT is executed with a relative file URI and that relative cwd
    let put_sql = format!("PUT 'file://./data.csv' @{stage_name} AUTO_COMPRESS=FALSE");
    let put: PutResult = execute_with_cwd(&client, &put_sql, relative_cwd);

    // Then The file is uploaded from the joined path
    assert_eq!(put.status, "UPLOADED");
    assert_eq!(put.target, "data.csv");
}

#[test]
fn should_resolve_a_relative_put_source_path_against_an_absolute_cwd() {
    // Given A source file exists in a temporary directory
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name();
    client.create_temporary_stage(&stage_name);
    let work_dir = tempfile::TempDir::new().unwrap();
    create_test_file(work_dir.path(), "data.csv", "a,b,c\n");

    // When PUT is executed with a relative file URI and that directory as cwd
    let put_sql = format!("PUT 'file://./data.csv' @{stage_name} AUTO_COMPRESS=FALSE");
    let put: PutResult = execute_with_cwd(
        &client,
        &put_sql,
        work_dir.path().to_str().expect("utf-8 cwd"),
    );

    // Then The file is uploaded from the joined path
    assert_eq!(put.status, "UPLOADED");
    assert_eq!(put.target, "data.csv");
}

#[test]
fn should_ignore_cwd_when_the_get_destination_path_is_absolute() {
    // Given A file exists on a stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name();
    client.create_temporary_stage(&stage_name);
    put_data_csv_on_stage(&client, &stage_name);
    // And cwd points at a different empty directory
    let other_dir = tempfile::TempDir::new().unwrap();

    // When GET is executed with an absolute destination URI and cwd
    let dest_dir = tempfile::TempDir::new().unwrap();
    let get_sql = format!(
        "GET @{stage_name}/data.csv 'file://{}/'",
        path_to_sql_uri(dest_dir.path())
    );
    let get: GetResult = execute_with_cwd(
        &client,
        &get_sql,
        other_dir.path().to_str().expect("utf-8 cwd"),
    );

    // Then The file is downloaded to the absolute path
    assert_eq!(get.status, "DOWNLOADED");
    assert_eq!(get.file, "data.csv");
    assert!(dest_dir.path().join("data.csv").exists());
    assert!(!other_dir.path().join("data.csv").exists());
}

#[test]
fn should_resolve_a_relative_get_destination_against_a_relative_cwd() {
    // Given A file exists on a stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name();
    client.create_temporary_stage(&stage_name);
    put_data_csv_on_stage(&client, &stage_name);
    // And a destination directory exists relative to the process working directory
    let process_cwd = std::env::current_dir().expect("process cwd");
    let dest_dir = tempfile::TempDir::new_in(&process_cwd).expect("temp dir under process cwd");
    let relative_cwd = dest_dir
        .path()
        .strip_prefix(&process_cwd)
        .expect("temp dir is under process cwd")
        .to_str()
        .expect("utf-8 relative cwd");

    // When GET is executed with a relative destination URI and that relative cwd
    let get_sql = format!("GET @{stage_name}/data.csv 'file://./'");
    let get: GetResult = execute_with_cwd(&client, &get_sql, relative_cwd);

    // Then The file is downloaded to the joined path
    assert_eq!(get.status, "DOWNLOADED");
    assert_eq!(get.file, "data.csv");
    assert!(dest_dir.path().join("data.csv").exists());
}

#[test]
fn should_resolve_a_relative_get_destination_against_an_absolute_cwd() {
    // Given A file exists on a stage
    let client = SnowflakeTestClient::connect_with_default_auth();
    let stage_name = random_stage_name();
    client.create_temporary_stage(&stage_name);
    put_data_csv_on_stage(&client, &stage_name);
    // And a destination directory exists in a temporary directory
    let dest_dir = tempfile::TempDir::new().unwrap();

    // When GET is executed with a relative destination URI and that directory as cwd
    let get_sql = format!("GET @{stage_name}/data.csv 'file://./'");
    let get: GetResult = execute_with_cwd(
        &client,
        &get_sql,
        dest_dir.path().to_str().expect("utf-8 cwd"),
    );

    // Then The file is downloaded to the joined path
    assert_eq!(get.status, "DOWNLOADED");
    assert_eq!(get.file, "data.csv");
    assert!(dest_dir.path().join("data.csv").exists());
}
