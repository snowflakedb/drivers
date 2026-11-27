---
category: bugfix
---

- Fixed OAuth token caching when no user is configured: with `client_store_temporary_credential` on and no `user`, OAuth access and refresh tokens were never cached (`Invalid token key format ... username=<empty>`), so every connection opened the browser again. OAuth cache keys now accept an empty username; MFA and ID tokens still require one.
