# Changelog

## Upcoming Release

Changes:

- GCS GET: disable reqwest auto-gunzip so wire bytes match stored object. (snowflake-eng/universal-driver#60)
- GCS multi-file downloads now use the per-file presigned URLs supplied by the server
  (`data.presignedUrls`), with graceful handling of list-length mismatches. (snowflake-eng/universal-driver#61)
- GCS GET and PUT now reactively recover from expired credentials: a 400 on a presigned
  URL triggers a one-time URL refresh and retry; a 401 with a bearer token triggers a
  one-time token refresh and retry. Multi-file glob PUT batches recover per file by
  re-issuing the PUT command rewritten for the current destination file, matching the
  Python and ODBC drivers. (snowflake-eng/universal-driver#62)
- Added HTTP proxy connection parameters (`proxy_host`, `proxy_port`, `proxy_user`, `proxy_password`, `no_proxy`) with ODBC aliases including legacy `PROXY` URL form; wired through `ProxyConfig` and `create_tls_client_with_proxy()`. Explicit `proxy_host` overrides env vars; otherwise reqwest env detection applies unless disabled. (snowflake-eng/universal-driver#29)
- GCS PUT: skip upload when remote `x-goog-meta-sfc-digest` matches local SHA-256. (snowflake-eng/universal-driver#57)
- CSE: compute the stored content digest over the plaintext (not the ciphertext) so it is stable across uploads, enables the content-match upload skip on client-side-encrypted stages, and is interoperable with other drivers. (snowflake-eng/universal-driver#57)
- GCS GET: fail when response body length differs from `Content-Length`. (snowflake-eng/universal-driver#58)

