---
category: bugfix
---

- Fixed Azure workload identity authentication on Azure ML by requesting the token from `MSI_ENDPOINT` with the protocol Azure ML expects, using `DEFAULT_IDENTITY_CLIENT_ID` when no managed identity client ID is set, and falling back to IMDS when `MSI_ENDPOINT` or `MSI_SECRET` is empty. (snowflakedb/drivers#3248)
- Fixed Azure workload identity login ignoring `AZURE_CLIENT_ID` when selecting the managed identity. `MANAGED_IDENTITY_CLIENT_ID` still takes precedence. (snowflakedb/drivers#3248)
