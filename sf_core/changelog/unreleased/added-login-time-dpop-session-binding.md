---
category: internal
---

- Added login-time DPoP session binding for key-pair and workload identity logins, behind a `LoginParameters` field that no connection option sets yet: a fresh key per login, a proof on the login request, `cnf.jkt` in the key-pair JWT, and a check that the server bound the session. (snowflakedb/drivers#3233)
