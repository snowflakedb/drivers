use crate::common::mocks;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use serde_json::json;
use sf_core::protobuf::generated::database_driver_v1::execute_query_response;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const DELAY: Duration = Duration::from_millis(400);

fn take<T>(handle: std::thread::ScopedJoinHandle<'_, T>) -> T {
    handle
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

struct DelayedQuery {
    max: Arc<AtomicUsize>,
    starts: Mutex<Vec<Instant>>,
}

impl Respond for DelayedQuery {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let now = Instant::now();
        let mut starts = self.starts.lock().unwrap();
        starts.retain(|start| now.duration_since(*start) < DELAY);
        starts.push(now);
        self.max.fetch_max(starts.len(), Ordering::Relaxed);
        ResponseTemplate::new(200)
            .set_delay(DELAY)
            .set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "single-id",
                    "queryResultFormat": "json",
                    "rowset": [],
                    "rowtype": [],
                }
            }))
    }
}

async fn mount_delayed_query(server: &MockServer) -> Arc<AtomicUsize> {
    let max = Arc::new(AtomicUsize::new(0));
    Mock::given(method("POST"))
        .and(path_regex(r"/queries/v1/query-request.*"))
        .respond_with(DelayedQuery {
            max: Arc::clone(&max),
            starts: Mutex::new(Vec::new()),
        })
        .expect(2)
        .mount(server)
        .await;
    max
}

#[tokio::test(flavor = "multi_thread")]
async fn should_overlap_execute_on_distinct_statements_of_one_connection() {
    let server = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&server).await;
    let inflight = mount_delayed_query(&server).await;
    let uri = server.uri();

    tokio::task::spawn_blocking(move || {
        // Given Snowflake client is logged in
        let client = SnowflakeTestClient::connect_integration_test(Some(&uri));
        let first = client.new_statement();
        let second = client.new_statement();
        client.set_sql_query(&first, "SELECT 1");
        client.set_sql_query(&second, "SELECT 2");

        // When two statements execute concurrently on the same connection
        let (result_a, result_b) = std::thread::scope(|scope| {
            let a = scope.spawn(|| client.execute_statement_query(&first));
            let b = scope.spawn(|| client.execute_statement_query(&second));
            (take(a), take(b))
        });

        // Then both executions succeed
        assert!(matches!(
            result_a,
            execute_query_response::Result::Single(_)
        ));
        assert!(matches!(
            result_b,
            execute_query_response::Result::Single(_)
        ));

        // And the calls overlap
        assert!(
            inflight.load(Ordering::Relaxed) >= 2,
            "distinct statements overlap: max in-flight was {}",
            inflight.load(Ordering::Relaxed)
        );
    })
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn should_serialize_concurrent_execute_calls_on_the_same_statement() {
    let server = MockServer::start().await;
    mocks::auth::mount_jwt_login_success(&server).await;
    let inflight = mount_delayed_query(&server).await;
    let uri = server.uri();

    tokio::task::spawn_blocking(move || {
        // Given Snowflake client is logged in
        let client = SnowflakeTestClient::connect_integration_test(Some(&uri));
        let stmt = client.new_statement();
        client.set_sql_query(&stmt, "SELECT 1");

        // When the same statement is executed from 2 threads
        let (result_a, result_b) = std::thread::scope(|scope| {
            let a = scope.spawn(|| client.execute_statement_query(&stmt));
            let b = scope.spawn(|| client.execute_statement_query(&stmt));
            (take(a), take(b))
        });

        // Then both executions succeed
        assert!(matches!(
            result_a,
            execute_query_response::Result::Single(_)
        ));
        assert!(matches!(
            result_b,
            execute_query_response::Result::Single(_)
        ));

        // And the calls run one after another
        assert_eq!(
            inflight.load(Ordering::Relaxed),
            1,
            "same-statement executes serialize: max in-flight was {}",
            inflight.load(Ordering::Relaxed)
        );
    })
    .await
    .unwrap();
}
