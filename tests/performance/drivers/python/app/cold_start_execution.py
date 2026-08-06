"""Cold-start child process: measures import → connect → SELECT 1.

Invoked as a subprocess by main.py for each cold-start iteration.
Prints a single CSV row to stdout so the parent can aggregate results.

Environment variables:
    CONNECTION_PARAMS_JSON  – JSON-encoded dict of snowflake.connector.connect() kwargs
    DRIVER_TYPE             – "universal" or "old" (for WireMock TLS/proxy setup)
"""
import json
import os
import resource
import sys
import time

t0 = time.perf_counter()

from snowflake import connector  # noqa: E402  — import timing is the point

t1 = time.perf_counter()

from config import _disable_tls_verification_for_wiremock

_disable_tls_verification_for_wiremock(os.getenv("DRIVER_TYPE", "universal"))

params = json.loads(os.environ["CONNECTION_PARAMS_JSON"])
conn = connector.connect(**params)

t2 = time.perf_counter()

cur = conn.cursor()
cur.execute("SELECT 1")
row = cur.fetchone()
assert row[0] == 1, f"Expected 1, got {row[0]}"
cur.close()
conn.close()

t3 = time.perf_counter()

timestamp_ms = int(time.time() * 1000)
e2e_s = t3 - t0
load_s = t1 - t0
connect_s = t2 - t1
select1_s = t3 - t2
cpu_time_s = time.process_time()
if sys.platform == "darwin":
    peak_rss_mb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / (1024 * 1024)
else:
    peak_rss_mb = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024

print(f"{timestamp_ms},{e2e_s:.6f},{load_s:.6f},{connect_s:.6f},{select1_s:.6f},{cpu_time_s:.6f},{peak_rss_mb:.1f}")
