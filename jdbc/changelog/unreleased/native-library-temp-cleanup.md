---
category: bugfix
---

- Reclaimed leftover JDBC native-library copies under the JVM temp directory after a crash or a Windows run that could not delete a loaded DLL. (snowflakedb/drivers#3488)
