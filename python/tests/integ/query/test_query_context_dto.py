"""
Wiremock integration tests for query context DTO cache round-trip.

Implements scenarios from: tests/definitions/shared/query/query_context_dto.feature

Verifies that the driver correctly caches queryContext entries from responses
and includes them as queryContextDTO in subsequent requests.
"""

from __future__ import annotations

import json

import pytest


@pytest.mark.skip_reference(reason="Query context DTO cache is universal-driver-only")
class TestQueryContextDtoCache:
    # Scenario: should send cached query context in subsequent requests
    def test_should_send_cached_query_context_in_subsequent_requests(self, int_test_connection_factory, wiremock):
        # Given a wiremock server with login and query response containing queryContext
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_seed_response.json")

        # When the client executes two queries
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT 1")
                cur.execute("SELECT 2")

        # Then the second request contains the cached queryContextDTO entries
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 2

        second_req = json.loads(requests[1]["body"])
        dto = second_req.get("queryContextDTO", {})
        entries = dto.get("entries", [])
        assert len(entries) >= 1
        assert entries[0]["id"] == 1
        assert entries[0]["priority"] == 4
        assert entries[0]["timestamp"] == 100

    # Scenario: should keep cache unchanged when response has no queryContext
    def test_should_keep_cache_unchanged_when_response_has_no_query_context(
        self, int_test_connection_factory, wiremock
    ):
        # Given a wiremock server with response that has no queryContext field
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_seed_then_no_context.json")

        # When the client executes three queries
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT 1")
                cur.execute("SELECT 2")
                cur.execute("SELECT 3")

        # Then all three queries complete without error and the third
        # request still contains the cached entries from the seed response
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 3

        third_req = json.loads(requests[2]["body"])
        entries = third_req.get("queryContextDTO", {}).get("entries", [])
        assert len(entries) >= 1, "Cache should still have entries from seed"
        assert entries[0]["id"] == 1
        assert entries[0]["priority"] == 4

    # Scenario: should clear cache when response has null entries
    def test_should_clear_cache_when_response_has_null_entries(self, int_test_connection_factory, wiremock):
        # Given a wiremock server with response that has null queryContext entries
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_seed_then_null_entries.json")

        # When the client executes three queries
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT 1")
                cur.execute("SELECT 2")
                cur.execute("SELECT 3")

        # Then all three queries complete without error and the third request has no cached entries
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 3

        third_req = json.loads(requests[2]["body"])
        dto = third_req.get("queryContextDTO", {})
        entries = dto.get("entries")
        assert entries is None or entries == [], "Cache should be empty after null entries response"

    # Scenario: should merge entries when response IDs overlap
    def test_should_merge_entries_when_response_ids_overlap(self, int_test_connection_factory, wiremock):
        # Given a wiremock server with seed and overlap merge responses
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_seed_then_overlap_merge.json")

        # When the client executes three queries
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT 1")
                cur.execute("SELECT 2")
                cur.execute("SELECT 3")

        # Then the third request contains merged queryContextDTO entries
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 3

        third_req = json.loads(requests[2]["body"])
        entries = third_req.get("queryContextDTO", {}).get("entries", [])
        ids = {e["id"] for e in entries}
        assert ids == {1, 2, 3}, f"Expected merged ids {{1,2,3}}, got {ids}"

        entry1 = next(e for e in entries if e["id"] == 1)
        assert entry1["timestamp"] == 999, "id=1 timestamp should be updated"

    # Scenario: should evict highest priority number when cache exceeds capacity
    def test_should_evict_highest_priority_number_when_cache_exceeds_capacity(
        self, int_test_connection_factory, wiremock
    ):
        # Given a wiremock server with 4 entries and QUERY_CONTEXT_CACHE_SIZE 3
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_eviction_response.json")

        # When the client executes two queries
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT 1")
                cur.execute("SELECT 2")

        # Then the second request has 3 entries with highest priority number evicted
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 2

        second_req = json.loads(requests[1]["body"])
        entries = second_req.get("queryContextDTO", {}).get("entries", [])
        assert len(entries) == 3, f"Expected 3 entries after eviction, got {len(entries)}"
        ids = {e["id"] for e in entries}
        assert 3 not in ids, "id=3 (highest priority number=30) should be evicted"
        assert ids == {1, 2, 4}, f"Expected ids {{1, 2, 4}}, got {ids}"

    # Scenario: should respect cache size parameter
    def test_should_respect_cache_size_parameter(self, int_test_connection_factory, wiremock):
        # Given a wiremock server with 5 entries and QUERY_CONTEXT_CACHE_SIZE 3
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_cache_size_response.json")

        # When the client executes two queries
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT 1")
                cur.execute("SELECT 2")

        # Then the second request has exactly 3 entries
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 2

        second_req = json.loads(requests[1]["body"])
        entries = second_req.get("queryContextDTO", {}).get("entries", [])
        assert len(entries) == 3, f"Expected 3 entries (cache size limit), got {len(entries)}"
        priorities = {e["priority"] for e in entries}
        assert priorities == {10, 20, 30}, f"Should keep lowest priority numbers, got {priorities}"

    # Scenario: should update cache on failed query response
    def test_should_update_cache_on_failed_query_response(self, int_test_connection_factory, wiremock):
        # Given a wiremock server that returns an error response with queryContext
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_error_then_success.json")

        # When the client executes a failing query followed by a successful one
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                try:
                    cur.execute("INVALID SQL")
                except Exception:
                    pass
                cur.execute("SELECT 2")

        # Then the second request carries the context from the error response
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 2

        second_req = json.loads(requests[1]["body"])
        entries = second_req.get("queryContextDTO", {}).get("entries", [])
        assert len(entries) >= 1, "Cache should have entries from error response"
        assert entries[0]["id"] == 1

    # Scenario: should allow duplicate priorities to coexist in cache
    def test_should_allow_duplicate_priorities_to_coexist_in_cache(self, int_test_connection_factory, wiremock):
        # Given a wiremock server with 3 entries sharing the same priority
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_duplicate_priorities_response.json")

        # When the client executes two queries
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT 1")
                cur.execute("SELECT 2")

        # Then the second request contains all 3 entries with the same priority
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 2

        second_req = json.loads(requests[1]["body"])
        entries = second_req.get("queryContextDTO", {}).get("entries", [])
        assert len(entries) == 3, f"All 3 entries with same priority should coexist, got {len(entries)}"
        ids = {e["id"] for e in entries}
        assert ids == {1, 2, 3}, f"Expected ids {{1, 2, 3}}, got {ids}"
        for entry in entries:
            assert entry["priority"] == 5, f"All entries should have priority=5, got {entry['priority']}"

    # Scenario: should evict highest priority number among duplicate priorities
    def test_should_evict_highest_priority_number_among_duplicate_priorities(
        self, int_test_connection_factory, wiremock
    ):
        # Given a wiremock server with 4 entries at priority 5 and QUERY_CONTEXT_CACHE_SIZE 3
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_duplicate_priorities_eviction_response.json")

        # When the client executes two queries
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT 1")
                cur.execute("SELECT 2")

        # Then the second request has 3 entries and the entry with the lowest timestamp is evicted
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 2

        second_req = json.loads(requests[1]["body"])
        entries = second_req.get("queryContextDTO", {}).get("entries", [])
        assert len(entries) == 3, f"Expected 3 entries after eviction, got {len(entries)}"
        ids = {e["id"] for e in entries}
        assert 1 not in ids, "id=1 (lowest timestamp at same priority) should be evicted"
        assert ids == {2, 3, 4}, f"Expected ids {{2, 3, 4}}, got {ids}"

    # Scenario: should insert new id at occupied priority and evict by capacity
    def test_should_insert_new_id_at_occupied_priority_and_evict_by_capacity(
        self, int_test_connection_factory, wiremock
    ):
        # Given a wiremock server with seed entries and a merge response adding a new id at an existing priority
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_seed_then_duplicate_priority_merge.json")

        # When the client executes three queries
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT 1")
                cur.execute("SELECT 2")
                cur.execute("SELECT 3")

        # Then the third request contains the new entry and evicts the lowest-importance entry
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 3

        third_req = json.loads(requests[2]["body"])
        entries = third_req.get("queryContextDTO", {}).get("entries", [])
        ids = {e["id"] for e in entries}
        assert 5 in ids, "id=5 (new entry) should be present"
        assert 2 in ids, "id=2 should remain"
        assert 1 in ids, "id=1 should remain"
        assert 3 not in ids, "id=3 (highest priority number=20) should be evicted by capacity"

    # Scenario: should re-index entry when priority changes with same timestamp
    def test_should_re_index_entry_when_priority_changes_with_same_timestamp(
        self, int_test_connection_factory, wiremock
    ):
        # Given a wiremock server with seed entry at priority 10 and a merge
        # response changing priority to 5 with same timestamp
        wiremock.add_mapping("auth/login_success_any.json")
        wiremock.add_mapping("query_context/qcc_seed_then_priority_change.json")

        # When the client executes three queries
        with int_test_connection_factory(server_url=wiremock.http_url()) as conn:
            with conn.cursor() as cur:
                cur.execute("SELECT 1")
                cur.execute("SELECT 2")
                cur.execute("SELECT 3")

        # Then the third request contains the entry with updated priority 5
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) >= 3

        third_req = json.loads(requests[2]["body"])
        entries = third_req.get("queryContextDTO", {}).get("entries", [])
        entry = next((e for e in entries if e["id"] == 1), None)
        assert entry is not None, "id=1 should still exist"
        assert entry["priority"] == 5, f"priority should be updated from 10 to 5, got {entry['priority']}"
