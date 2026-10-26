# JDBC wrapper

JDBC-specific troubleshooting for the sf_core-based `snowflake-jdbc` driver:
native-library loading over JNI, the two-way logging bridge (which is where most
JDBC-specific confusion lives), and the fact that the core TLS stack is **not**
JSSE. For auth, TLS/CRL, and query issues that aren't JDBC-specific, follow the
links into the [core pages](../index.md).

Source tree: `jdbc/` (Java) + `jdbc_bridge/` (Rust JNI bridge).

---

## Architecture

```
java.sql API  (DriverManager / DataSource)
        │  net.snowflake.client.api.driver.SnowflakeDriver   (URL: jdbc:snowflake://…)
        ▼
jdbc_bridge  (JNI)                       jdbc_bridge/src/
        ▼
sf_core  (protobuf dispatch, Rust-native HTTP + TLS)
```

The driver class is `net.snowflake.client.api.driver.SnowflakeDriver`; connection
URLs start with `jdbc:snowflake://`. The core HTTP/TLS transport is **Rust-native
(rustls)**, reached over JNI — it is **not** the JVM's JSSE stack. That single fact
explains the TLS symptom below.

---

## Logging (the JDBC-specific part)

This is where JDBC differs most from the other wrappers, and where stale advice
causes the most wasted time. Two things are selectable:

### 1. Which logging framework receives the events

Selected by the JVM system property `net.snowflake.jdbc.loggerImpl`
(`jdbc/src/main/java/net/snowflake/client/internal/log/SFLoggerFactory.java`):

| `-Dnet.snowflake.jdbc.loggerImpl=…` | Framework | Default? |
|---|---|---|
| *(unset)* or `net.snowflake.client.log.JDK14Logger` | `java.util.logging` (JUL) | **yes** |
| `net.snowflake.client.log.SLF4JLogger` | SLF4J (→ logback / log4j2 / …) | no |

The default is **JUL**, not SLF4J.

### 2. Level and destination

- **Under SLF4J** — the driver does **not** set the level or destination. Your
  SLF4J backend (logback `logback.xml`, log4j2, …) controls everything, as for any
  library. If you see no driver logs, the level for the `net.snowflake` logger
  hierarchy is above what you expect, or no backend is on the classpath.
- **Under JUL (default)** — set the `TRACING` connection property to a
  `java.util.logging` level name (`ALL`, `FINE`, `INFO`, `SEVERE`, …). That
  activates JUL file logging to `~/snowflake_jdbc%u.log`
  (`Jdk14LoggerBootstrap`). It is **skipped** if you supply your own
  `-Djava.util.logging.config.file=…` (your external JUL config wins) or if you
  switched `loggerImpl` to SLF4J.

> **There is no `sf.logLevel` property.** If you're carrying that over from older
> notes, drop it — it does nothing here. Use `TRACING` (JUL) or your SLF4J backend.

### Where the core (Rust) logs go

You do **not** get a separate `sf_core` log file with JDBC. Java-origin logs are
emitted **through** the core tracing pipeline and then handed back to your Java
logger, and core-origin events are delivered onto that same Java logger
(`CoreLogger` + `jdbc_bridge` `SFLoggerLayer`; see
[docs/logging/logging-architecture.md](../../logging/logging-architecture.md)).
So the core's `log_path` file layer does **not** apply to JDBC — configure JUL or
SLF4J and everything lands there. See
[logging-diagnostics.md](../core/logging-diagnostics.md#log_path-gates-the-core-file-layer-toml--odbc).

When **troubleshooting mode** is active, the driver opens the level gate and
force-delivers all wrapper logs to the diagnostic layer regardless of the
configured level — see
[Runbook §1](../../troubleshooting-runbook.md#1-troubleshooting-logs) /
[§2](../../troubleshooting-runbook.md#2-connection-diagnosis).

---

## Symptom: `UnsatisfiedLinkError` / native library fails to load

The JNI layer could not load the bundled native `sf_core` library. This is the
JDBC analog of Python's `_core` import failure.

1. **Platform/arch mismatch** — use the jar built for your OS + CPU (e.g. Linux
   `aarch64` vs `x86_64`, macOS `arm64`). A jar for the wrong platform has no
   loadable native library.
2. **Repackaged / shaded fat-jar** — a shading step that strips or relocates the
   bundled native resource breaks the loader. Verify the native library resource
   is still present in the final artifact.
3. Capture the full stack trace — the `UnsatisfiedLinkError` message names the
   library it tried to load.

---

## Symptom: no driver logs appear

Walk the two selectors above, in order:

1. Which `loggerImpl`? Default is **JUL** — if you configured a `logback.xml` but
   never set `-Dnet.snowflake.jdbc.loggerImpl=net.snowflake.client.log.SLF4JLogger`,
   the driver is still on JUL and ignoring logback.
2. **JUL path:** is the `TRACING` connection property set? Is a competing
   `-Djava.util.logging.config.file` present (which suppresses the built-in JUL
   file setup)?
3. **SLF4J path:** is a backend on the classpath, and is the `net.snowflake`
   logger level low enough?

---

## Symptom: TLS/certificate errors, and the JVM truststore has no effect

Setting `-Djavax.net.ssl.trustStore` (or editing the JDK `cacerts`) does **not**
change the driver's connection trust decisions. The core transport is rustls-based
and does not consult JSSE — exactly as `REQUESTS_CA_BUNDLE` is ignored on the
Python side.

**Resolution:** supply the trust anchor to the core as the `custom_root_store_path`
connection property. It **replaces** the system root store (it is not additive, and
there is no `use_system_roots` toggle) — put both public and private CAs into the
one PEM bundle if you need both. See
[cert-chain.md §B2](../core/crl-tls/cert-chain.md#b2-custom-trust-store-empty-or-malformed)
and, to reproduce outside the JVM entirely,
[tls-client-tool.md](../core/tls-client-tool.md).

---

## Result delivery

The native bridge owns the Arrow C stream and keeps each current `RecordBatch`
alive. JDBC wraps those columnar buffers with direct `ByteBuffer`s (`NewDirectByteBuffer`)
and the existing converters read them in place. Result reads have no Arrow Java
allocator and no `--add-opens=java.base/java.nio` requirement.

Cell values stay in native Arrow buffers until a converter materializes a Java
object (for example a `String` or `byte[]` from a variable-width column). Direct
buffers from a batch are invalid after the next batch load or stream close.
Bound in-flight native memory with `CLIENT_MEMORY_LIMIT` /
`CLIENT_PREFETCH_THREADS` and consume the result incrementally — see
[query-execution.md → OOM on a large result set](../core/query-execution.md#symptom-oom-on-a-large-result-set).

---

## Symptom: `InaccessibleObjectException` / "module java.base does not open java.nio"

The universal driver does not need `--add-opens=java.base/java.nio=ALL-UNNAMED`
on its own: Arrow Java is no longer on the classpath. If the exception still
appears, something else in the application is reaching into `java.nio.Buffer`
reflectively.

1. **Legacy snowflake-jdbc on the classpath.** The pinned `snowflake-jdbc`
   artifact (pre-removal) bundles Arrow Memory and still asks for the open on
   Java 17+. The project's `referenceTest` and `parityTest` Gradle tasks already
   set the flag; downstream applications that keep the legacy driver side-by-side
   need the same JVM arg.
2. **A different library.** Netty direct-buffer helpers and some JSON parsers
   also hit `java.nio.Buffer`. The driver itself does not; narrow the flag to
   the offender's modules.

---

## Auth, session expiry, timeouts

Not JDBC-specific — the behavior is in the core; JDBC forwards connection
properties/URL parameters through to sf_core:

- Key-pair (JWT), PAT, OAuth, MFA, external-browser/Okta →
  [authentication.md](../core/authentication.md).
- A persistent `390114` after a working connect means the master token expired —
  reconnect; and `CLIENT_SESSION_KEEP_ALIVE` runs a heartbeat for long-idle
  connections. See
  [authentication.md § Session expiry & renewal](../core/authentication.md#session-expiry--renewal).
- `Statement.setQueryTimeout(int)` maps to the core query timeout;
  `DriverManager.setLoginTimeout` / the `loginTimeout` property maps to the login
  timeout. Retry behavior and 503s are core-wide —
  [query-execution.md § Retries & 503s](../core/query-execution.md#retries--503s).

---

## Connection properties: JDBC-specific behavior

Most connection properties forward straight through to the core (follow the
[core pages](../index.md) for their semantics). A few have JDBC-specific names or
value handling that explains a setting "not taking":

| Property | Behavior |
|---|---|
| `useProxy` | `true` or `on` activates the connection-level proxy and requires both `proxyHost` and `proxyPort`. When false or absent, the driver falls back to JVM proxy properties only when `-Dhttp.useProxy=true`. |
| `proxyHost` / `proxyPort` / `proxyUser` / `proxyPassword` | The driver's proxy, set as connection properties (or the matching `SnowflakeDataSource` setters). These map to the core's `proxy_host`, `proxy_port`, `proxy_user`, and `proxy_password` settings. Connection-level settings take precedence over JVM proxy settings. See [proxy-tls.md](../core/crl-tls/proxy-tls.md). |
| `nonProxyHosts` | Hosts that bypass the proxy. Accepts the **legacy Java forms** — pipe-delimited (`host1\|host2`) and the `*.host` subdomain glob — and normalizes them (pipe→comma, `*.foo.com`→`.foo.com`). Note the resulting leading-dot form matches the **apex** host too, a slight over-match versus Java's subdomain-only glob. |
| JVM proxy properties | With `-Dhttp.useProxy=true`, the driver reads the legacy `http.proxy*`, `https.proxy*`, `http.proxyProtocol`, and `http.nonProxyHosts` properties and combines `NO_PROXY` with the bypass list. |
| `proxyProtocol` / `disableSocksProxy` | Accepted on `SnowflakeDataSource` for source compatibility. `proxyProtocol=https` maps to core `proxy_scheme=https` (TLS to the proxy). `disableSocksProxy` does not change routing; the native transport does not use JVM SOCKS. |
| `loginTimeout` / `queryTimeoutSeconds` | Login and query time budgets (seconds) as connection properties. `Statement.setQueryTimeout(int)` sets the query budget per statement. |
| `maxHttpRetries` / `putGetMaxRetries` | Retry-count caps for query HTTP calls and for PUT/GET transfers, respectively. |
| `allowUnderscoresInHost` | Permits `_` in the account host. Relevant to the SSO underscore→hyphen rule — see [authentication.md](../core/authentication.md). |
| `min_tls_version` / `max_tls_version` | Pin the TLS window (`tls12` / `tls13`); an inverted window is rejected up front. See [tls-handshake.md](../core/crl-tls/tls-handshake.md). |

Logging (`tracing`, `net.snowflake.jdbc.loggerImpl`) and the trust store
(`custom_root_store_path`) are covered in the sections above.

---

## Connection setup

```java
String url = "jdbc:snowflake://myaccount.snowflakecomputing.com";
Properties props = new Properties();
props.put("user", "me");
props.put("password", "...");
props.put("warehouse", "wh");
props.put("db", "db");
props.put("schema", "sc");
// Turn up JUL file logging while diagnosing (default logger impl):
props.put("tracing", "ALL");
try (Connection con = DriverManager.getConnection(url, props)) {
    // ...
}
```

---

Related: [authentication.md](../core/authentication.md) ·
[cert-chain.md](../core/crl-tls/cert-chain.md) ·
[query-execution.md](../core/query-execution.md) ·
[logging-diagnostics.md](../core/logging-diagnostics.md) ·
[tls-client-tool.md](../core/tls-client-tool.md) · up to the
[deep-dive index](../index.md) · [Runbook](../../troubleshooting-runbook.md)
