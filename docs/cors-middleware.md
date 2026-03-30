# CORS Middleware

## Overview

This document describes the CORS (Cross-Origin Resource Sharing)
middleware configured in `predifi-backend`. It controls which frontend
origins are allowed to make requests to the API, preventing unauthorized
cross-origin access.

---

## What is CORS?

When a browser makes a request to a different domain than the page it
is on, it performs a CORS check. The server must explicitly allow the
origin in its response headers, otherwise the browser blocks the request.

---

## Allowed Origins

The following frontend origins are permitted by default:

- `http://localhost:3000` — local development
- `http://localhost:5173` — Vite dev server
- `https://predifi.app` — production frontend

To add more origins, update the `ALLOWED_ORIGINS` array in `src/main.rs`.

---

## Allowed Methods

| Method | Purpose |
|--------|---------|
| GET | Read data |
| POST | Create resources |
| PUT | Update resources |
| DELETE | Remove resources |
| OPTIONS | Preflight requests |

## Allowed Headers

- `Content-Type`
- `Authorization`
- `Accept`

---

## Implementation

**File:** `src/main.rs`

### `build_cors() -> CorsLayer`
Builds a `tower-http` CorsLayer configured with the allowed origins,
methods and headers. Called once during router setup.

### `build_router() -> Router`
Attaches the CORS layer to the router before the logging layer so
CORS headers are always present in responses.

---

## Security Assumptions

- Only origins explicitly listed in `ALLOWED_ORIGINS` are permitted
- All other origins are rejected by the browser automatically
- The `Authorization` header is allowed so JWT tokens can be sent
- OPTIONS preflight requests are handled automatically by the layer
- CORS is enforced by the browser — server-to-server requests bypass it

---

## Abuse and Failure Paths

| Scenario | Behaviour |
|----------|-----------|
| Request from unlisted origin | Browser blocks response |
| Preflight OPTIONS request | Returns 200 with CORS headers |
| Request with disallowed method | Browser blocks request |
| Request with disallowed header | Browser blocks request |

---

## Test Coverage

**File:** `src/tests.rs`

| Test | What it verifies |
|------|-----------------|
| `cors_allows_allowed_origin` | CORS header returned for allowed origin |
| `cors_handles_preflight_request` | OPTIONS preflight returns 200 |

---

## Example
```bash
# Request from allowed origin
curl -H "Origin: http://localhost:5173" http://localhost:3000/health
# Response includes: access-control-allow-origin: http://localhost:5173

# Preflight request
curl -X OPTIONS \
  -H "Origin: http://localhost:5173" \
  -H "Access-Control-Request-Method: GET" \
  http://localhost:3000/health
```

---

## Related Files

- `src/main.rs` — CORS configuration and router setup
- `src/tests.rs` — CORS test suite
- `Cargo.toml` — tower-http dependency with cors feature
