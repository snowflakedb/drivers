---
category: internal
---
- Taught the bundled native library loader to extract and load the companion libraries listed in a per-platform `companions.txt` before `jdbc_bridge`, and added a `-Pfips` Gradle build that packages the FIPS natives under the `snowflake-jdbc-native-fips` and `snowflake-jdbc-native-all-fips` coordinates. (snowflakedb/drivers#3523)
