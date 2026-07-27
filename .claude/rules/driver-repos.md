# Driver Repository Locations

Canonical reference for original driver repo identifiers and local paths.
Used by skills that need to locate or access old-driver source code.

| Driver | GitHub repo | Typical local paths |
|---|---|---|
| Python | `snowflakedb/snowflake-connector-python` | `~/snowflake-connector-python`, `~/emu/snowflake-connector-python` |
| ODBC | `snowflakedb/snowflake-odbc` | `~/snowflake-odbc`, `~/emu/snowflake-odbc` |
| JDBC | `snowflakedb/snowflake-jdbc` | `~/snowflake-jdbc`, `~/emu/snowflake-jdbc` |
| Node.js | `snowflakedb/snowflake-connector-nodejs` | `~/snowflake-connector-nodejs`, `~/emu/snowflake-connector-nodejs` |

If a repo does not exist at the expected local path, tell the user and ask for
the correct location, or fall back to GitHub MCP (`mcp__github__github_get_file`)
using the GitHub repo column.

<!-- sync-target: this file is a reference doc loaded on demand via @ include in
     skills. It is NOT alwaysApply, so no .cursor/rules counterpart is needed.
     Edit this file only. -->
