# .NET connection pool

The .NET wrapper exposes the connection pool API of the legacy .NET connector.
Today the API is a set of stubs. The planned first implementation never reuses
a session. This page records the decisions behind that shape so later work does
not rediscover them.

## Where the pool lives

The pool is C# code in the .NET wrapper. `sf_core` has no pool: it creates and
closes one connection per `DatabaseHandle` / `ConnectionHandle` pair, and the
other wrappers do not share a pool implementation with it.

## Public surface

The public surface is the multiple-pool surface of the legacy connector
(`snowflakedb/snowflake-connector-net`). Callers use only
`SnowflakeDbConnectionPool` and `SnowflakeDbConnection` for pooling:

| Type | Role |
| --- | --- |
| `SnowflakeDbConnectionPool` | Non-static class with only static members, like the legacy type: `ClearAllPools`, the global getters and setters, and the per-pool getters and `ClearPool`. |
| `SnowflakeDbConnection.PreventPooling()` | Throws `Exception` with the legacy message when no session exists. After `Open`, it is a no-op until reuse exists. In the legacy connector it keeps a closing connection from returning its session to a pool. |

The per-pool members take the connection string that identifies the pool,
followed by optional `password`, `oauthClientSecret`, and `token` secrets:
`GetPooling`, `GetMinPoolSize`, `GetMaxPoolSize`, `GetCurrentPoolSize`,
`GetExpirationTimeout`, `GetConnectionTimeout`, `GetWaitForIdleSessionTimeout`,
`GetChangedSession`, and `ClearPool`. The timeouts are in seconds. The
connection string is required, so `GetMaxPoolSize()`, `GetCurrentPoolSize()`,
and `GetPooling()` without arguments still call the global getters.

The legacy connector reaches the same members through
`SnowflakeDbConnectionPool.GetPool(...)`, which returns a `SnowflakeDbSessionPool`.
Neither is ported. That compile break is recorded in
`dotnet/behavior_differences/session/pool-session-pool-type-removed.yaml`.

The setters on `SnowflakeDbConnectionPool` (`SetMaxPoolSize`, `SetTimeout`,
`SetPooling`) throw `Exception` with the legacy message "You cannot change
connection pool parameters for all the pools. Instead you can change it on a
particular pool". Per-pool values are intended to come from the connection
string; no pool key is read today.

`SnowflakeDbConnectionPool.GetTimeout()` is the session expiration timeout in
the legacy connector, not the connection timeout.

## The cache-pool variant is not ported

`SnowflakeDbConnectionPool.SetOldConnectionPoolVersion()` switches every
connection on the legacy connector to one shared cache. The new driver does
not provide that method, so only the per-connection-string pools exist. That
compile break is recorded in
`dotnet/behavior_differences/session/pool-cache-variant-not-ported.yaml`.

## Seam and stub, not pooling

The public members are stubs that throw `NotImplementedException`, except the
setters above. `SnowflakeDbConnection` does not go through a pool. Nothing
reuses a session.

The planned follow-up adds an internal pool seam with acquire and release
members, routes `SnowflakeDbConnection.Open` and `Close` through it, and adds a
pool that opens a new session on every acquire and closes it on every release.
`PreventPooling()` still throws when no session exists. The pool reports the
legacy multiple-pool defaults without enforcing them:

| Setting | Default |
| --- | --- |
| Minimum pool size | 2 |
| Maximum pool size | 10 |
| Changed session | `Destroy` |
| Wait for idle session timeout | 30 seconds |
| Expiration timeout | 60 minutes |
| Pooling enabled | true |
| Connection timeout | 300 seconds |

`GetTimeout()` reports the expiration timeout default, 60 minutes.

Reuse, minimum and maximum enforcement, waiting for an idle session,
expiration, changed-session handling, and reading pool keys from the connection
string are not part of this work. Today `ConnectionStringParser` forwards every
key to `sf_core`.
