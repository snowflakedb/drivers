# Changelog

## Upcoming Release

Changes:

- GCS GET: disable reqwest auto-gunzip so wire bytes match stored object. (snowflakedb/universal-driver#60)
- GCS multi-file downloads now use the per-file presigned URLs supplied by the server
  (`data.presignedUrls`), with graceful handling of list-length mismatches. (snowflakedb/universal-driver#61)
- GCS GET and PUT now reactively recover from expired credentials: a 400 on a presigned
  URL triggers a one-time URL refresh and retry; a 401 with a bearer token triggers a
  one-time token refresh and retry. Multi-file glob PUT batches recover per file by
  re-issuing the PUT command rewritten for the current destination file, matching the
  Python and ODBC drivers. (snowflakedb/universal-driver#62)

