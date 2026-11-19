---
category: bugfix
---

- Fixed `snowflake-jdbc-native-all` silencing an application's SLF4J 1.7 logging by leaving `slf4j-api` out of the fat jar. (snowflakedb/drivers#3461)
