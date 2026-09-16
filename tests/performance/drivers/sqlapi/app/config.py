"""Configuration parsing for the SQL API perf driver."""

import json
import os
import sys

from client import sqlapi_format


class TestConfig:
    def __init__(self):
        self.sql_command = os.getenv("SQL_COMMAND")
        self.test_name = os.getenv("TEST_NAME")
        self.iterations = int(os.getenv("PERF_ITERATIONS", "1"))
        self.warmup_iterations = int(os.getenv("PERF_WARMUP_ITERATIONS", "0"))
        self.params_json = os.getenv("PARAMETERS_JSON")
        self.setup_queries_json = os.getenv("SETUP_QUERIES")
        # e2e_runner omits FETCH_MODE when the test uses the default fetchmany.
        self.fetch_mode = os.getenv("FETCH_MODE", "sequential")
        if self.fetch_mode in ("fetchmany", "sequential", ""):
            self.fetch_mode = "sequential"
        if self.fetch_mode != "sequential":
            print(
                f"ERROR: Invalid fetch mode '{self.fetch_mode}'. "
                "SQL API supports: sequential"
            )
            sys.exit(1)
        self.result_format = (os.getenv("RESULT_FORMAT") or "arrow").strip().lower()
        sqlapi_format(self.result_format)
        if not all([self.sql_command, self.test_name, self.params_json]):
            print("ERROR: Missing required environment variables")
            sys.exit(1)

    def get_setup_queries(self):
        if self.setup_queries_json:
            return json.loads(self.setup_queries_json)
        return []
