# Snowflake.Data.Tests.Common

Shared test infrastructure consumed by both `Snowflake.Data.Tests` and `Snowflake.Data.Tests.Reference`.

This is a **class library** (`IsTestProject=false`) — xUnit will not discover tests here.

## Dual xUnit support

Targets both xUnit v3 (net8.0+) and xUnit v2 (net472/net481) via conditional compilation (`OLD_XUNIT`).
The `targets/` directory contains the package reference imports for each version.
