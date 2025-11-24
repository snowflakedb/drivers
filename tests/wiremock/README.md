# Wiremock Integration Testing

This directory contains shared Wiremock standalone jar and mappings for integration tests for core and all wrappers.

## Overview

Wiremock is used to mock Snowflake server responses for integration tests, allowing to:
- Test client behavior without connecting to real Snowflake instances
- Test error handling and edge cases

## Shared Infrastructure

The Wiremock setup is **shared across all drivers**:

```
tests/wiremock/
├── wiremock_standalone/
│   └── wiremock-standalone-3.13.2.jar    # Wiremock server JAR
├── mappings/                              # Request/response mappings (shared)
│   ├── auth/
│   │   └── login_success_jwt.json
│   └── put_get/
│       └── put_unsupported_compression_type.json
└── __files/                               # Runtime files (gitignored)
```

## Driver-Specific Clients

Each driver must implement a **WiremockClient** to manage the Wiremock jar process and communicate with it via HTTP.

### Required Methods

#### 1. `start()` - Start Wiremock Server

**Responsibilities:**
- Find a free port/ports
- Start the Wiremock standalone JAR process with required arguments
- Wait for Wiremock to become healthy
- Return client instance with dynamic port

#### 2. `http_url()` - Get Server URL

Returns the full HTTP URL with dynamically assigned port.

#### 3. `add_mapping(path, placeholders?)` - Add Request/Response Mapping

**Responsibilities:**
- Load mapping file from `tests/wiremock/mappings/{path}`
- Replace placeholders (see below)
- POST mapping to Wiremock admin API: `{url}/__admin/mappings`
- Handle both single mappings and mapping arrays

#### 4. Automatic Cleanup
Each client should automatically stop Wiremock when dropped/destroyed

## Placeholder Support

Mappings should support dynamic placeholder replacement, which will allow to inject required details into mappings files. Usage: 

- replace environment specific details, like repository root
- reuse similar mappings by passing only changed details

### Built-in Placeholders

#### `REPO_ROOT` - Repository Root Path

**Example Mapping File:**
```json
{
  "request": {
    "method": "POST",
    "url": "/queries/v1/query-request"
  },
  "response": {
    "status": 200,
    "jsonBody": {
      "data": {
        "stageInfo": {
          "location": "file://{{REPO_ROOT}}/tests/test_data/generated_test_data/compression/",
          "locationType": "LOCAL_FS"
        }
      }
    }
  }
}
```

### Custom Placeholders

You can pass custom placeholders when adding mappings:

**Rust:**
```rust
let mut placeholders = std::collections::HashMap::new();
placeholders.insert("{{STAGE_NAME}}".to_string(), "MY_STAGE".to_string());
wiremock.add_mapping("put_get/custom_mapping.json", Some(&placeholders));
```

**Python:**
```python
placeholders = {
    "{{STAGE_NAME}}": "MY_STAGE",
    "{{FILE_NAME}}": "test_file.csv"
}
wiremock.add_mapping("put_get/custom_mapping.json", placeholders)
```

**Note:** `{{REPO_ROOT}}` cannot be overridden - it's always set to the repository root.
