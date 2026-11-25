---
category: internal
---

- Replaced the `fs2` dependency with `std::fs::File::try_lock` for advisory file locking so that `sf_core` builds on AIX. (snowflakedb/drivers#3631)
