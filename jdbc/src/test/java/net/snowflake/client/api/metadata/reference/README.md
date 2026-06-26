# Reference DatabaseMetaData integration tests

Integration tests ported from the legacy **snowflake-jdbc** driver's internal JDBC metadata test suite.
They exercise `DatabaseMetaData` behavior against a live Snowflake account and serve as a parity harness while universal-driver implements metadata support.

## Source

These classes were copied and adapted from snowflake-jdbc metadata ITs (e.g. `DatabaseMetaDataIT`, `DatabaseMetaDataInternalIT`, and related helpers).
Supporting types in this package (`TestUtil`, `MetaDataResultSetFormat`, base classes) exist only to keep the ported tests runnable here.

## Sunset

*This package is temporary.*

Remove it once the universal-driver metadata test suite is mature enough to replace the legacy parity harness.
