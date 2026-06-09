# Changelog

## Upcoming Release

Changes:

- GCS GET: disable reqwest auto-gunzip so wire bytes match stored object. (snowflake-eng/universal-driver#60)
- GCS multi-file downloads now use the per-file presigned URLs supplied by the server
  (`data.presignedUrls`), with graceful handling of list-length mismatches. (snowflake-eng/universal-driver#61)
