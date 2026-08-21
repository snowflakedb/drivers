# PrivateLink

How AWS PrivateLink / Azure Private Link / GCP Private Service Connect change the
driver's connectivity, the two routing scopes, and the DNS / CRL / certificate
gotchas specific to PrivateLink. Part of the
[troubleshooting deep-dive](../index.md). For the live connectivity probe, use
the Runbook's [connection diagnosis](../../troubleshooting-runbook.md#2-connection-diagnosis).

The driver itself does **not** implement PrivateLink — it simply connects to
whatever host it is given. Everything below is about pointing the driver at the
right hostnames and making sure DNS and the firewall cooperate.

---

## The account host

For a PrivateLink deployment the `account` identifier must carry the region and a
`.privatelink` segment, e.g. `myaccount.us-east-1.privatelink`. The login
endpoint is built as `account` + `.snowflakecomputing.com`, yielding a host like:

```
myaccount.us-east-1.privatelink.snowflakecomputing.com
```

An explicit `host` (and `port`) connection parameter overrides that construction
— verify it points at the PrivateLink host, not the public one.

---

## Two routing scopes

PrivateLink can cover either just the control plane or the control plane **plus**
internal stages. Knowing which one your account uses tells you what has to be
reachable.

| Scope | What routes through PrivateLink | What stays public |
|---|---|---|
| **Control plane only** (default) | The Snowflake API endpoint (`*.snowflakecomputing.com`: login, queries, token refresh) | Stage / cloud-storage traffic (PUT/GET, result chunks) still goes to public S3 / Azure Blob / GCS endpoints |
| **`ENABLE_INTERNAL_STAGES_PRIVATELINK`** (account-level) | Control plane **and** internal-stage traffic (PUT/GET, result-chunk downloads) | — |

`ENABLE_INTERNAL_STAGES_PRIVATELINK` is a **server-side account setting**, not a
driver parameter. When it is on, the hostnames returned by
`SYSTEM$ALLOWLIST_PRIVATELINK()` include the internal-stage endpoints, and stage
traffic must reach those PrivateLink hostnames rather than the public
cloud-storage ones. See [stage-cloud-storage.md](stage-cloud-storage.md).

### Getting the hostnames to allowlist

```sql
-- On a PrivateLink account, use this instead of SYSTEM$ALLOWLIST():
SELECT * FROM TABLE(SYSTEM$ALLOWLIST_PRIVATELINK());
```

Open every hostname it returns. This is the authoritative list — do not hardcode
IP ranges.

---

## DNS is the most common failure

PrivateLink hostnames must resolve to **private** IPs (the VPC/VNet endpoint). If
they resolve to public IPs, DNS is misconfigured — traffic will either leave your
private network or fail outright.

The driver's [connection diagnosis](../../troubleshooting-runbook.md#2-connection-diagnosis)
checks this explicitly: it flags a `.privatelink.` host that resolves to a public
IP with **"PrivateLink host resolved to public IP … — check DNS configuration"**
(`sf_core/src/diagnostic/mod.rs`). If you see that line, fix the private DNS zone
(Route 53 private hosted zone, Azure Private DNS, or GCP Cloud DNS) before looking
anywhere else.

Manual check:

```sh
# Should return a PRIVATE (RFC 1918) address on a correctly configured client:
nslookup myaccount.us-east-1.privatelink.snowflakecomputing.com
```

---

## CRL fetches still need public port 80

This surprises people: **CRL revocation checks bypass PrivateLink.** CRL
distribution point (CDP) URLs are embedded in the public CA certificates and
point to **public CA infrastructure over HTTP port 80** — regardless of
PrivateLink. So when `crl_check_mode` is `ENABLED` or `ADVISORY`, outbound
**port 80 to the public CA CDP hosts must be open** even on an otherwise fully
private network. The default `crl_check_mode=DISABLED` needs no port 80.

See [crl-revocation.md](crl-tls/crl-revocation.md#crl-3-fetch-errors--network--timeout)
for the CDP firewall test.

---

## Certificates

PrivateLink endpoints present certificates that chain to the **public web PKI** —
the same system root store that validates the public endpoint also validates the
PrivateLink host. You should **not** need `custom_root_store_path` for PrivateLink
itself.

If chain validation fails on a PrivateLink host, suspect the usual culprits, not
PrivateLink:

- a TLS-inspecting corporate proxy re-signing with a private CA
  ([proxy-tls.md](crl-tls/proxy-tls.md#c4-tls-interception-by-a-corporate-mitm-proxy)),
- missing intermediates or a broken custom trust store
  ([cert-chain.md](crl-tls/cert-chain.md)).

---

## Per-wrapper notes

The account/host parameters are set the same way everywhere; only the parameter
plumbing differs:

| Wrapper | Where to set the PrivateLink host |
|---|---|
| Python | `account="myaccount.us-east-1.privatelink"` (or explicit `host=`) |
| JDBC | JDBC URL host, or `account` property |
| Node.js | `account` / `host` in the connection options |
| ODBC | `server` / `account` INI keys (see [odbc.md](../wrappers/odbc.md)) |

---

Related: [stage-cloud-storage.md](stage-cloud-storage.md) ·
[crl-revocation.md](crl-tls/crl-revocation.md) ·
[proxy-tls.md](crl-tls/proxy-tls.md) ·
[authentication.md](authentication.md) · up to the
[deep-dive index](../index.md) · [Runbook](../../troubleshooting-runbook.md)
