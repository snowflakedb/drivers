"""Integration tests for session logout functionality."""

import pytest


class TestLogoutIntegration:
    """Integration tests for logout logic."""

    @pytest.mark.skip(reason="TODO: SNOW-2872349")
    def test_should_return_true_when_first_running_async_query_is_detected_without_checking_remaining_queries(self):
        #Given Async query registry contains multiple queries
        #And First query in registry is running
        #When Auto-detection checks for running queries
        #Then Detection returns true immediately
        #And Remaining queries are not checked
        pytest.fail("TODO: SNOW-2872349")
