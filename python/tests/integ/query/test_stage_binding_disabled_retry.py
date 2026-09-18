"""
Wiremock integration test for the SYSTEM$BIND stage-binding-disabled retry.

Verifies that when CLIENT_STAGE_ARRAY_BINDING_THRESHOLD selects stage (CSV)
binding and the SYSTEM$BIND stage creation is rejected, the driver retries
the same statement with inline JSON bindings instead of failing outright.
"""

from __future__ import annotations

import json

import pytest


@pytest.mark.skip_reference(reason="Stage-binding-disabled retry is universal-driver-only")
class TestStageBindingDisabledRetry:
    # Scenario: should retry with inline JSON bindings when the SYSTEM$BIND stage is disabled
    def test_should_retry_with_inline_json_bindings_when_stage_binding_is_disabled(
        self, int_test_connection_factory, wiremock
    ):
        # Given CLIENT_STAGE_ARRAY_BINDING_THRESHOLD is 1, so an array-bound INSERT
        # selects stage (CSV) binding, and CREATE TEMPORARY STAGE SYSTEM$BIND fails
        wiremock.add_mapping("auth/login_success_low_stage_binding_threshold.json")
        wiremock.add_mapping("query/describe_array_bind_supported.json")
        wiremock.add_mapping("query/create_stage_binding_disabled.json")
        wiremock.add_mapping("query/insert_success_after_stage_binding_retry.json")

        # When the statement is executed via executemany, which transposes the
        # single row into column-major array bindings and triggers stage binding
        with int_test_connection_factory(server_url=wiremock.http_url(), paramstyle="qmark") as conn:
            with conn.cursor() as cur:
                cur.executemany("INSERT INTO t (id) VALUES (?)", [(1,)])

        # Then it succeeds, having transparently retried with inline JSON bindings
        requests = wiremock.get_requests("/queries/v1/query-request.*")
        assert len(requests) == 3

        describe_count = 0
        create_stage_count = 0
        retried_insert_count = 0
        for req in requests:
            body = json.loads(req["body"])
            if "describeOnly" in body:
                describe_count += 1
            elif "CREATE TEMPORARY STAGE" in body["sqlText"]:
                create_stage_count += 1
            else:
                retried_insert_count += 1
                assert "bindings" in body
                assert "bindStage" not in body

        assert describe_count == 1
        assert create_stage_count == 1
        assert retried_insert_count == 1
