# Connection State Machine Design

## 1. Motivation & Goal

### The Fundamental Observation

Analysis of all Snowflake drivers reveals a consistent pattern: **every driver is inherently stateful**. A connection exists in one of several distinct states, and the behavior of operations fundamentally changes depending on the current state.

| Driver | States Observed |
|--------|-----------------|
| Node.js | Pristine → Connecting → Connected → Renewing → Disconnected |
| Python | Uninitialized → Connected → Closed (implicit via `_rest` object) |
| JDBC | Similar lifecycle managed by connection pool |
| Go | Connection state via `cn.rest` validity |
| ODBC/C | Handle-based state (allocated → connected → closed) |

Despite this universal pattern, **state management is implicit and scattered** across all implementations. Each driver handles state transitions ad-hoc, leading to subtle bugs, race conditions, and undefined behavior at state boundaries.

### Goal

Design an **explicit, type-safe connection state machine** for the universal-driver Rust core that:

1. Makes connection state **visible and inspectable** at all times
2. **Validates transitions** before they occur, rejecting invalid operations
3. Provides **observability** through state change notifications
4. Handles **concurrent access** safely during transient states
5. Serves as a **single source of truth** for all language wrappers

## 2. Problems This Design Addresses

### 2.1 Session Renewal Storm

**Problem:** When multiple concurrent queries receive `SessionExpired` errors simultaneously, each may independently attempt to renew the session token.

```
Query A: SessionExpired → tries to renew
Query B: SessionExpired → tries to renew  
Query C: SessionExpired → tries to renew
                   ↓
      3 parallel renewal requests to Snowflake
      (only 1 is needed, 2 are wasted + potential conflicts)
```

**Solution:** The state machine transitions to `Renewing` atomically. Subsequent queries detecting expiration see the state is already `Renewing` and wait on the pending operations queue instead of initiating their own renewal.

### 2.2 Token Access Race Condition

**Problem:** A critical security issue in multi-tenant pooled environments:

```
Time T0: User closes connection → close() starts deleting tokens
Time T1: Renewal goroutine/task completes → assigns NEW tokens to connection
Time T2: close() completes → connection returned to pool WITH new tokens
Time T3: New tenant gets connection → authenticates as PREVIOUS user!
```

**Solution:** The state machine enforces that token operations are only valid in appropriate states:
- `Renewing` → Can update tokens
- `Disconnected` → Token updates rejected
- State transition to `Disconnected` invalidates any in-flight renewal results

### 2.3 Operations During Transient States

**Problem:** What happens when a query is submitted while the connection is:
- Still connecting?
- In the middle of token renewal?
- Being closed?

Current drivers handle this inconsistently—some queue silently, some fail with cryptic errors, some exhibit undefined behavior.

**Solution:** Explicit queuing with visibility:
- `Connecting` / `Renewing` → Operations queued with timeout
- `Disconnected` → Operations rejected with clear reason
- `Pristine` → Operations rejected ("connection not initialized")

### 2.4 Query Execution in Closing Session

**Problem:** A query starts executing, then `close()` is called. The query may:
- Continue running server-side with no way to cancel
- Fail with confusing "connection closed" error mid-execution
- Leave resources dangling

**Solution:** State transition to `Disconnected` propagates to all waiters:
- Pending operations receive `ConnectionDisconnected` error with reason
- In-flight operations can check state and abort cleanly
- Clear distinction between "user initiated close" vs "error disconnection"

## 3. Previous Attempt Analysis: Node.js Driver

The Node.js Snowflake connector (`snowflake-connector-nodejs/lib/services/sf.js`) implemented an explicit state machine pattern. While architecturally sound in concept, the implementation suffered from several issues that made it difficult to work with.

### 3.1 What Was Attempted

The Node.js driver defined explicit states and a `transitionTo()` function:

```javascript
var Pristine = 0, Connecting = 1, Connected = 2, Renewing = 3, Disconnected = 4;

function transitionTo(connection, nextState, args) {
  var connectionConfig = connection.getConfig();
  currentState = states[nextState];
  currentState.enter.apply(currentState, args);
}
```

### 3.2 Why It Didn't Work

| Issue | Description | Impact |
|-------|-------------|--------|
| **Hidden State** | `currentState` stored in closure, not inspectable | Impossible to debug or observe |
| **No Transition Validation** | Any state could theoretically transition to any other | Silent corruption |
| **Callback Mutation** | Options objects mutated in place during operations | Unexpected side effects |
| **Unbounded Queue** | Pending operations array with no limits or timeouts | Memory leaks, hangs |
| **`scope` Anti-Pattern** | Manual `this` binding via `scope` property | Confusing, error-prone |
| **1600+ Line Monolith** | All states, operations, queue logic in one file | Unmaintainable |
| **No Types** | Pure JavaScript with no type annotations | Runtime errors only |
| **Callback Hell** | Deep nesting of wrapped callbacks | Unreadable flow |

### 3.3 Example of Problematic Code

```javascript
// The infamous callback mutation pattern
StateConnected.prototype.request = function (options) {
  const scopeOrig = options.scope;
  const callbackOrig = options.callback;

  options.scope = this;                    // MUTATES input!
  options.callback = async function (err, body) {  // REPLACES callback!
    if (!err) {
      await callbackOrig.apply(scopeOrig, [err, body]);
    } else {
      options.scope = scopeOrig;           // RESTORES on error path!
      options.callback = callbackOrig;
      // ... complex error handling
    }
  };
};
```

### 3.4 Lessons Learned

1. **Explicit state is necessary but not sufficient** — the implementation must also be clean
2. **Transition validation must be enforced**, not advisory
3. **Queuing needs bounds and timeouts** to prevent resource exhaustion
4. **State must be observable** for debugging and monitoring
5. **Type safety catches errors early** — before production

## 4. Proposed Solution: Rust State Machine

### 4.1 Design Principles

| Principle | Implementation |
|-----------|----------------|
| Explicit State | `ConnectionState` enum with variants |
| Validated Transitions | `Result<(), StateMachineError>` on every transition |
| Observable | `tokio::sync::broadcast` for state change events |
| Bounded Queuing | Max queue size + operation timeouts |
| Type-Safe Errors | `snafu` errors with automatic source location |
| Async-First | Native `async/await`, no callback wrapping |
| Reconnection Support | Conditional reconnect based on disconnect reason |

### 4.2 State Diagram

```
                                    ┌──────────────────────────────────┐
                                    │                                  │
    ┌──────────┐     connect()     ┌▼───────────┐    success     ┌──────────┐
    │ Pristine │ ────────────────▶ │ Connecting │ ─────────────▶ │Connected │ ◀──────────┐  
    └──────────┘                   └────────────┘                └──────────┘            │
                                         │                        │  │    ▲              │
                                         │ failure                │  │    │              │
                                         ▼                        │  │    │ token        │
    ┌─────────────────────────────────────────────────────────┐   │  │    │ refresh      │
    │              Disconnected { reason }                    │◀──┘  │    │ success      │
    │  ┌─────────────────────────────────────────────────┐    │      │    │              │
    │  │ UserInitiated      → can_reconnect() = true     │    │      │    │              │
    │  │ MasterTokenExpired → can_reconnect() = true     │    │      │    │              │
    │  │ LoginFailed        → can_reconnect() = true     │    │      │    │              │
    │  │ InternalError      → can_reconnect() = false    │    │      │    │              │
    │  └─────────────────────────────────────────────────┘    │      │    │              │
    └─────────────────────────────────────────────────────────┘      │    │              │
         │                                                           │    │              │
         │ connect() [if can_reconnect()]                            │    │              │
         └───────────────────────────────────────────────────────────┼────┘              │
                                                                     │                   │
    ┌──────────┐  session token near expiry  ◀───────────────────────┘                   │
    │ Renewing │ ────────────────────────────────────────────────────────────────────────┘
    └──────────┘                     success (back to Connected)
          │
          │ master token expired / error
          ▼
    Disconnected { MasterTokenExpired | InternalError }
```

### 4.3 Valid State Transitions

| From | To | Trigger | Pending Ops Behavior |
|------|-----|---------|---------------------|
| `Pristine` | `Connecting` | `connect()` called | Queue starts accepting |
| `Connecting` | `Connected` | Login success | Queue drained (ready) |
| `Connecting` | `Disconnected` | Login failure | Queue drained (error) |
| `Connected` | `Renewing` | Token near expiry | Queue starts accepting |
| `Connected` | `Disconnected` | User close or error | Queue drained (error) |
| `Renewing` | `Connected` | Token refresh success | Queue drained (ready) |
| `Renewing` | `Disconnected` | Master expired / error | Queue drained (error) |
| `Disconnected` | `Connecting` | `connect()` if `can_reconnect()` | Queue starts accepting |

### 4.4 Key Components

#### ConnectionState Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Pristine,
    Connecting,
    Connected,
    Renewing,
    Disconnected { reason: DisconnectReason },
}
```

#### DisconnectReason with Reconnection Logic

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisconnectReason {
    UserInitiated,
    MasterTokenExpired,
    LoginFailed { code: i32, message: String },
    InternalError { message: String },
}

impl DisconnectReason {
    /// Only InternalError prevents reconnection
    pub fn can_reconnect(&self) -> bool {
        !matches!(self, DisconnectReason::InternalError { .. })
    }
}
```

#### ConnectionStateMachine

```rust
pub struct ConnectionStateMachine {
    /// Current state behind async RwLock
    state: Arc<RwLock<ConnectionState>>,
    /// Broadcast channel for state change notifications  
    state_tx: broadcast::Sender<ConnectionState>,
    /// Pending operations queue (bounded)
    pending_ops: Arc<RwLock<PendingOperationsQueue>>,
}
```

#### PendingOperation Queue

```rust
struct PendingOperation {
    ready_tx: oneshot::Sender<Result<()>>,
    queued_at: Instant,
    description: String,
}

struct PendingOperationsQueue {
    queue: Vec<PendingOperation>,
    // MAX_PENDING_OPS = 1000 (configurable)
}
```

## 5. How This Addresses Each Problem

### Session Renewal Storm

```rust
// In with_valid_session():
match self.state_machine.state().await {
    ConnectionState::Connected => {
        // Check if token needs refresh
        if session_tokens.needs_refresh() {
            // Atomic transition - only first caller wins
            self.state_machine.start_renewing().await?;
            // Others will see Renewing and wait
        }
    }
    ConnectionState::Renewing => {
        // Already renewing - just wait
        self.state_machine.wait_ready(timeout).await?;
    }
    // ...
}
```

### Token Access Race

```rust
// Renewal completion checks state before applying tokens
pub async fn complete_renewal(&self, new_tokens: SessionTokens) -> Result<()> {
    let state = self.state_machine.state().await;
    match state {
        ConnectionState::Renewing => {
            // Safe to apply tokens
            self.session_tokens.write().await.replace(new_tokens);
            self.state_machine.renewal_complete().await?;
        }
        ConnectionState::Disconnected { .. } => {
            // Connection was closed during renewal - discard tokens!
            tracing::warn!("Discarding renewed tokens - connection already closed");
            return Err(StateMachineError::ConnectionDisconnected { ... });
        }
        _ => {
            return Err(StateMachineError::InvalidStateForRenewal { ... });
        }
    }
}
```

### Query Execution in Closing Session

```rust
// Before executing query:
self.state_machine.wait_ready(timeout).await?;

// If close() is called during execution:
// - State transitions to Disconnected { UserInitiated }
// - All pending operations receive ConnectionDisconnected error
// - Error includes reason, enabling clean abort
```

## 6. Comparison: Implicit vs. Explicit State Machine

| Aspect | Implicit (Current Drivers) | Explicit (Proposed) |
|--------|---------------------------|---------------------|
| State visibility | Hidden in variables/flags | Inspectable enum |
| Invalid transitions | Silent/undefined behavior | Compile/runtime error |
| Concurrent access | Race conditions | `Arc<RwLock>` protected |
| Renewal coordination | Multiple storms possible | Single renewal, others wait |
| Token lifecycle | Race-prone | State-gated updates |
| Observability | None | `broadcast::Receiver` |
| Debugging | "Why did this fail?" | Clear state + transition log |

## 7. Integration with Protobuf API

```protobuf
// Expose state to language wrappers
message ConnectionGetStateRequest {
  ConnectionHandle conn_handle = 1;
}

message ConnectionGetStateResponse {
  ConnectionState state = 1;
  optional string disconnect_reason = 2;  // If disconnected
}

enum ConnectionState {
  CONNECTION_STATE_UNSPECIFIED = 0;
  CONNECTION_STATE_PRISTINE = 1;
  CONNECTION_STATE_CONNECTING = 2;
  CONNECTION_STATE_CONNECTED = 3;
  CONNECTION_STATE_RENEWING = 4;
  CONNECTION_STATE_DISCONNECTED = 5;
}
```

## 8. Files in This POC

| File | Purpose |
|------|---------|
| `sf_core/src/connection_state/mod.rs` | Module root and re-exports |
| `sf_core/src/connection_state/state.rs` | `ConnectionState` and `DisconnectReason` enums |
| `sf_core/src/connection_state/machine.rs` | `ConnectionStateMachine` implementation |
| `sf_core/src/connection_state/pending_ops.rs` | Bounded pending operations queue |
| `sf_core/src/connection_state/error.rs` | `StateMachineError` with `snafu` |

## 9. Usage Example

```rust
use sf_core::connection_state::{ConnectionStateMachine, ConnectionState};
use std::time::Duration;

// Create state machine
let sm = ConnectionStateMachine::new();

// Subscribe to state changes (for monitoring/debugging)
let mut rx = sm.subscribe();
tokio::spawn(async move {
    while let Ok(state) = rx.recv().await {
        tracing::info!("Connection state: {:?}", state);
    }
});

// Start connecting
sm.start_connecting().await?;

// ... perform login ...

// Signal connection established
sm.connection_established().await?;

// Operations wait if not ready
sm.wait_ready(Duration::from_secs(30)).await?;

// Check state programmatically
assert!(sm.state().await.is_ready());

// Close connection
sm.close().await?;

// Can reconnect if reason permits
sm.start_connecting().await?;  // OK - UserInitiated allows reconnect
```

## 10. Next Steps

1. **Review** - Gather feedback on this design
2. **Integrate** - Wire `ConnectionStateMachine` into existing `Connection` struct
3. **Update `with_valid_session`** - Use state machine for renewal coordination
4. **Protobuf API** - Expose state queries to language wrappers
5. **Integration Tests** - Test concurrent scenarios (renewal storm, close during query)
6. **Documentation** - Migration guide for existing code

## References

- Node.js driver analysis: `drivers/snowflake-connector-nodejs/lib/services/sf.js`
- Python driver reconnection: `drivers/snowflake-connector-python/src/snowflake/connector/connection.py`
- Current Rust connection: `sf_core/src/apis/database_driver_v1/connection.rs`
