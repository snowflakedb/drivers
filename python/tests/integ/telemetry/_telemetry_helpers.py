"""Shared helpers for the telemetry integration tests.

One place for the JWT-auth connection overrides and the decoding of
Wiremock-captured ``/telemetry/send`` bodies, so the per-lane telemetry tests
don't each re-implement them.
"""

from __future__ import annotations

import gzip
import json

from pathlib import Path

from cryptography.hazmat.primitives.serialization import Encoding, NoEncryption, PrivateFormat, load_pem_private_key

from tests.compatibility import is_new_driver
from tests.private_key_helper import get_test_private_key_path


def jwt_private_key_params() -> dict:
    """Connection overrides needed for JWT auth against Wiremock.

    The old driver needs ``private_key`` as DER bytes; the universal driver
    accepts ``private_key_file`` directly (so returns no overrides).
    """
    if is_new_driver():
        return {}
    pem_data = Path(get_test_private_key_path()).read_bytes()
    pk = load_pem_private_key(pem_data, password=None)
    return {
        "private_key": pk.private_bytes(Encoding.DER, PrivateFormat.PKCS8, NoEncryption()),
        "private_key_file": None,
    }


def decode_telemetry_body(request: dict) -> dict:
    """Decode the JSON body of a Wiremock-captured ``/telemetry/send`` request.

    Wiremock may transparently decompress gzip, so try both.
    """
    raw_body = request["body"]
    if isinstance(raw_body, str):
        raw_body = raw_body.encode("latin-1")
    try:
        return json.loads(gzip.decompress(raw_body))
    except gzip.BadGzipFile:
        return json.loads(raw_body)


def collect_log_entries(telemetry_requests: list[dict]) -> list[dict]:
    """Flatten the ``logs`` arrays across every captured telemetry request."""
    entries: list[dict] = []
    for request in telemetry_requests:
        body = decode_telemetry_body(request)
        for entry in body.get("logs", []):
            if isinstance(entry, dict) and isinstance(entry.get("message"), dict):
                entries.append(entry)
    return entries
