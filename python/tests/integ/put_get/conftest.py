import pytest


def _strip_azure_blob_prefix(url: str) -> str:
    """Recover the embedded wiremock URL from an old-driver-mangled `<account>.blob.<scheme>://...` Azure URL."""
    AZURE_BLOB_MARKERS = (".blob.http://", ".blob.https://")
    for marker in AZURE_BLOB_MARKERS:
        idx = url.find(marker)
        if idx > 0:
            return url[idx + len(".blob.") :]
    return url


@pytest.fixture
def old_driver_azure_routes_to_wiremock(wiremock, monkeypatch):
    """Strip the `<account>.blob.<endpoint>` prefix from old-driver Azure URLs so they reach wiremock."""
    try:
        from snowflake.connector.vendored import requests as vendored_requests

        orig_request = vendored_requests.Session.request

        def patched_request(self, method, url, *args, **kwargs):
            return orig_request(self, method, _strip_azure_blob_prefix(url), *args, **kwargs)

        monkeypatch.setattr(vendored_requests.Session, "request", patched_request)
    except ImportError:
        # vendored requests isn't available in this environment, so there's nothing to route.
        pass

    yield
