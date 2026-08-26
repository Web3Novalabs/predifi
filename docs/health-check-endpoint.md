# Health Check Endpoint

## Overview

This document describes the health check endpoint implemented in
`predifi-backend`. It provides a simple way for load balancers,
orchestrators, and developers to verify the service is running.

---

## Endpoint
```
GET /health
```

### Response
```json
{
  "status": "ok",
  "service": "predifi-backend",
  "version": "0.1.0"
}
```

- `status` — always `"ok"` when the server is running
- `service` — name of the service
- `version` — current version from Cargo.toml

---

## Implementation

**File:** `src/main.rs`

### `health()` handler
An async Axum handler that returns HTTP 200 with basic system info.
The version is read at compile time from `Cargo.toml` using
`env!("CARGO_PKG_VERSION")` — no runtime overhead.

---

## Test Coverage

**File:** `src/tests.rs`

| Test | What it verifies |
|------|-----------------|
| `health_returns_200_with_ok_body` | Returns HTTP 200 with status field |
| `health_returns_system_info` | Returns service name and version |
| `middleware_does_not_alter_200_status` | Middleware doesn't change 200 response |

---

## Example Usage
```bash
curl http://localhost:3000/health
```

Expected response:
```json
{"status":"ok","service":"predifi-backend","version":"0.1.0"}
```