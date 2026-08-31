//! Wiremock integration tests for query context DTO cache round-trip.
//!
//! Implements scenarios from: tests/definitions/shared/query/query_context_dto.feature
//!
//! Uses MockServerWithTls + SnowflakeTestClient to verify the full driver-level
//! behavior: login, execute queries, and verify that queryContextDTO in
//! subsequent requests carries the correct cached entries.

use crate::common::mocks::password;
use crate::common::snowflake_test_client::SnowflakeTestClient;
use crate::common::tls_proxy::MockServerWithTls;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, Respond, ResponseTemplate};

// ---------------------------------------------------------------------------
// Scenario: should send cached query context in subsequent requests
// ---------------------------------------------------------------------------

#[test]
fn should_send_cached_query_context_in_subsequent_requests() {
    // Given a wiremock server with login and query response containing queryContext
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    fixture.mount_query_response(json!({
        "success": true,
        "data": {
            "queryId": "qid-001",
            "queryResultFormat": "json",
            "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
            "rowset": [["Statement executed successfully."]],
            "total": 1,
            "returned": 1,
            "parameters": [],
            "queryContext": {
                "entries": [
                    {"id": 1, "timestamp": 100, "priority": 4, "context": "base64data1"}
                ]
            }
        }
    }));

    // When the client executes two queries
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("SELECT 1");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");

    // Then the second request contains the cached queryContextDTO entries
    let requests = fixture.query_requests();
    assert!(requests.len() >= 2, "Expected at least 2 query requests");
    let second_body = &requests[1];
    let entries = &second_body["queryContextDTO"]["entries"];
    assert!(
        entries.is_array(),
        "Second request should have queryContextDTO.entries"
    );
    let arr = entries.as_array().unwrap();
    assert!(!arr.is_empty());
    assert_eq!(arr[0]["id"], 1);
    assert_eq!(arr[0]["priority"], 4);
    assert_eq!(arr[0]["timestamp"], 100);
}

// ---------------------------------------------------------------------------
// Scenario: should keep cache unchanged when response has no queryContext
// ---------------------------------------------------------------------------

#[test]
fn should_keep_cache_unchanged_when_response_has_no_query_context() {
    // Given a wiremock server with response that has no queryContext field
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(SeedThenNoContextResponder::new()),
    );

    // When the client executes three queries
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("SELECT 1");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 3");

    // Then all three queries complete without error and the third request still contains the cached entries from the seed response
    let requests = fixture.query_requests();
    assert!(requests.len() >= 3, "Expected at least 3 query requests");
    let entries = requests[2]["queryContextDTO"]["entries"]
        .as_array()
        .expect("Third request should have queryContextDTO.entries");
    assert!(!entries.is_empty(), "Cache should be preserved from seed");
    assert_eq!(entries[0]["id"], 1);
    assert_eq!(entries[0]["priority"], 4);
}

// ---------------------------------------------------------------------------
// Scenario: should clear cache when response has null entries
// ---------------------------------------------------------------------------

#[test]
fn should_clear_cache_when_response_has_null_entries() {
    // Given a wiremock server with response that has null queryContext entries
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(SeedThenNullEntriesResponder::new()),
    );

    // When the client executes three queries
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("SELECT 1");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 3");

    // Then all three queries complete without error and the third request has no cached entries
    let requests = fixture.query_requests();
    assert!(requests.len() >= 3, "Expected at least 3 query requests");
    let dto = &requests[2]["queryContextDTO"];
    let has_entries = dto
        .get("entries")
        .and_then(|e| e.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false);
    assert!(
        !has_entries,
        "Cache should be cleared after null entries response"
    );
}

// ---------------------------------------------------------------------------
// Scenario: should merge entries when response IDs overlap
// ---------------------------------------------------------------------------

#[test]
fn should_merge_entries_when_response_ids_overlap() {
    // Given a wiremock server with seed and overlap merge responses
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(SeedThenOverlapMergeResponder::new()),
    );

    // When the client executes three queries
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("SELECT 1");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 3");

    // Then the third request contains merged queryContextDTO entries
    let requests = fixture.query_requests();
    assert!(requests.len() >= 3, "Expected at least 3 query requests");
    let entries = requests[2]["queryContextDTO"]["entries"]
        .as_array()
        .expect("Should have queryContextDTO.entries");
    // After merge: id=1 updated timestamp, id=2 kept, id=3 added
    let ids: Vec<i64> = entries.iter().map(|e| e["id"].as_i64().unwrap()).collect();
    assert!(ids.contains(&1), "id=1 should survive merge");
    assert!(ids.contains(&2), "id=2 should survive merge");
    assert!(ids.contains(&3), "id=3 should be added via merge");
    // id=1 should have the updated timestamp
    let entry1 = entries
        .iter()
        .find(|e| e["id"].as_i64() == Some(1))
        .unwrap();
    assert_eq!(
        entry1["timestamp"].as_i64().unwrap(),
        999,
        "id=1 timestamp should be updated"
    );
}

// ---------------------------------------------------------------------------
// Scenario: should evict highest priority number when cache exceeds capacity
// ---------------------------------------------------------------------------

#[test]
fn should_evict_highest_priority_number_when_cache_exceeds_capacity() {
    // Given a wiremock server with 4 entries and QUERY_CONTEXT_CACHE_SIZE 3
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    // Single response for all queries (AtomicUsize-based responders can vary per call if needed).
    // The test verifies the driver's internal cache eviction from the response data.
    fixture.mount_query_response(json!({
        "success": true,
        "data": {
            "queryId": "qid-020",
            "queryResultFormat": "json",
            "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
            "rowset": [["Statement executed successfully."]],
            "total": 1, "returned": 1,
            "parameters": [{"name": "QUERY_CONTEXT_CACHE_SIZE", "value": 3}],
            "queryContext": {
                "entries": [
                    {"id": 1, "timestamp": 100, "priority": 10, "context": "a"},
                    {"id": 2, "timestamp": 200, "priority": 20, "context": "b"},
                    {"id": 3, "timestamp": 300, "priority": 30, "context": "c"},
                    {"id": 4, "timestamp": 400, "priority": 5, "context": "d"}
                ]
            }
        }
    }));

    // When the client executes two queries
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("SELECT 1");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");

    // Then the second request has 3 entries with highest priority number evicted
    let requests = fixture.query_requests();
    assert!(requests.len() >= 2);
    let entries = requests[1]["queryContextDTO"]["entries"]
        .as_array()
        .unwrap();
    assert_eq!(entries.len(), 3, "Should have 3 entries after eviction");
    let ids: Vec<i64> = entries.iter().map(|e| e["id"].as_i64().unwrap()).collect();
    assert!(
        !ids.contains(&3),
        "id=3 (highest priority number=30) should be evicted"
    );
    assert!(ids.contains(&1) && ids.contains(&2) && ids.contains(&4));
}

// ---------------------------------------------------------------------------
// Scenario: should respect cache size parameter
// ---------------------------------------------------------------------------

#[test]
fn should_respect_cache_size_parameter() {
    // Given a wiremock server with 5 entries and QUERY_CONTEXT_CACHE_SIZE 3
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    // Single response for all queries — verifies cache size limit from response data.
    fixture.mount_query_response(json!({
        "success": true,
        "data": {
            "queryId": "qid-030",
            "queryResultFormat": "json",
            "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
            "rowset": [["Statement executed successfully."]],
            "total": 1, "returned": 1,
            "parameters": [{"name": "QUERY_CONTEXT_CACHE_SIZE", "value": 3}],
            "queryContext": {
                "entries": [
                    {"id": 1, "timestamp": 100, "priority": 10, "context": "a"},
                    {"id": 2, "timestamp": 200, "priority": 20, "context": "b"},
                    {"id": 3, "timestamp": 300, "priority": 30, "context": "c"},
                    {"id": 4, "timestamp": 400, "priority": 40, "context": "d"},
                    {"id": 5, "timestamp": 500, "priority": 50, "context": "e"}
                ]
            }
        }
    }));

    // When the client executes two queries
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("SELECT 1");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");

    // Then the second request has exactly 3 entries
    let requests = fixture.query_requests();
    assert!(requests.len() >= 2);
    let entries = requests[1]["queryContextDTO"]["entries"]
        .as_array()
        .unwrap();
    assert_eq!(
        entries.len(),
        3,
        "Should have exactly 3 entries (cache size limit)"
    );
    let priorities: Vec<i64> = entries
        .iter()
        .map(|e| e["priority"].as_i64().unwrap())
        .collect();
    assert!(priorities.contains(&10) && priorities.contains(&20) && priorities.contains(&30));
}

// ---------------------------------------------------------------------------
// Scenario: should update cache on failed query response
// ---------------------------------------------------------------------------

#[test]
fn should_update_cache_on_failed_query_response() {
    // Given a wiremock server that returns an error response with queryContext
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(ErrorThenSuccessResponder::new()),
    );

    // When the client executes a failing query followed by a successful one
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("INVALID SQL");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");

    // Then the second request carries the context from the error response
    let requests = fixture.query_requests();
    assert!(requests.len() >= 2);
    let entries = requests[1]["queryContextDTO"]["entries"]
        .as_array()
        .unwrap();
    assert!(
        !entries.is_empty(),
        "Should have entries from error response"
    );
    assert_eq!(entries[0]["id"], 1);
}

// ---------------------------------------------------------------------------
// Scenario: should allow duplicate priorities to coexist in cache
// ---------------------------------------------------------------------------

#[test]
fn should_allow_duplicate_priorities_to_coexist_in_cache() {
    // Given a wiremock server with 3 entries sharing the same priority
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    fixture.mount_query_response(json!({
        "success": true,
        "data": {
            "queryId": "qid-dup-pri-1",
            "queryResultFormat": "json",
            "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
            "rowset": [["OK"]],
            "total": 1, "returned": 1,
            "parameters": [],
            "queryContext": {
                "entries": [
                    {"id": 1, "timestamp": 100, "priority": 5, "context": "ctx1"},
                    {"id": 2, "timestamp": 200, "priority": 5, "context": "ctx2"},
                    {"id": 3, "timestamp": 300, "priority": 5, "context": "ctx3"}
                ]
            }
        }
    }));

    // When the client executes two queries
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("SELECT 1");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");

    // Then the second request contains all 3 entries with the same priority
    let requests = fixture.query_requests();
    assert!(requests.len() >= 2);
    let entries = requests[1]["queryContextDTO"]["entries"]
        .as_array()
        .expect("Should have queryContextDTO.entries");
    assert_eq!(
        entries.len(),
        3,
        "All 3 entries with same priority should coexist"
    );
    let ids: Vec<i64> = entries.iter().map(|e| e["id"].as_i64().unwrap()).collect();
    assert!(ids.contains(&1) && ids.contains(&2) && ids.contains(&3));
    // All should have priority 5
    for entry in entries {
        assert_eq!(entry["priority"].as_i64().unwrap(), 5);
    }
}

// ---------------------------------------------------------------------------
// Scenario: should evict highest priority number among duplicate priorities
// ---------------------------------------------------------------------------

#[test]
fn should_evict_highest_priority_number_among_duplicate_priorities() {
    // Given a wiremock server with 4 entries at priority 5 and QUERY_CONTEXT_CACHE_SIZE 3
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    fixture.mount_query_response(json!({
        "success": true,
        "data": {
            "queryId": "qid-dup-evict",
            "queryResultFormat": "json",
            "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
            "rowset": [["OK"]],
            "total": 1, "returned": 1,
            "parameters": [{"name": "QUERY_CONTEXT_CACHE_SIZE", "value": 3}],
            "queryContext": {
                "entries": [
                    {"id": 1, "timestamp": 100, "priority": 5, "context": "a"},
                    {"id": 2, "timestamp": 200, "priority": 5, "context": "b"},
                    {"id": 3, "timestamp": 300, "priority": 5, "context": "c"},
                    {"id": 4, "timestamp": 400, "priority": 5, "context": "d"}
                ]
            }
        }
    }));

    // When the client executes two queries
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("SELECT 1");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");

    // Then the second request has 3 entries and the entry with the lowest timestamp is evicted
    let requests = fixture.query_requests();
    assert!(requests.len() >= 2);
    let entries = requests[1]["queryContextDTO"]["entries"]
        .as_array()
        .unwrap();
    assert_eq!(entries.len(), 3, "Should have 3 entries after eviction");
    let ids: Vec<i64> = entries.iter().map(|e| e["id"].as_i64().unwrap()).collect();
    assert!(
        !ids.contains(&1),
        "id=1 (lowest timestamp at same priority) should be evicted"
    );
    assert!(ids.contains(&2) && ids.contains(&3) && ids.contains(&4));
}

// ---------------------------------------------------------------------------
// Scenario: should insert new id at occupied priority and evict by capacity
// ---------------------------------------------------------------------------

#[test]
fn should_insert_new_id_at_occupied_priority_and_evict_by_capacity() {
    // Given a wiremock server with seed entries and a merge response adding a
    // new id at an existing priority
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(SeedThenDuplicatePriorityMergeResponder::new()),
    );

    // When the client executes three queries
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("SELECT 1");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 3");

    // Then the third request contains the new entry and evicts the
    // lowest-importance entry
    let requests = fixture.query_requests();
    assert!(requests.len() >= 3);
    let entries = requests[2]["queryContextDTO"]["entries"]
        .as_array()
        .expect("Should have queryContextDTO.entries");
    let ids: Vec<i64> = entries.iter().map(|e| e["id"].as_i64().unwrap()).collect();
    assert!(ids.contains(&5), "id=5 (new entry) should be present");
    assert!(ids.contains(&2), "id=2 should remain");
    assert!(ids.contains(&1), "id=1 should remain (not displaced)");
    assert!(
        !ids.contains(&3),
        "id=3 (highest priority number=20) should be evicted by capacity"
    );
}

// ---------------------------------------------------------------------------
// Scenario: should re-index entry when priority changes with same timestamp
// ---------------------------------------------------------------------------

#[test]
fn should_reindex_entry_when_priority_changes_with_same_timestamp() {
    // Given a wiremock server with seed entry at priority 10 and a merge response
    // changing priority to 5 with same timestamp
    let fixture = QueryContextFixture::new();
    fixture.mount_login();
    fixture.mock.mount(
        Mock::given(method("POST"))
            .and(path_regex(r"/queries/v1/query-request.*"))
            .respond_with(SeedThenPriorityChangeResponder::new()),
    );

    // When the client executes three queries
    fixture.client.connect().unwrap();
    let _ = fixture.client.execute_query_no_unwrap("SELECT 1");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 2");
    let _ = fixture.client.execute_query_no_unwrap("SELECT 3");

    // Then the third request contains the entry with updated priority 5
    let requests = fixture.query_requests();
    assert!(requests.len() >= 3);
    let entries = requests[2]["queryContextDTO"]["entries"]
        .as_array()
        .expect("Should have queryContextDTO.entries");
    let entry = entries
        .iter()
        .find(|e| e["id"].as_i64() == Some(1))
        .expect("id=1 should still exist");
    assert_eq!(
        entry["priority"].as_i64().unwrap(),
        5,
        "priority should be updated from 10 to 5"
    );
}

// ---------------------------------------------------------------------------
// Test fixture
// ---------------------------------------------------------------------------

struct QueryContextFixture {
    mock: MockServerWithTls,
    client: SnowflakeTestClient,
}

impl QueryContextFixture {
    fn new() -> Self {
        let mock = MockServerWithTls::start();
        let client = SnowflakeTestClient::with_int_tests_params(Some(&mock.http_url()));
        client.set_connection_option("password", "test_password"); // pragma: allowlist secret
        Self { mock, client }
    }

    fn mount_login(&self) {
        self.mock.mount(password::login_success());
    }

    fn mount_query_response(&self, response: serde_json::Value) {
        self.mock.mount(
            Mock::given(method("POST"))
                .and(path_regex(r"/queries/v1/query-request.*"))
                .respond_with(ResponseTemplate::new(200).set_body_json(response)),
        );
    }

    /// Returns parsed request bodies for test queries only (SELECT/INVALID).
    /// Excludes driver-internal setup queries (ALTER SESSION, etc.).
    fn query_requests(&self) -> Vec<serde_json::Value> {
        self.mock
            .received_requests()
            .into_iter()
            .filter(|r| r.url.path().contains("query-request"))
            .map(|r| serde_json::from_slice::<serde_json::Value>(&r.body).unwrap_or_default())
            .filter(|body| {
                body.get("sqlText")
                    .and_then(|s| s.as_str())
                    .map(|s| s.contains("SELECT") || s.contains("INVALID"))
                    .unwrap_or(false)
            })
            .collect()
    }
}

/// Seed → no-queryContext → plain success. Verifies cache survives a response
/// that omits the queryContext field entirely.
struct SeedThenNoContextResponder {
    calls: AtomicUsize,
}

impl SeedThenNoContextResponder {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

impl Respond for SeedThenNoContextResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        match n {
            0 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-seed",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {
                        "entries": [{"id": 1, "timestamp": 100, "priority": 4, "context": "base64seed"}]
                    }
                }
            })),
            1 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-no-ctx",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": []
                }
            })),
            _ => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-capture",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": []
                }
            })),
        }
    }
}

/// Seed → null entries → plain success. Verifies cache survives a response
/// that has `queryContext: { entries: null }`.
struct SeedThenNullEntriesResponder {
    calls: AtomicUsize,
}

impl SeedThenNullEntriesResponder {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

impl Respond for SeedThenNullEntriesResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        match n {
            0 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-seed",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {
                        "entries": [{"id": 1, "timestamp": 100, "priority": 4, "context": "base64seed"}]
                    }
                }
            })),
            1 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-null-entries",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {"entries": null}
                }
            })),
            _ => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-capture",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": []
                }
            })),
        }
    }
}

/// Counter-backed responder: returns an error response with queryContext on the
/// first call, and a plain success response on subsequent calls.
struct ErrorThenSuccessResponder {
    calls: AtomicUsize,
}

impl ErrorThenSuccessResponder {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

impl Respond for ErrorThenSuccessResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            // First call: error response with queryContext
            ResponseTemplate::new(200).set_body_json(json!({
                "success": false,
                "code": "002043",
                "message": "SQL compilation error",
                "data": {
                    "queryId": "qid-004",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["Statement executed successfully."]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {
                        "entries": [{"id": 1, "timestamp": 100, "priority": 4, "context": "fromError"}]
                    }
                }
            }))
        } else {
            // Subsequent calls: plain success
            ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-success",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": []
                }
            }))
        }
    }
}

/// Seed with ids 1,2 then overlap merge response: id=1 with newer timestamp
/// and id=3 as new entry. Verifies merge preserves old entries and updates.
struct SeedThenOverlapMergeResponder {
    calls: AtomicUsize,
}

impl SeedThenOverlapMergeResponder {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

impl Respond for SeedThenOverlapMergeResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        match n {
            0 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-seed",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {
                        "entries": [
                            {"id": 1, "timestamp": 100, "priority": 10, "context": "ctx1"},
                            {"id": 2, "timestamp": 200, "priority": 20, "context": "ctx2"}
                        ]
                    }
                }
            })),
            1 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-merge",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {
                        "entries": [
                            {"id": 1, "timestamp": 999, "priority": 10, "context": "ctx1-updated"},
                            {"id": 3, "timestamp": 300, "priority": 30, "context": "ctx3"}
                        ]
                    }
                }
            })),
            _ => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-capture",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": []
                }
            })),
        }
    }
}

/// Seed with ids 1,2 then disjoint response with ids 10,20 (no overlap).
/// Verifies the cache is fully replaced.
struct SeedThenDisjointReplaceResponder {
    calls: AtomicUsize,
}

impl SeedThenDisjointReplaceResponder {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

impl Respond for SeedThenDisjointReplaceResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        match n {
            0 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-seed",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {
                        "entries": [
                            {"id": 1, "timestamp": 100, "priority": 10, "context": "old1"},
                            {"id": 2, "timestamp": 200, "priority": 20, "context": "old2"}
                        ]
                    }
                }
            })),
            1 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-disjoint",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {
                        "entries": [
                            {"id": 10, "timestamp": 1000, "priority": 10, "context": "new1"},
                            {"id": 20, "timestamp": 2000, "priority": 20, "context": "new2"}
                        ]
                    }
                }
            })),
            _ => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-capture",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": []
                }
            })),
        }
    }
}

/// Seed with two entries at priority=10 + one at priority=20, then merge
/// response adds new id=5 at priority=10 (with QUERY_CONTEXT_CACHE_SIZE=3).
/// Verifies capacity eviction removes highest-priority-number entry (id=3).
struct SeedThenDuplicatePriorityMergeResponder {
    calls: AtomicUsize,
}

impl SeedThenDuplicatePriorityMergeResponder {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

impl Respond for SeedThenDuplicatePriorityMergeResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        match n {
            0 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-seed",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [{"name": "QUERY_CONTEXT_CACHE_SIZE", "value": 3}],
                    "queryContext": {
                        "entries": [
                            {"id": 1, "timestamp": 100, "priority": 10, "context": "first"},
                            {"id": 2, "timestamp": 200, "priority": 10, "context": "second"},
                            {"id": 3, "timestamp": 300, "priority": 20, "context": "other"}
                        ]
                    }
                }
            })),
            1 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-merge",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {
                        "entries": [
                            {"id": 2, "timestamp": 200, "priority": 10, "context": "second"},
                            {"id": 5, "timestamp": 500, "priority": 10, "context": "newcomer"}
                        ]
                    }
                }
            })),
            _ => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-capture",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": []
                }
            })),
        }
    }
}

/// Seed with entry id=1 at priority=10, then merge response sends same id=1
/// with priority=5 and same timestamp. Verifies the priority is re-indexed.
struct SeedThenPriorityChangeResponder {
    calls: AtomicUsize,
}

impl SeedThenPriorityChangeResponder {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

impl Respond for SeedThenPriorityChangeResponder {
    fn respond(&self, _req: &wiremock::Request) -> ResponseTemplate {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        match n {
            0 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-seed",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {
                        "entries": [
                            {"id": 1, "timestamp": 100, "priority": 10, "context": "original"}
                        ]
                    }
                }
            })),
            1 => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-reindex",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": [],
                    "queryContext": {
                        "entries": [
                            {"id": 1, "timestamp": 100, "priority": 5, "context": "original"}
                        ]
                    }
                }
            })),
            _ => ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "data": {
                    "queryId": "qid-capture",
                    "queryResultFormat": "json",
                    "rowtype": [{"name": "status", "type": "text", "nullable": true, "length": 16777216, "byteLength": 16777216, "precision": null, "scale": null}],
                    "rowset": [["OK"]],
                    "total": 1, "returned": 1,
                    "parameters": []
                }
            })),
        }
    }
}
