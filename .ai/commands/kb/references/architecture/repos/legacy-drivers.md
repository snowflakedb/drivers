---
description: The four legacy driver repos the Universal Driver replaces — their GitHub identifiers, the local paths a checkout is expected at, and the GitHub MCP fallback when no checkout exists.
no-pointer: true
---

# Driver Repository Locations

| Driver | GitHub repo | Typical local paths |
|---|---|---|
| Python | `snowflakedb/snowflake-connector-python` | `~/snowflake-connector-python`, `~/emu/snowflake-connector-python` |
| ODBC | `snowflakedb/snowflake-odbc` | `~/snowflake-odbc`, `~/emu/snowflake-odbc` |
| JDBC | `snowflakedb/snowflake-jdbc` | `~/snowflake-jdbc`, `~/emu/snowflake-jdbc` |
| Node.js | `snowflakedb/snowflake-connector-nodejs` | `~/snowflake-connector-nodejs`, `~/emu/snowflake-connector-nodejs` |

If a repo does not exist at the expected local path, tell the user and ask for
the correct location, or fall back to GitHub MCP (`mcp__github__github_get_file`)
using the GitHub repo column.

These repos are not precloned into an agent workspace. Which of their tests the Universal Driver covers is tracked separately — see [../../concepts/old-driver-coverage-attribution.md](../../concepts/old-driver-coverage-attribution.md).
