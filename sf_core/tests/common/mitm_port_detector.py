"""mitmdump addon for the universal driver's live-proxy e2e test.

Loaded via `mitmdump -s <this file>` by `common::mitm_proxy::MitmProxy`. Two jobs:

1. `running()` — after mitmdump binds its listener (started with `--listen-port 0`
   so the OS picks a free port), write the actual bound port to `$MITM_PORT_FILE`.
   The Rust side polls that file to learn which port to point `ProxyConfig` at,
   avoiding a bind-then-reuse port race.
2. `request()` — append one NDJSON line (`{"method","host","path"}`) per
   intercepted request to `$MITM_REQUEST_LOG`. The Rust side reads it back
   (`MitmProxy::recorded_requests`) to assert, by path and count, that the
   expected storage requests transited the proxy.

The CA-trust-exclusivity proof still stands on its own: the test trusts *only*
mitmdump's generated CA (`custom_root_store_path` replaces the built-in roots),
so a byte-for-byte PUT/GET succeeding over HTTPS is already only possible if the
traffic transited mitmdump. The request log is independent, direct corroboration
of that structural argument — not a replacement for it.

Ported from snowflake-connector-python's `port_detector_addon.py`.
"""

import json
import logging
import os
import sys

logger = logging.getLogger(__name__)


def running():
    from mitmproxy import ctx

    port_file = os.environ.get("MITM_PORT_FILE")
    if not port_file:
        logger.error("MITM_PORT_FILE environment variable not set")
        sys.exit(1)

    # `--listen-port 0` binds an OS-assigned port; recover the real one.
    # listen_addrs() -> [('::', port, 0, 0), ('0.0.0.0', port)].
    addrs = ctx.master.addons.get("proxyserver").listen_addrs()
    if not addrs:
        logger.error("proxyserver reported no listen addresses")
        sys.exit(1)

    port = addrs[0][1]
    try:
        with open(port_file, "w") as f:
            f.write(str(port))
    except OSError as e:
        logger.error(f"failed to write port to {port_file}: {e}")
        sys.exit(1)


def request(flow):
    # One NDJSON line per request (method/host/path, no body). Open/append/close
    # per request so a reader mid-transfer never sees a partial line; a missing
    # env var / write error logs and returns rather than sys.exit(), so a logging
    # hiccup never tears down the proxy mid-transfer.
    log_file = os.environ.get("MITM_REQUEST_LOG")
    if not log_file:
        return
    record = {
        "method": flow.request.method,
        "host": flow.request.pretty_host,
        "path": flow.request.path,
    }
    try:
        with open(log_file, "a") as f:
            f.write(json.dumps(record) + "\n")
    except OSError as e:
        logger.warning(f"failed to append request to {log_file}: {e}")
