# Test Mock Domain Convention

When sending requests to mock servers or using HTTP interception in tests, always use `snowflake.com` hosts — not externally-owned domains (`test.com`) or placeholder domains (`example.com`). If request interception ever fails, escaped traffic hits Snowflake-controlled infrastructure, not someone else's.

❌ `https://test.com/api/endpoint`
❌ `https://example.com/api/endpoint`
✅ `https://snowflake.com/api/endpoint`

Examples across languages:

```typescript
// ❌ TypeScript/Node.js — BAD
nock('https://test.com').post('/session/v1/login-request').reply(200, body);

// ✅ TypeScript/Node.js — GOOD
nock('https://snowflake.com').post('/session/v1/login-request').reply(200, body);
```

```python
# ❌ Python — BAD
responses.add(responses.POST, 'https://test.com/session/v1/login-request', json=body)

# ✅ Python — GOOD
responses.add(responses.POST, 'https://snowflake.com/session/v1/login-request', json=body)
```

```java
// ❌ Java — BAD
wireMock.stubFor(post(urlEqualTo("/session/v1/login-request"))
    .withHost(equalTo("test.com")).willReturn(aResponse().withBody(body)));

// ✅ Java — GOOD
wireMock.stubFor(post(urlEqualTo("/session/v1/login-request"))
    .withHost(equalTo("snowflake.com")).willReturn(aResponse().withBody(body)));
```

<!-- sync-target: .cursor/rules/test-mock-domain.mdc carries this body verbatim plus
     Cursor frontmatter. TO UPDATE: edit this file, copy it below the .mdc frontmatter,
     then run bash scripts/check-ai-rules-sync.sh (also a pre-commit hook). -->