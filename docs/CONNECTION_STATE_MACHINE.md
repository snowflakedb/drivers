# Connection State Machine Design

## Overview

This document proposes an explicit, type-safe connection state machine for the universal-driver Rust core. The design addresses pain points observed in the Node.js Snowflake connector's state machine implementation while providing a clean, observable, and maintainable foundation for all driver wrappers.

## Motivation: Problems with Node.js Implementation

The current Node.js driver (`snowflake-connector-nodejs/lib/services/sf.js`) uses a state machine pattern with significant usability issues:

| Problem | Description |
|---------|-------------|
| **Callback Mutation** | Options objects are mutated in place, callbacks wrapped inside callbacks |
| **Hidden State** | `currentState` is a closure variable, not inspectable |
| **No Observability** | No events/notifications when state changes |
| **Unbounded Queue** | Pending operations queue has no limits or timeouts |
| **`scope` Anti-Pattern** | Manual `this` binding via `scope` property |
| **1600+ Line File** | All states, operations, and helpers in one file |
| **No Types** | No TypeScript or JSDoc type annotations |

### Example of Problematic Node.js Code

```javascript
// Node.js: Callback mutation hell
StateConnected.prototype.request = function (options) {
  const scopeOrig = options.scope;
  const callbackOrig = options.callback;

  options.scope = this;                    // MUTATES input!
  options.callback = async function (err, body) {  // REPLACES callback!
    if (!err) {
      await callbackOrig.apply(scopeOrig, [err, body]);
    } else {
      options.scope = scopeOrig;           // RESTORES on error!
      options.callback = callbackOrig;
      // ... handle errors
    }
  };
};
```

## Proposed Solution: Explicit Rust State Machine

### Design Principles

1. **Explicit State Enum** - State is a real type, not hidden in closures
2. **Validated Transitions** - Invalid state changes are compile/runtime errors
3. **Observable** - Broadcast channel for state change notifications
4. **Bounded Queuing** - Pending operations have timeouts and limits
5. **Type-Safe Errors** - `snafu` errors with automatic source location
6. **Async-First** - Native `async/await`, no callback wrapping
7. **Reconnection Support** - Can reconnect from certain disconnected states

### State Diagram

```
                                    ┌──────────────────────────────┐
                                    │                              │
    ┌──────────┐     connect()     ┌▼───────────┐    success    ┌──────────┐
    │ Pristine │ ────────────────▶ │ Connecting │ ────────────▶ │Connected │
    └──────────┘                   └─────────────┘              └──────────┘
                                         │                       │  │    ▲
                                         │ failure               │  │    │
                                         ▼                       │  │    │
    ┌─────────────────────────────────────────────────────────┐  │  │    │
    │              Disconnected { reason }                     │◀─┘  │    │
    │  ┌─────────────────────────────────────────────────┐    │     │    │
    │  │ UserInitiated      → can_reconnect() = true     │    │     │    │
    │  │ MasterTokenExpired → can_reconnect() = true     │    │     │    │
    │  │ LoginFailed        → can_reconnect() = true     │    │     │    │
    │  │ InternalError      → can_reconnect() = false    │    │     │    │
    │  └─────────────────────────────────────────────────┘    │     │    │
    └─────────────────────────────────────────────────────────┘     │    │
         │                                                          │    │
         │ connect() if can_reconnect()                             │    │
         └──────────────────────────────────────────────────────────┼────┘
                                                                    │
    ┌──────────┐  session token expired  ◀──────────────────────────┘
    │ Renewing │────────────────────────────────────────────────────────▶
    └──────────┘                    success (back to Connected)
```

### Valid State Transitions

| From | To | Condition |
|------|-----|-----------|
| `Pristine` | `Connecting` | Always |
| `Connecting` | `Connected` | Login success |
| `Connecting` | `Disconnected` | Login failure |
| `Connected` | `Renewing` | Session token expired |
| `Connected` | `Disconnected` | User close or error |
| `Renewing` | `Connected` | Token refresh success |
| `Renewing` | `Disconnected` | Master token expired or error |
| `Disconnected` | `Connecting` | **Only if `reason.can_reconnect()`** |

### Key Components

#### 1. ConnectionState Enum

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

#### 2. DisconnectReason with Reconnection Logic

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DisconnectReason {
    UserInitiated,
    MasterTokenExpired,
    LoginFailed { code: i32, message: String },
    InternalError { message: String },
}

impl DisconnectReason {
    pub fn can_reconnect(&self) -> bool {
        !matches!(self, DisconnectReason::InternalError { .. })
    }
}
```

#### 3. ConnectionStateMachine

- Holds current state in `Arc<RwLock<ConnectionState>>`
- Validates transitions before applying
- Broadcasts state changes via `tokio::sync::broadcast`
- Manages pending operations queue with timeouts

#### 4. PendingOperation Queue

When in `Connecting` or `Renewing` states, requests are queued:

```rust
struct PendingOperation {
    ready_tx: oneshot::Sender<Result<(), ApiError>>,
    queued_at: Instant,
    timeout: Duration,
    description: String,
}
```

- When state becomes `Connected`, all pending ops are signaled
- When state becomes `Disconnected`, all pending ops receive errors
- Operations time out if state doesn't change within their deadline

## Integration with Existing Code

### Current `with_valid_session` Function

The existing `connection.rs` already has a `with_valid_session` helper that handles token refresh. The state machine enhances this by:

1. Adding explicit state tracking
2. Providing state change observability
3. Handling queuing during transient states

### Protobuf API Extension (Optional)

```protobuf
message ConnectionGetStateRequest {
  ConnectionHandle conn_handle = 1;
}

message ConnectionGetStateResponse {
  ConnectionState state = 1;
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

## Comparison: Node.js vs. Rust

| Aspect | Node.js | Rust (Proposed) |
|--------|---------|-----------------|
| State representation | Closure variable | Explicit enum |
| Transitions | Hidden function calls | Validated Result<> |
| Invalid transitions | Silent/undefined | Compile/runtime error |
| Observability | None | `broadcast::Receiver` |
| Request queuing | Unbounded array | Bounded with timeout |
| Concurrency | Callback hell | `async/await` |
| Error handling | Inconsistent | Typed `snafu` |
| Code organization | 1600-line file | Separate modules |

## Files in This POC

- `sf_core/src/connection_state/mod.rs` - Main state machine module
- `sf_core/src/connection_state/state.rs` - State enum and transitions
- `sf_core/src/connection_state/machine.rs` - State machine implementation
- `sf_core/src/connection_state/pending_ops.rs` - Pending operations queue
- `sf_core/src/connection_state/error.rs` - State-specific errors

## Usage Example

```rust
use sf_core::connection_state::{ConnectionStateMachine, ConnectionState};

// Create state machine
let sm = ConnectionStateMachine::new();

// Subscribe to state changes
let mut rx = sm.subscribe();
tokio::spawn(async move {
    while let Ok(state) = rx.recv().await {
        println!("State changed to: {:?}", state);
    }
});

// Transition to Connecting
sm.transition(ConnectionState::Connecting).await?;

// Wait for connection to be ready (blocks if Connecting/Renewing)
sm.wait_ready(Duration::from_secs(30)).await?;

// Check current state
let state = sm.state().await;
assert!(state.is_ready());
```

## Next Steps

1. Review POC implementation
2. Integrate with existing `Connection` struct
3. Update `with_valid_session` to use state machine
4. Add protobuf API for state queries
5. Write integration tests
6. Document migration path for existing code

## References

- Node.js driver: `drivers/snowflake-connector-nodejs/lib/services/sf.js`
- Current Rust connection: `sf_core/src/apis/database_driver_v1/connection.rs`
- Python driver reconnection: `drivers/snowflake-connector-python/src/snowflake/connector/connection.py`
