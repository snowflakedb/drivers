"""Configuration parsing for the ADBC perf driver."""

import json
import os
import sys


class TestConfig:
    def __init__(self):
        self.sql_command = os.getenv("SQL_COMMAND")
        self.test_name = os.getenv("TEST_NAME")
        self.iterations = int(os.getenv("PERF_ITERATIONS", "1"))
        self.warmup_iterations = int(os.getenv("PERF_WARMUP_ITERATIONS", "0"))
        self.params_json = os.getenv("PARAMETERS_JSON")
        self.setup_queries_json = os.getenv("SETUP_QUERIES")
        # Arrow-native path is the fair comparison to Python fetch_arrow_batches.
        # e2e_runner omits FETCH_MODE when the test uses the default fetchmany.
        fetch_mode = os.getenv("FETCH_MODE", "arrow_batches")
        if fetch_mode in ("fetchmany", "arrow_batches", ""):
            fetch_mode = "arrow_batches"
        if fetch_mode != "arrow_batches":
            print(
                f"ERROR: Invalid fetch mode '{fetch_mode}'. "
                "ADBC supports: arrow_batches (iterates cursor.fetch_record_batch())"
            )
            sys.exit(1)
        self.fetch_mode = fetch_mode
        if not all([self.sql_command, self.test_name, self.params_json]):
            print("ERROR: Missing required environment variables")
            sys.exit(1)

    def get_setup_queries(self):
        if self.setup_queries_json:
            return json.loads(self.setup_queries_json)
        return []
