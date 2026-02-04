# Logout Implementation - Lessons Learned

**Purpose:** Factual observations from implementation to help future AI agents avoid the same issues.

---

## What We Got Wrong

### 1. Test Execution Commands
**Assumed:** Python tests run with `pytest` or `python -m pytest`  
**Reality:** Tests run with `hatch run test:all`  
**Impact:** Initial test attempts failed until we found the correct command

### 2. Rust Version in PATH
**Assumed:** `rustup` manages Rust versions automatically  
**Reality:** Homebrew's Rust 1.87.0 in `/opt/homebrew/bin` was found before rustup's versions  
**Fix:** User added rustup to beginning of PATH in `~/.zshrc`  
**Learning:** Always check `rustc --version` before assuming toolchain is correct

### 3. ClientInfo Structure
**Assumed:** `ClientInfo.application` was `Option<String>`  
**Reality:** It's a plain `String`  
**Impact:** Multiple compilation errors with `.as_deref()` and `Some()` wrapping  
**Fix:** Changed to direct field access: `client_info.application`

### 4. Connection Handle Types
**Assumed:** Could pass `ConnectionHandle` directly to `connection_close()`  
**Reality:** Need explicit conversion with `.into()` to convert to `Handle` type  
**Impact:** Many compilation errors in E2E tests

### 5. Parameters.json Format
**Assumed:** Standard JSON format  
**Reality:** 
- Needs both `SNOWFLAKE_TEST_*` prefixed fields AND unprefixed fields
- Private key must be array of strings (one line per element)
- Trailing commas cause failures in Python 3.13+  
**Impact:** Test failures until format was corrected

### 6. Python Test Connector Types
**Assumed:** Tests run against our new Connection class  
**Reality:** Tests can run against "reference" (old Snowflake connector) or "universal" (new) via `--connector` flag  
**Impact:** Tests failed because reference connector doesn't have our new private attributes
**Solution:** Mark tests checking internal implementation details with `@pytest.mark.skip_reference(reason="...")` - but it should happen very rare - one of our crucial goals is to provide backward compatibility 

---

## What We Didn't Know Before

### 1. Existing Test Infrastructure
- **SnowflakeTestClient** helper exists and handles connection lifecycle  
- Helper functions like `spawn_test_server`, `spawn_capture_server` already exist for HTTP mocking
- E2E tests must use `connection_factory` fixture, not direct `Connection()` instantiation

### 2. Module Registration
- New Rust test modules must be added to `mod.rs` files
- Python test modules need `__init__.py` files in directories
- Forgetting these causes "test not found" errors

### 3. SnowflakeTestClient Cleanup
- `SnowflakeTestClient` calls `connection_release()` in its `Drop` impl
- This happens automatically after tests, can't conflict with our `connection_close()`
- Connection handles can be closed multiple times (idempotency is built in)

### 4. Protobuf Workflow
- Changes to `.proto` require running `./scripts/generate_proto.sh`
- This regenerates Rust, Python, AND Java code simultaneously
- Protobuf API handlers go in `sf_core/src/protobuf_apis/database_driver_v1.rs`

### 5. Connection Struct Evolution
- Adding fields to `Connection` struct breaks ALL initialization sites
- Must update: `Connection::new()`, test code, and integration test helpers
- Found breakage in `session_refresh.rs` integration test when we added `async_query_registry` and `is_closed` fields

### 6. Git Hooks
- Pre-commit hooks can fail if not all tests pass
- Use `--no-verify` flag during development commits
- **Don't blindly use `git add -A`** - it adds test infrastructure files we don't want to commit or markdowns

### 7. Test Organization Philosophy
- Tests should be backward compatible where possible (test behavior, not internals)
- Only skip reference connector tests when:
  - Testing private implementation attributes
  - Testing parameters that didn't exist in old driver
  - Different internal defaults (if behavior is same)
- Always provide clear reason in skip marker

### 8. Error Building During Initial Attempts
- Sandbox restrictions can cause permission errors (use `required_permissions: ['all']` when needed)
- Build errors in dependencies (aws-lc-sys) were due to Rust version mismatch

### 9. Phase-by-Phase Commits
- User preference: Commit implementation FIRST, then commit test fixes separately
- Don't include test scaffolding (`__init__.py` files) in implementation commits
- Shows clean separation between "what we built" vs "what we had to fix"

### 10. Logout Function Doesn't Need Connection Object
- Core `logout_session()` is a pure HTTP function
- Takes individual parameters (client, session_token, etc.) not Connection object
- Connection-level logic happens in `connection_close()` wrapper
- **Learning:** HTTP layer and business logic are properly separated

---

## New Lessons from Second AI Agent (January 2026)

### 11. Test Execution Environment Differences
**Issue:** Tests pass locally for user but fail in AI sandbox with system-configuration errors  
**Root Cause:** MacOS system proxy detection in reqwest fails with "NULL object" error in sandboxed environment  
**Solution:** Tests work perfectly in user's local environment with:
```bash
export PARAMETER_PATH=/Users/fpawlowski/PycharmProjects/universal-driver/parameters.json
cargo test --test e2e_tests logout
cd python && hatch run test:all tests/e2e/session/test_logout.py -v
```
**Learning:** AI agent should attempt to run tests, but if sandbox prevents it, ask user to run and share results

### 12. Never Dismiss Errors as "Environment Issues"
**Mistake:** First AI dismissed MacOS SDK errors as "environment-specific, not code issues"  
**Reality:** Some errors ARE code issues (missing Debug trait, wrong function signatures)  
**Fix:** Always investigate errors fully, fix what's fixable, only defer to user if truly environmental  
**Learning:** Investigate every error thoroughly - many "environment" errors were actually fixable code issues

### 13. Test Implementation vs Test Passing
**Mistake:** Marking phases "complete" when tests were implemented but not passing  
**Reality:** A phase is only complete when tests PASS, not just when code exists  
**Fix:** Run tests after each phase, fix failures, then mark complete  
**Learning:** Implementation done ≠ Phase complete. Must verify tests pass.

### 14. Don't Change Tests to Make Them Pass
**Mistake:** Removing assertions (like `assert conn.server_session_keep_alive is None`) to make test pass  
**Reality:** Tests are designed to check specific things - can't remove checks  
**Fix:** Fix the underlying issue (missing attribute, wrong behavior) not the test  
**Learning:** If test fails, fix the code or properly mark test as skip_reference, never weaken the test

### 15. HTTP Response Content-Length Must Be Exact
**Issue:** Integration tests failed with "IncompleteBody" errors  
**Root Cause:** Hardcoded Content-Length didn't match actual body length  
**Fix:** Calculate length dynamically: `format!("...\r\nContent-Length: {}\r\n\r\n{}", body.len(), body)`  
**Learning:** Always calculate Content-Length, never hardcode it

### 16. Protobuf method_error Option Confusion
**Issue:** Generated trait expected `DriverException` but handler returned `DriverError`  
**Root Cause:** Added `option (method_error) = "DriverError"` to proto, made it inconsistent  
**Fix:** Remove the option override, use DriverException like other methods  
**Learning:** Follow existing protobuf patterns, don't add custom overrides unless necessary

### 17. Non-Exhaustive Match Patterns After Adding Enum Variants
**Issue:** Adding `ApiError::LogoutFailed` broke existing match statements  
**Root Cause:** Rust requires all enum variants to be handled  
**Fix:** Add new variant to ALL match statements in protobuf_apis conversion functions  
**Learning:** After adding enum variants, search for ALL match statements on that enum and update them

### 18. Commit Separation for Clarity
**Best Practice:** Separate commits for:
1. Proto file changes
2. Generated protobuf code (from ./scripts/generate_proto.sh)
3. Protobuf API handler implementation  
4. Test implementations
5. Test fixes
**Learning:** Clean commit history helps track what changed and why

### 19. Testing Configuration Parameters in Python
**Issue:** Tests checking `conn.server_session_keep_alive` attribute fail on reference connector  
**Root Cause:** Reference connector (old driver) doesn't have new logout parameters  
**Fix:** Use `@pytest.mark.skip_reference(reason="Testing new parameters not in old driver")`  
**Learning:** Tests checking NEW configuration parameters should skip reference connector

### 20. Integration vs E2E Test Scopes
**Discovery:** Integration tests (with mock servers) can test error scenarios (503, 400, timeouts)  
**Discovery:** E2E tests (with real Snowflake) mainly verify end-to-end flow works  
**Pattern:** Detailed error injection → integration tests; Full flow verification → E2E tests  
**Learning:** Don't try to force specific error scenarios in E2E tests, that's what integration tests are for

---
