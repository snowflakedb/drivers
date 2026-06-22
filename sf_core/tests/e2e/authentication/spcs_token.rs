use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::snowflake_test_client::SnowflakeTestClient;

// SPCS auth E2E. This test does NOT authenticate via SPCS itself — it runs on
// the CI host with a normal connection and orchestrates the real SPCS flow:
// it launches a one-shot job service whose container (the `spcs_probe` binary,
// built into the image at $SPCS_PROBE_IMAGE) authenticates inside SPCS using the
// platform-injected OAuth token (no user, made optional by SNOW-3647715), with
// the driver attaching the SPCS_TOKEN service identifier automatically. The
// container then runs a query. `EXECUTE JOB SERVICE` is synchronous and fails if
// the container exits non-zero, so a successful statement proves the in-SPCS
// login worked end-to-end.
//
// Requires a published probe image + the compute pool / image repository from
// ci/account_setup.sql. Gated behind the `auth_spcs_e2e` feature and #[ignore];
// run from the dedicated CI workflow (or locally) with:
//   PARAMETER_PATH=parameters.json \
//   SPCS_PROBE_IMAGE=/testing_setup/public/ud_test_image_repo/spcs_probe:1 \
//     cargo test -p sf_core --test e2e_tests --features auth_spcs_e2e -- --ignored spcs_

const COMPUTE_POOL: &str = "ud_test_spcs_pool";
const DEFAULT_PROBE_IMAGE: &str = "/testing_setup/public/ud_test_image_repo/spcs_probe:1";

#[test]
#[ignore = "Requires a published SPCS probe image + compute pool (run from the SPCS CI workflow)"]
fn should_authenticate_inside_spcs_using_the_injected_session_token() {
    // Given a probe image is published to the SPCS image repository
    let image =
        std::env::var("SPCS_PROBE_IMAGE").unwrap_or_else(|_| DEFAULT_PROBE_IMAGE.to_string());
    let client = SnowflakeTestClient::connect_with_default_auth();
    let job_name = unique_job_name();
    let sql = execute_job_service_sql(&job_name, &image);

    // When the probe runs as a job service in the compute pool
    let result = client.execute_query_no_unwrap(&sql);

    // Then it authenticates with the injected session token and the query succeeds
    result.unwrap_or_else(|e| {
        panic!(
            "EXECUTE JOB SERVICE failed — the in-SPCS probe did not authenticate \
             with the injected token (job did not reach DONE): {e}"
        )
    });
}

fn unique_job_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before epoch")
        .as_nanos();
    format!("ud_spcs_probe_job_{nanos}")
}

/// Builds a synchronous `EXECUTE JOB SERVICE` statement that runs the probe
/// container once. SPCS injects everything the probe needs — the account/host/
/// database/schema, the session token at /snowflake/session/spcs_token, and the
/// `SNOWFLAKE_RUNNING_INSIDE_SPCS` marker (a reserved env var the platform sets;
/// the spec must NOT set it).
fn execute_job_service_sql(job_name: &str, image: &str) -> String {
    format!(
        "EXECUTE JOB SERVICE\n  \
         IN COMPUTE POOL {COMPUTE_POOL}\n  \
         NAME = {job_name}\n  \
         FROM SPECIFICATION $$\n\
         spec:\n  \
         containers:\n    \
         - name: probe\n      \
         image: {image}\n\
         $$"
    )
}
